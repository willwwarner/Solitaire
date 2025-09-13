/* patsolve.rs
 *
 * Copyright 1998-2002 Tom Holroyd <tomh@kurage.nimh.nih.gov>
 * Copyright 2006-2009 Stephan Kulow <coolo@kde.org>
 * Copyright 2025 Will Warner
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Exit status for a solver run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    NoSolutionExists,
    SolutionExists,
    SearchAborted,
    MemoryLimitReached,
    UnableToDetermineSolvability,
}

/// Metadata about a generated move, supplied by the Game implementation.
/// - `pri`: higher means more urgent before squash; solver will squash by outs.
/// - `auto_out`: moves that are "forced"/auto can be treated specially.
#[derive(Debug, Clone, Copy)]
pub struct MoveMeta {
    pub pri: i32,
    pub auto_out: bool,
}

impl MoveMeta {
    pub fn new(pri: i32, auto_out: bool) -> Self {
        Self { pri, auto_out }
    }
}

/// A trait the concrete game must implement to plug into the solver.
///
/// The solver is stateful and calls `make_move`/`undo_move` and re-loads states
/// when dequeuing via `load_from_key`. To support fast restore, the game is
/// expected to encode all piles/freecells/foundations etc. into a compact Key.
///
/// Notes for implementors:
/// - Ensure `Key` is canonical for positions that are equivalent under
///   your game's symmetry rules (e.g., renumber piles if pile order doesn't
///   matter) so the visited set is effective.
/// - `cluster_id` can be used to split the search space by "outs" equivalent
///   classes. If unsure, return 0 and ignore clustering.
/// - `out_count` is used for priority squashing; return number of cards in
///   foundations (or equivalent progress metric).
pub trait Game {
    type Move: Clone + Debug + Send + Sync + 'static;
    type Key: Clone + Eq + Hash + Send + Sync + 'static;

    /// Current state's unique key (after any updates).
    fn current_key(&self) -> Self::Key;

    /// Load internal state from a given key. Must be deterministic and complete.
    fn load_from_key(&mut self, key: &Self::Key);

    /// A cluster id for the current state (e.g. based on foundations). Positions
    /// from different clusters are never equal; used to partition visited sets.
    fn cluster_id(&self) -> u32;

    /// Number of cards "out" (or another monotone progress metric).
    fn out_count(&self) -> usize;

    /// Generate all legal moves from the current state, filling `moves` and
    /// matching metadata in `metas`. Both vectors will be cleared before use.
    fn generate_moves(&mut self, moves: &mut Vec<Self::Move>, metas: &mut Vec<MoveMeta>);

    /// Optionally adjust priorities (e.g. based on heuristics) after generation.
    /// Default is no-op.
    fn prioritize(&self, _metas: &mut [MoveMeta]) {}

    /// Apply a move to the current state.
    fn make_move(&mut self, mv: &Self::Move);

    /// Undo a move previously applied. The solver calls this in strict LIFO order.
    fn undo_move(&mut self, mv: &Self::Move);

    /// Whether the current state is a win.
    fn is_won(&self) -> bool;
}

/// Configuration for the search. Tweak to tune performance/quality.
#[derive(Debug, Clone)]
pub struct SolverConfig {
    /// Maximum number of unique positions to visit before aborting.
    pub max_positions: Option<u64>,
    /// Number of priority queues. More queues yields finer gradation.
    pub num_queues: usize,
    /// If true, place non-auto-out moves before auto-out moves initially.
    pub defer_auto_out_queueing: bool,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_positions: None,
            num_queues: 64,
            defer_auto_out_queueing: true,
        }
    }
}

/// Internal node stored in the work graph.
#[derive(Debug, Clone)]
struct Node<M, K> {
    key: K,
    cluster: u32,
    depth: u32,
    parent: Option<usize>,
    by_move: Option<M>,
    n_children_live: u32,
}

/// A generic, prioritized breadth-first solver.
///
/// This is a near feature-parity rewrite oriented around a generic trait,
/// allowing multiple games to plug-in while reusing the search engine.
///
/// The solver maintains:
/// - multiple priority queues (round-robin sweeping)
/// - a visited set partitioned by cluster
/// - first-move list from the root
/// - winning-move reconstruction by following parent links
pub struct Solver<G: Game> {
    game: G,
    cfg: SolverConfig,
    stop: Arc<AtomicBool>,

    // Work queues (priority buckets)
    queues: Vec<VecDeque<usize>>,
    // All created nodes (index stable)
    nodes: Vec<Node<G::Move, G::Key>>,
    // visited[cluster] = set of keys
    visited: HashMap<u32, HashSet<G::Key>>,

    // Stats and status
    total_generated: u64,
    total_unique: u64,
    depth_sum: u64,
    status: ExitStatus,

    // Results/introspection
    first_moves: Vec<G::Move>,
    win_moves: Vec<G::Move>,

    // Round-robin state
    rr_qpos: isize,
    rr_minpos: isize,
    rr_maxq: isize,
}

impl<G: Game> Solver<G> {
    pub fn new(game: G) -> Self {
        Self::with_config(game, SolverConfig::default())
    }

    pub fn with_config(game: G, cfg: SolverConfig) -> Self {
        let num_queues = cfg.num_queues.max(1);
        Self {
            game,
            cfg,
            stop: Arc::new(AtomicBool::new(false)),

            queues: (0..num_queues).map(|_| VecDeque::new()).collect(),
            nodes: Vec::with_capacity(4096),
            visited: HashMap::new(),

            total_generated: 0,
            total_unique: 0,
            depth_sum: 0,
            status: ExitStatus::NoSolutionExists,

            first_moves: Vec::new(),
            win_moves: Vec::new(),

            rr_qpos: 0,
            rr_minpos: 0,
            rr_maxq: 0,
        }
    }

    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop)
    }

    pub fn stop_execution(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub fn status(&self) -> ExitStatus {
        self.status
    }

    pub fn first_moves(&self) -> &[G::Move] {
        &self.first_moves
    }

    pub fn win_moves(&self) -> &[G::Move] {
        &self.win_moves
    }

    pub fn total_generated(&self) -> u64 {
        self.total_generated
    }

    pub fn total_unique(&self) -> u64 {
        self.total_unique
    }

    /// Run the solver until completion or aborted by stop/config limits.
    pub fn run(&mut self) -> ExitStatus {
        self.reset_run_state();

        // Seed with initial state
        let root_key = self.game.current_key();
        let cluster = self.game.cluster_id();
        let depth = 0u32;
        let root_idx = self.new_node(root_key.clone(), cluster, depth, None, None);
        if let Some(maxp) = self.cfg.max_positions {
            if self.total_unique > maxp {
                self.status = ExitStatus::MemoryLimitReached;
                return self.finish();
            }
        }

        // Root goes to queue with priority 0
        self.enqueue(root_idx, 0);

        // Main loop
        while let Some(idx) = self.dequeue() {
            if self.status != ExitStatus::NoSolutionExists {
                break;
            }
            if self.stop.load(Ordering::Relaxed) {
                self.status = ExitStatus::SearchAborted;
                break;
            }
            // Restore state for this node
            let (key, cluster, depth) = {
                let n = &self.nodes[idx];
                (n.key.clone(), n.cluster, n.depth)
            };
            self.game.load_from_key(&key);

            // Win check
            if self.game.is_won() {
                self.status = ExitStatus::SolutionExists;
                self.reconstruct_and_store_win(idx);
                break;
            }

            // Generate moves
            let mut moves: Vec<G::Move> = Vec::with_capacity(64);
            let mut metas: Vec<MoveMeta> = Vec::with_capacity(64);
            self.game.generate_moves(&mut moves, &mut metas);
            self.total_generated = self.total_generated.saturating_add(moves.len() as u64);

            if moves.is_empty() {
                continue;
            }

            // Allow game to reprioritize
            self.game.prioritize(&mut metas);

            // Capture first moves from root
            if depth == 0 {
                self.first_moves.clear();
                self.first_moves.extend(moves.iter().cloned());
            }

            // Optionally defer auto-out moves to later in the same layer
            let defer_auto_out = self.cfg.defer_auto_out_queueing && metas.iter().any(|m| m.auto_out);
            let (primary_range, secondary_range) = if defer_auto_out {
                let mut primary = Vec::new();
                let mut secondary = Vec::new();
                for (i, meta) in metas.iter().enumerate() {
                    if meta.auto_out {
                        secondary.push(i);
                    } else {
                        primary.push(i);
                    }
                }
                (primary, secondary)
            } else {
                let all = (0..moves.len()).collect::<Vec<usize>>();
                (all, Vec::new())
            };


            // Explore successors
            let mut explored_any = false;

            // Helper closure for pushing a successor
            let mut push_successor = |i: usize| {
                let mv = moves[i].clone();
                let meta = metas[i];
                self.game.make_move(&mv);

                // New state's key and cluster
                let succ_key = self.game.current_key();
                let succ_cluster = self.game.cluster_id();

                // Dedup by cluster + key
                let was_new = self.try_visit(succ_cluster, &succ_key);
                if was_new {
                    let child_idx = self.new_node(
                        succ_key.clone(),
                        succ_cluster,
                        depth + 1,
                        Some(idx),
                        Some(mv.clone()),
                    );
                    // Queue squashing by outs
                    let pri = self.squash_priority(meta.pri, self.game.out_count());
                    if succ_cluster != cluster {
                        // Different cluster: explore immediately (more promising)
                        // Recursive-like: we directly expand by re-queuing at high priority 0
                        self.enqueue(child_idx, pri.min(0));
                    } else {
                        self.enqueue(child_idx, pri);
                    }
                    explored_any = true;
                }

                self.game.undo_move(&mv);
            };

            // Process primary moves
            for i in primary_range.iter().copied() {
                push_successor(i);
            }
            // Then secondary moves (e.g., auto-outs)
            for i in secondary_range.iter().copied() {
                push_successor(i);
            }

            // Decrement parent's live-children count when applicable
            if !explored_any {
                self.prune_upwards(idx);
            }
        }

        self.finish()
    }

    fn reset_run_state(&mut self) {
        self.stop.store(false, Ordering::Relaxed);
        self.queues.iter_mut().for_each(|q| q.clear());
        self.nodes.clear();
        self.visited.clear();
        self.total_generated = 0;
        self.total_unique = 0;
        self.depth_sum = 0;
        self.status = ExitStatus::NoSolutionExists;
        self.first_moves.clear();
        self.win_moves.clear();
        self.rr_qpos = 0;
        self.rr_minpos = 0;
        self.rr_maxq = 0;
    }

    fn finish(&mut self) -> ExitStatus {
        if self.status == ExitStatus::SearchAborted {
            self.first_moves.clear();
            self.win_moves.clear();
        }
        self.status
    }

    fn new_node(
        &mut self,
        key: G::Key,
        cluster: u32,
        depth: u32,
        parent: Option<usize>,
        by_move: Option<G::Move>,
    ) -> usize {
        let idx = self.nodes.len();
        let node = Node {
            key,
            cluster,
            depth,
            parent,
            by_move,
            n_children_live: 0,
        };
        self.nodes.push(node);
        self.total_unique = self.total_unique.saturating_add(1);
        self.depth_sum = self.depth_sum.saturating_add(depth as u64);
        idx
    }

    fn try_visit(&mut self, cluster: u32, key: &G::Key) -> bool {
        let set = self.visited.entry(cluster).or_insert_with(HashSet::new);
        if set.contains(key) {
            false
        } else {
            set.insert(key.clone());
            // Check memory/position limit
            if let Some(maxp) = self.cfg.max_positions {
                if self.total_unique > maxp {
                    self.status = ExitStatus::MemoryLimitReached;
                }
            }
            true
        }
    }

    // Priority squashing based on "outs"
    // Original approach uses a quadratic: pri += round( (a * nout + b) * nout + c )
    // Tunable constants chosen to loosely mirror the original behavior.
    fn squash_priority(&self, pri: i32, nout: usize) -> i32 {
        // Defaults chosen to produce a moderate squashing effect.
        let y0 = 0.0032_f64;
        let y1 = 0.32_f64;
        let y2 = -3.0_f64;
        let x = (y0 * nout as f64 + y1) * nout as f64 + y2;
        let adj = x.round() as i32;
        let mut p = pri + adj;
        if p < 0 {
            p = 0;
        }
        let maxq = (self.cfg.num_queues as i32).saturating_sub(1);
        if p > maxq {
            p = maxq;
        }
        p
    }

    fn enqueue(&mut self, idx: usize, pri: i32) {
        let pri = pri.clamp(0, self.cfg.num_queues as i32 - 1) as usize;
        // Head insert (LIFO), matches "pretending it's a stack"
        self.queues[pri].push_front(idx);
        if (pri as isize) > self.rr_maxq {
            self.rr_maxq = pri as isize;
        }
    }

    fn dequeue(&mut self) -> Option<usize> {
        // Prioritized round-robin sweep
        let mut last = false;
        let mut qpos = self.rr_qpos;
        let mut minpos = self.rr_minpos;

        loop {
            qpos -= 1;
            if qpos < minpos {
                if last {
                    // Nothing to dequeue
                    self.rr_qpos = qpos;
                    self.rr_minpos = minpos;
                    return None;
                }
                qpos = self.rr_maxq;
                minpos -= 1;
                if minpos < 0 {
                    minpos = self.rr_maxq;
                }
                if minpos == 0 {
                    last = true;
                }
            }

            let qp = qpos as usize;
            if let Some(idx) = self.queues[qp].pop_front() {
                // Possibly shrink rr_maxq if emptied
                while self.rr_maxq >= 0
                    && self.rr_maxq as usize == qp
                    && self.queues[qp].is_empty()
                    && self.rr_maxq > 0
                {
                    self.rr_maxq -= 1;
                }

                self.rr_qpos = qpos;
                self.rr_minpos = minpos;
                return Some(idx);
            }
        }
    }

    fn prune_upwards(&mut self, mut idx: usize) {
        // Decrement live children up the parent chain; free nodes implicitly
        while let Some(parent) = self.nodes[idx].parent {
            let p = &mut self.nodes[parent];
            if p.n_children_live > 0 {
                p.n_children_live -= 1;
            }
            if p.n_children_live > 0 {
                break;
            }
            idx = parent;
        }
    }

    fn reconstruct_and_store_win(&mut self, mut idx: usize) {
        // Collect moves from root to idx
        let mut seq: Vec<G::Move> = Vec::new();
        while let Some(node) = self.nodes.get(idx) {
            if let Some(mv) = node.by_move.as_ref() {
                seq.push(mv.clone());
            }
            if let Some(parent) = node.parent {
                idx = parent;
            } else {
                break;
            }
        }
        seq.reverse();
        self.win_moves = seq;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A tiny mock game to validate the engine:
    // State is a single integer; goal is to reach value 3 by +1 moves.
    // No clustering; key is just the integer.
    #[derive(Clone)]
    struct MockGame {
        v: i32,
    }

    #[derive(Clone, Debug)]
    enum MockMove {
        Inc,
    }

    impl Game for MockGame {
        type Move = MockMove;
        type Key = i32;

        fn current_key(&self) -> Self::Key {
            self.v
        }

        fn load_from_key(&mut self, key: &Self::Key) {
            self.v = *key;
        }

        fn cluster_id(&self) -> u32 {
            0
        }

        fn out_count(&self) -> usize {
            self.v as usize
        }

        fn generate_moves(&mut self, moves: &mut Vec<Self::Move>, metas: &mut Vec<MoveMeta>) {
            moves.clear();
            metas.clear();
            if self.v < 3 {
                moves.push(MockMove::Inc);
                metas.push(MoveMeta::new(0, false));
            }
        }

        fn make_move(&mut self, mv: &Self::Move) {
            match mv {
                MockMove::Inc => self.v += 1,
            }
        }

        fn undo_move(&mut self, mv: &Self::Move) {
            match mv {
                MockMove::Inc => self.v -= 1,
            }
        }

        fn is_won(&self) -> bool {
            self.v >= 3
        }
    }

    #[test]
    fn mock_game_solves() {
        let cfg = SolverConfig {
            max_positions: Some(100),
            num_queues: 8,
            defer_auto_out_queueing: true,
        };
        let mut solver = Solver::with_config(MockGame { v: 0 }, cfg);
        let status = solver.run();
        assert_eq!(status, ExitStatus::SolutionExists);
        assert_eq!(solver.win_moves().len(), 3);
        assert_eq!(solver.first_moves().len(), 1);
    }
}