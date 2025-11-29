/* solver.rs
 *
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

use super::*;
use crate::runtime::MoveInstruction;

#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    pub origin_stack: usize,
    pub card: u8,
    pub destination_stack: usize,
    pub instruction: MoveInstruction,
    pub flip_index: Option<usize>,
}

// NOTE: IndexMap will panic if origin_stack and destination_stack are the same.
pub fn perform_state_move(move_option: &mut Move, game_state: &mut Vec<Vec<u8>>, undo: bool) {
    debug_assert!(move_option.origin_stack != move_option.destination_stack, "Origin and destination stacks are the same: {move_option:?}");
    let (destination_stack, origin_stack);
    if !undo {
        let Ok([tmp_destination, tmp_origin]) =
            game_state.get_disjoint_mut([move_option.destination_stack, move_option.origin_stack])
        else { panic!("Couldn't get stacks from {move_option:?}") };
        (destination_stack, origin_stack) = (tmp_destination, tmp_origin);
    } else {
        let Ok([tmp_destination, tmp_origin]) =
            game_state.get_disjoint_mut([move_option.destination_stack, move_option.origin_stack])
        else { panic!("Couldn't get stacks from {move_option:?}") };
        (destination_stack, origin_stack) = (tmp_origin, tmp_destination);
    }
    let card_index = origin_stack.iter().position(|x| *x == move_option.card).expect(format!("Couldn't find card {} in {origin_stack:?} undo: {undo}", move_option.card).as_str());
    if move_option.instruction == MoveInstruction::Flip {
        let new_card = origin_stack.last().unwrap();
        move_option.card = card_flipped(new_card);
        for i in (card_index..origin_stack.len()).rev() {
            let mut card = origin_stack.remove(i);
            flip(&mut card);
            destination_stack.push(card);
        }
    } else {
        for _ in card_index..origin_stack.len() {
            let card = origin_stack.remove(card_index);
            destination_stack.push(card);
        }
    }
}

#[derive(PartialEq)]
#[derive(Clone)]
pub struct Node {
    pub parent: Option<usize>,
    pub move_option: Move,
    pub state_key: usize,
}

pub fn solver_card_to_name(card: u8) -> glib::GString {
    let new_id = card & 0x7F;
    if new_id > 54 { return glib::GString::default() }
    match new_id {
        53 => return glib::GString::from("joker_red"),
        54 => return glib::GString::from("joker_black"),
        _ => (),
    }
    let suite_index = (new_id / 13) as usize;
    let rank_index = (new_id % 13) as usize;
    glib::GString::from(format!("{}_{}", SUITES[suite_index], RANKS[rank_index]))
}

pub fn solver_card_to_id(card: &u8) -> u8 {
    card & 0x7F
}
pub fn mut_solver_card_to_id(card: &mut u8) {
    *card &= 0x7F
}

pub fn card_name_to_solver(name: &str, is_flipped: bool) -> u8 {
    let mut name_parts = name.split("_");
    let suite_name = name_parts.next().unwrap();
    let rank_name = name_parts.next().unwrap();
    let suite_index = SUITES.iter().position(|x| x == &suite_name).unwrap();
    let rank_index = RANKS.iter().position(|x| x == &rank_name).unwrap();
    let base_id = ((suite_index * 13) + rank_index) as u8;
    debug_assert!(base_id < 128);
    if is_flipped { base_id | 0x80 } else { base_id & !0x80 }
}

pub fn is_flipped(card: &u8) -> bool {
    (card & 0x80) != 0
}

pub fn flip(card: &mut u8) {
    *card ^= 0x80;
}

pub fn card_flipped(card: &u8) -> u8 {
    card ^ 0x80
}

pub fn is_one_rank_above(card_lower: &u8, card_higher: &u8) -> bool {
    let lower_rank = solver_card_to_id(card_lower) % 13;
    let higher_rank = solver_card_to_id(card_higher) % 13;
    (lower_rank + 1) == higher_rank
}

pub fn is_same_suit(card_1: &u8, card_2: &u8) -> bool {
    (solver_card_to_id(card_1) / 13) == (solver_card_to_id(card_2) / 13)
}

pub fn is_similar_suit(card_1: &u8, card_2: &u8) -> bool {
    let self_suit = (solver_card_to_id(card_1) / 13) as usize;
    let other_suit = (solver_card_to_id(card_2) / 13) as usize;
    (self_suit == 0 || self_suit == 2) == (other_suit == 0 || other_suit == 2)
}

pub fn is_red(card: &u8) -> bool {
    let suit = solver_card_to_id(card) / 13;
    suit == 1 || suit == 3
}

pub fn get_rank(card: &u8) -> &str {
    let rank = solver_card_to_id(card) % 13;
    RANKS[rank as usize]
}

pub fn create_move(origin_stack: usize, card: &u8, destination_stack: usize, instruction: MoveInstruction) -> Move {
    Move {
        origin_stack,
        card: card.to_owned(),
        destination_stack,
        instruction,
        flip_index: None,
    }
}

use indexmap::IndexSet;
use std::collections::VecDeque;

pub struct State {
    game_state: Vec<Vec<u8>>,
    states: IndexSet<Vec<Vec<u8>>>,
    nodes: Vec<Node>,
    queues: Vec<VecDeque<usize>>,
    q_index: usize,
    parent_node: Option<usize>,
}

impl State {
    pub fn get_stack(&self, n: usize) -> &Vec<u8> {
        &self.game_state[n]
    }

    pub fn get_stack_owned(&self, n: usize) -> Vec<u8> {
        self.game_state[n].to_owned()
    }

    pub fn get_stack_mut(&mut self, n: usize) -> &mut Vec<u8> {
        &mut self.game_state[n]
    }

    pub fn get_stacks(&mut self, stacks_n: &[usize]) -> Vec<(usize, Vec<u8>)> {
        let mut result = Vec::new();
        for n in stacks_n { result.push((*n, self.get_stack_owned(*n))) }
        result
    }

    pub fn get_stacks_cards(&mut self, stacks_n: &[usize]) -> Vec<(usize, u8)> {
        let mut result = Vec::new();
        for (n, stack) in self.get_stacks(stacks_n) {
            for card in stack { result.push((n, card)) }
        }
        result
    }

    pub fn get_stacks_top(&mut self, stacks_n: &[usize]) -> Vec<(usize, u8)> {
        let mut result = Vec::new();
        for n in stacks_n {
            if let Some(top_card) = self.get_stack(*n).last() { result.push((*n, *top_card)) }
        }
        result
    }

    pub fn try_move<F: FnMut(&mut Move, &mut State, bool), P: FnMut(&mut State) -> usize>(&mut self, mut move_option: Move, rank: usize, mut priority_fn: P, mut on_move: F) -> bool {
        perform_state_move(&mut move_option, &mut self.game_state, false);
        on_move(&mut move_option, self, false);
        let new_state = self.game_state.clone();
        on_move(&mut move_option, self, true);
        perform_state_move(&mut move_option, &mut self.game_state, true);
        debug_assert_ne!(self.game_state, new_state, "try_move: move did not change state");
        if self.states.insert(new_state) {
            let new_node = Node { parent: self.parent_node, move_option, state_key: self.states.len() - 1 };
            let new_node_index = self.nodes.len();
            self.nodes.push(new_node);
            let outs = priority_fn(self);
            let queue: &mut VecDeque<usize> = self.queues.get_mut(outs).unwrap();
            if outs > self.q_index { queue.push_front(new_node_index); } else { queue.insert(queue.len() / rank, new_node_index); }
            return true;
        }
        return false;
    }
}

pub fn no_onmove(_move: &mut Move, _state: &mut State, _undo: bool) {}

pub fn new_ghost_state(game_state: Vec<Vec<u8>>) -> State {
    State {game_state, states: IndexSet::new(), nodes: Vec::new(), queues: Vec::new(), q_index: 0, parent_node: None}
}

static SHOULD_STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_should_stop(should_stop: bool) {
    SHOULD_STOP.store(should_stop, std::sync::atomic::Ordering::SeqCst);
}

pub fn get_should_stop() -> bool {
    SHOULD_STOP.load(std::sync::atomic::Ordering::Relaxed)
}

pub(super) fn solve<M: FnMut(&mut State), W: FnMut(&mut State) -> bool>
    (game_state: Vec<Vec<u8>>, mut move_generator: M, mut is_won_fn: W) -> Option<Vec<Move>> {

    let mut state = State {game_state, states: IndexSet::new(), nodes: Vec::new(), queues: vec![VecDeque::new(); 53], q_index: 0, parent_node: None};

    let mut n_q_expand = 0;
    let mut last_q_idx = 0;
    let mut expanded = 0;

    // Solver does not work without any moves
    move_generator(&mut state);

    while expanded < 15_000 {
        if expanded % 200 == 0 {
            if get_should_stop() { return None }
        }

        let mut q_index = None;
        let mut highest_q = true;
        for i in (0..53).rev() {
            if !state.queues[i].is_empty() {
                q_index = Some(i);
                if (highest_q && n_q_expand < i) ||
                    (i < last_q_idx || last_q_idx == 0) { break }
                highest_q = false;
            }
        }
        if q_index == None {
            glib::g_message!("solitaire", "solver: failed, n_nodes: {expanded}, n_q_expand: {n_q_expand}");
            return None
        }
        let q_index = q_index.unwrap();
        let queue = state.queues.get_mut(q_index).unwrap();
        let node_index = queue.pop_front().unwrap();
        let node = state.nodes.get(node_index).unwrap();
        state.game_state = state.states.get_index(node.state_key).unwrap().clone();

        if is_won_fn(&mut state) {
            glib::g_message!("solitaire", "solver: found solution, n_nodes: {expanded}, n_q_expand: {n_q_expand}");
            let mut history = Vec::new();
            let mut node = state.nodes.get(node_index).unwrap();

            while let Some(node_index) = node.parent {
                history.push(node.move_option.to_owned());
                node = state.nodes.get(node_index).unwrap();
            }
            history.push(node.move_option.to_owned()); // Don't forget the first move!
            history.reverse();

            return Some(history);
        }

        state.parent_node = Some(node_index);
        move_generator(&mut state);
        expanded += 1;
        if q_index == last_q_idx { n_q_expand += 1; }
        else { last_q_idx = q_index; n_q_expand = 0; }
    }
    glib::g_message!("solitaire", "solver: met node limit, n_nodes: {expanded}, n_q_expand: {n_q_expand}");

    Some(Vec::new())
}

#[cfg(feature = "solver-debug")]
pub(super) fn solver_debug(parent: &crate::window::SolitaireWindow, game_state: Vec<Vec<u8>>, stack_names: Vec<String>) {
    use adw::prelude::*;

    thread_local! {
        static DEBUG_STATE:std::cell::RefCell<Option<State>> = std::cell::RefCell::new(None);
        static DEBUG_STACK_NAMES:std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    }

    let builder = gtk::Builder::from_resource("/org/gnome/gitlab/wwarner/Solitaire/solver-debug.ui");
    let dialog = builder.object::<adw::Dialog>("solver-debug-dialog").unwrap();
    let node_list = builder.object::<gtk::ListBox>("node_list").unwrap();
    let node_view = builder.object::<gtk::TextView>("node_view").unwrap();
    let expand_button = builder.object::<gtk::Button>("expand_button").unwrap();
    DEBUG_STATE.set(Some(State {game_state, states: IndexSet::new(), nodes: Vec::new(), queues: vec![VecDeque::new(); 53], q_index: 0, parent_node: None}));
    DEBUG_STACK_NAMES.set(stack_names);

    fn get_nodes() -> Vec<Node> {
        let state = DEBUG_STATE.take().unwrap();
        let nodes = state.nodes.clone();
        DEBUG_STATE.set(Some(state));
        nodes
    }

    let make_node_rows = |node_list: &gtk::ListBox, node_view: &gtk::TextView, start_index: usize| {
        let nodes = get_nodes();
        for i in start_index..nodes.len() {
            let move_ = &nodes[i].move_option;
            let stack_names = DEBUG_STACK_NAMES.with(|v| v.borrow().clone());
            let move_str = format!("{}-{}->{} !{:?}", stack_names[move_.origin_stack], solver_card_to_name(move_.card), stack_names[move_.destination_stack], move_.instruction);
            let row = adw::ActionRow::builder().title(move_str).activatable(true).build();
            let state_key = nodes[i].state_key;
            let view_ref = node_view.clone();
            row.connect_activated(move |_| {
                let mut state = DEBUG_STATE.take().unwrap();
                state.game_state = state.states[state_key].clone();
                let mut text = String::new();
                let stack_names = DEBUG_STACK_NAMES.with(|v| v.borrow().clone());
                for i in 0..state.game_state.len() {
                    text.push_str(&format!("{}: ", stack_names[i]));
                    for card_id in &state.game_state[i] {
                        text.push_str(&format!("{}, ", solver_card_to_name(*card_id)));
                    }
                    text.push_str("\n\n");
                }
                view_ref.buffer().set_text(&text);
                DEBUG_STATE.set(Some(state));
            });
            node_list.append(&row);
        }
    };

    let mut game = CURRENT_GAME.lock().unwrap();
    let game = game.as_mut().expect("solver_debug: failed to get game");
    let mut state = DEBUG_STATE.take().unwrap();
    game.get_move_generator()(&mut state);
    DEBUG_STATE.set(Some(state));
    make_node_rows(&node_list, &node_view, 0);

    expand_button.connect_clicked(move |button| {
        let mut game = CURRENT_GAME.lock().unwrap();
        if let Some(game) = game.as_mut() {
            let n_expand = button.ancestor(adw::SpinRow::static_type()).unwrap().downcast::<adw::SpinRow>().unwrap().value() as u32;
            for _ in 0..n_expand {
                let mut state = DEBUG_STATE.take().unwrap();
                let node_index = state.nodes.len();
                game.get_move_generator()(&mut state);
                if let Some(node) = state.nodes.last() {
                    state.game_state = state.states[node.state_key].clone();
                }
                DEBUG_STATE.set(Some(state));
                make_node_rows(&node_list, &node_view, node_index);
            }
        }
    });

    let main_loop = glib::MainLoop::new(None, false);
    let main_loop_ref = main_loop.clone();
    dialog.connect_closed(move |_| main_loop_ref.quit());
    dialog.present(Some(parent));
    main_loop.run();
}
