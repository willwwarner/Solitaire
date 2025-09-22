/* games.rs
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

use indexmap::{IndexSet, IndexMap};
use std::collections::{VecDeque};
use std::sync::Mutex;
use std::format;
use gtk::prelude::*;
use gtk::{gio, glib};
use gettextrs::gettext;
use crate::{renderer, card::Card, card_stack::CardStack, runtime};

mod klondike;

pub const JOKERS: [&str; 2] = ["joker_red", "joker_black"];
pub const SUITES: [&str; 4] = ["club", "heart", "spade", "diamond"];
pub const RANKS: [&str; 13] = ["ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king"];
static CURRENT_GAME: Mutex<Option<Box<dyn Game>>> = Mutex::new(None);

pub fn load_game(game_name: &str, grid: &gtk::Grid) {
    let window = grid.root().unwrap().downcast::<gtk::Window>().unwrap().downcast::<crate::window::SolitaireWindow>().unwrap();
    window.lookup_action("undo").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(false);
    window.lookup_action("redo").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(false);
    window.lookup_action("hint").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(false);

    let mut cards = runtime::get_cards();
    if cards.is_empty() {
        // Create the renderer
        glib::g_message!("solitaire", "Loading SVG");
        let resource = gio::resources_lookup_data("/org/gnome/gitlab/wwarner/Solitaire/assets/anglo_poker.svg", gio::ResourceLookupFlags::NONE)
            .expect("Failed to load resource data");
        glib::g_message!("solitaire", "loaded resource data");
        let handle = rsvg::Loader::new()
            .read_stream(&gio::MemoryInputStream::from_bytes(&resource), None::<&gio::File>, None::<&gio::Cancellable>)
            .expect("Failed to load SVG");
        let renderer = rsvg::CairoRenderer::new(&handle);
        glib::g_message!("solitaire", "Done Loading SVG");

        for i in 0..52 {
            let card_name = format!("{}_{}", SUITES[i / 13], RANKS[i % 13]);
            let card = Card::new(&*card_name, i as u8, &renderer);
            cards.push(card);
        }
        renderer::set_back_texture(&renderer);
        glib::g_message!("solitaire", "Done setting textures");
    }

    // Store the current game type
    let mut game = CURRENT_GAME.lock().unwrap();
    *game = Some(Box::new(klondike::Klondike::new_game(cards, &grid)));

    // Log game loading
    println!("Loaded game: {}", game_name);
}

pub fn unload(grid: &gtk::Grid) {
    let mut game = CURRENT_GAME.lock().unwrap();
    *game = None;
    runtime::clear_history_and_moves();
    runtime::update_deals(0);
    let items = grid.observe_children().n_items();
    let mut cards = Vec::new();
    for _ in 0..items {
        let child = grid.first_child().expect("Couldn't get child");
        child.downcast::<CardStack>().unwrap().destroy_and_return_cards(&mut cards);
    }
    runtime::set_cards(cards);
}

pub fn get_games() -> Vec<String> {
    vec![gettext("Klondike")] //, "Spider", "FreeCell", "Tri-Peaks", "Pyramid", "Yukon"]; not yet :)
}

pub fn get_game_description(game_name: &str) -> String {
    match game_name {
        "Klondike" => gettext("Classic Solitaire"),
        _ => "".to_string()
    }
}

pub fn on_double_click(card: &Card) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_double_click(card);
    }
}

pub fn on_slot_click(slot: &CardStack) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_slot_click(slot);
    }
}

pub fn on_drag_completed(origin_stack: &CardStack) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_drag_completed(origin_stack);
    }
}

pub fn on_drop_completed(recipient_stack: &CardStack) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_drop_completed(recipient_stack);
    }
}

pub fn pre_undo_drag(origin_stack: &CardStack, dropped_stack: &CardStack) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.pre_undo_drag(origin_stack, dropped_stack);
    }
}

pub fn verify_drag(bottom_card: &Card, from_stack: &CardStack) -> bool {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.verify_drag(bottom_card, from_stack)
    } else {
        false
    }
}

pub fn verify_drop(bottom_card: &Card, to_stack: &CardStack) -> bool {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.verify_drop(bottom_card, to_stack)
    } else {
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
struct SolverMove {
    pub origin_stack: String,
    pub card: u8,
    pub destination_stack: String,
    pub instruction: Option<String>,
}

// NOTE: IndexMap will panic if origin_stack and destination_stack are the same.
fn perform_state_move(move_option: &mut SolverMove, game_state: &mut IndexMap<String, Vec<u8>>, undo: bool) {
    debug_assert!(move_option.origin_stack != move_option.destination_stack, "Origin and destination stacks are the same: {move_option:?}");
    let (destination_stack, origin_stack);
    if !undo {
        let [Some(tmp_destination), Some(tmp_origin)] =
            game_state.get_disjoint_mut([&move_option.destination_stack, &move_option.origin_stack]) 
        else { panic!("Couldn't get stacks from {move_option:?}") };
        (destination_stack, origin_stack) = (tmp_destination, tmp_origin);
    } else {
        let [Some(tmp_destination), Some(tmp_origin)] =
            game_state.get_disjoint_mut([&move_option.destination_stack, &move_option.origin_stack])
        else { panic!("Couldn't get stacks from {move_option:?}") };
        (destination_stack, origin_stack) = (tmp_origin, tmp_destination);
    }
    let card_index = origin_stack.iter().position(|x| *x == move_option.card).expect(format!("Couldn't find card {} in {origin_stack:?} undo: {undo}", move_option.card).as_str());
    if move_option.instruction == Some("flip".to_string()) {
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
struct SolverNode {
    parent: Option<usize>,
    move_option: SolverMove,
    state_key: usize,
}

fn solver_card_to_name(card: u8) -> glib::GString {
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

fn solver_card_to_id(card: &u8) -> u8 {
    card & 0x7F
}
fn mut_solver_card_to_id(card: &mut u8) {
    *card &= 0x7F
}

pub fn card_name_to_solver(name: &str, is_flipped: bool) -> u8 {
    let mut name_parts = name.split("_");
    let suite_name =name_parts.next().unwrap();
    let rank_name = name_parts.next().unwrap();
    let suite_index = SUITES.iter().position(|x| x == &suite_name).unwrap();
    let rank_index = RANKS.iter().position(|x| x == &rank_name).unwrap();
    let base_id = ((suite_index * 13) + rank_index) as u8;
    debug_assert!(base_id < 128);
    if is_flipped { base_id | 0x80 } else { base_id & !0x80 }
}

fn is_flipped(card: &u8) -> bool {
    (card & 0x80) != 0
}

fn flip(card: &mut u8) {
    *card ^= 0x80;
}

fn card_flipped(card: &u8) -> u8 {
    card ^ 0x80
}

fn is_one_rank_above(card_lower: &u8, card_higher: &u8) -> bool {
    let lower_rank = solver_card_to_id(card_lower) % 13;
    let higher_rank = solver_card_to_id(card_higher) % 13;
    (lower_rank + 1) == higher_rank
}

fn is_same_suit(card_1: &u8, card_2: &u8) -> bool {
    (solver_card_to_id(card_1) / 13) == (solver_card_to_id(card_2) / 13)
}

fn is_similar_suit(card_1: &u8, card_2: &u8) -> bool {
    let self_suit = (solver_card_to_id(card_1) / 13) as usize;
    let other_suit = (solver_card_to_id(card_2) / 13) as usize;
    (self_suit == 0 || self_suit == 2) == (other_suit == 0 || other_suit == 2)
}

fn get_rank(card: &u8) -> &str {
    let rank = solver_card_to_id(card) % 13;
    RANKS[rank as usize]
}

fn create_move(origin_stack: &str, card: &u8, destination_stack: &str, instruction: Option<&str>) -> SolverMove {
    let new_instruction;
    if instruction.is_some() { new_instruction = Some(instruction.unwrap().to_string()) }
    else { new_instruction = None}
    SolverMove {
        origin_stack: origin_stack.to_string(),
        card: card.to_owned(),
        destination_stack: destination_stack.to_string(),
        instruction: new_instruction,
    }
}

fn move_from_strings(origin_stack: String, card: &u8, destination_stack: String, instruction: Option<String>) -> SolverMove {
    SolverMove {
        origin_stack,
        card: card.to_owned(),
        destination_stack,
        instruction,
    }
}


pub fn solve_game(mut game_state: IndexMap<String, Vec<u8>>, stack_names: Vec<String>) -> Option<Vec<runtime::Move>> {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        glib::g_message!("solitaire", "solver: starting");
        let mut states:IndexSet<Vec<usize>> = IndexSet::new();
        let mut stacks:IndexSet<Vec<u8>> = IndexSet::new();
        let mut nodes:Vec<SolverNode> = Vec::new();
        let mut queues:Vec<VecDeque<usize>> = vec![VecDeque::new(); 50];
        let mut n_q_expand = 0;
        let mut last_q_idx = 0;
        let mut expanded = 0;

        let moves = game.get_automoves_ranked(&game_state);
        println!("Initial Moves: {:?}", moves);
        for mut move_option in moves {
            perform_state_move(&mut move_option, &mut game_state, false);
            game.solver_on_move(&move_option, &mut game_state, false);
            let mut new_stack_keys = Vec::new();
            for stack in game_state.values() {
                if let Some(stack_index) = stacks.get_index_of(stack) {
                    new_stack_keys.push(stack_index);
                } else {
                    new_stack_keys.push(stacks.len());
                    stacks.insert(stack.to_owned());
                }
            }
            let outs = game.get_priority(&game_state) as usize;
            game.solver_on_move(&move_option, &mut game_state, true);
            perform_state_move(&mut move_option, &mut game_state, true);
            if states.insert(new_stack_keys) {
                let new_node = SolverNode { parent: None, move_option, state_key: states.len() - 1 };
                let new_node_index = nodes.len();
                nodes.push(new_node);
                if let Some(queue) = queues.get_mut(outs) {
                    queue.push_back(new_node_index);
                } else {
                    queues[49].push_back(new_node_index);
                }
            }
        }

        while expanded < 20_000 {
            let mut q_index = usize::MAX; // Use MAX instead of -1, because usize
            let mut highest_q = true;
            for i in (0..50).rev() {
                if queues[i].is_empty() { continue }
                q_index = i;
                if (highest_q && n_q_expand < i) ||
                   (i < last_q_idx || last_q_idx == 0) { break }
                highest_q = false;
            }
            if q_index == usize::MAX { println!("expanded: {expanded}"); return None }
            let queue = queues.get_mut(q_index).unwrap();
            let node_index = queue.pop_front().unwrap();
            let node = nodes.get(node_index).unwrap();
            game_state.clear();
            let mut names_iter = stack_names.iter();
            let stack_keys = states.get_index(node.state_key).unwrap();
            for stack_key in stack_keys {
                let stack = stacks.get_index(*stack_key).unwrap();
                let mut new_stack = Vec::new();
                for card_id in stack {
                    new_stack.push(*card_id);
                }
                game_state.insert(names_iter.next().unwrap().to_owned(), new_stack);
            }
            if game.is_won(&game_state) {
                glib::g_message!("solitaire", "solver: found solution");
                let mut history = Vec::new();
                let mut node = node;
                while let Some(node_index) = node.parent {
                    let move_option = node.move_option.to_owned();
                    history.push(runtime::move_from_strings(move_option.origin_stack, solver_card_to_name(move_option.card).to_string(), move_option.destination_stack, move_option.instruction));
                    node = nodes.get(node_index).unwrap();
                }
                history.reverse();
                return Some(history);
            }
            let moves = game.get_automoves_ranked(&game_state);
            for mut move_option in moves {
                perform_state_move(&mut move_option, &mut game_state, false);
                game.solver_on_move(&move_option, &mut game_state, false);
                let mut new_stack_keys = Vec::new();
                for stack in game_state.values() {
                    if let Some(stack_index) = stacks.get_index_of(stack) {
                        new_stack_keys.push(stack_index);
                    } else {
                        new_stack_keys.push(stacks.len());
                        stacks.insert(stack.to_owned());
                    }
                }
                let outs = game.get_priority(&game_state) as usize;
                game.solver_on_move(&move_option, &mut game_state, true);
                perform_state_move(&mut move_option, &mut game_state, true);
                if states.insert(new_stack_keys) {
                    let new_node = SolverNode { parent: Some(node_index), move_option, state_key: states.len() - 1 };
                    let new_node_index = nodes.len();
                    nodes.push(new_node);
                    if let Some(queue) = queues.get_mut(outs) {
                        if outs > q_index { queue.push_front(new_node_index); }
                        else {queue.push_back(new_node_index); }
                    } else {
                        queues[49].push_back(new_node_index);
                    }
                }
            }
            expanded += 1;
            if q_index == last_q_idx { n_q_expand += 1; }
            else { last_q_idx = q_index; n_q_expand = 0; }
        }
        glib::g_message!("solitaire", "solver: finished, n_nodes: {expanded}, n_q_expand: {n_q_expand}");
        for i in 0..50 {
            println!("Queue-{i}: {}", queues[i].len());
        }
    }

    // Couldn't find a solution
    None
}

pub fn test_solver_state() {
    let mut game_state = IndexMap::new();
    let stack_names = vec![String::from("A"), String::from("B")];
    let mut stack_a = Vec::new();
    stack_a.push(card_name_to_solver(&glib::GString::from("club_ace"), false));
    for i in 2..7 { stack_a.push(card_name_to_solver(&glib::GString::from(format!("club_{i}")), false)); }
    for i in 7..9 {stack_a.push(card_name_to_solver(&glib::GString::from(format!("club_{i}")), true)); }
    game_state.insert(stack_names[0].clone(), stack_a);
    let stack_b = Vec::new();
    game_state.insert(stack_names[1].clone(), stack_b);
    let mut stacks:IndexSet<Vec<u8>> = IndexSet::new();

    let mut keys = Vec::new();
    for stack in game_state.values() {
        if let Some(stack_index) = stacks.get_index_of(stack) {
            keys.push(stack_index);
        } else {
            keys.push(stacks.len());
            stacks.insert(stack.to_owned());
        }
    }
    let mut new_state = IndexMap::new();
    let mut names_iter = stack_names.iter();
    for key in &keys {
        let stack = stacks.get_index(*key).unwrap();
        new_state.insert(names_iter.next().unwrap().to_owned(), stack.to_owned());
    }
    assert_eq!(game_state, new_state, "Round-trip state mismatch!");

    // SAMPLE MOVE UNDO CHECK
    let moves = vec![create_move("A", &card_name_to_solver("club_6", false), "B", None),
                     create_move("A", &card_name_to_solver("club_ace", false), "B", None),
                     create_move("A", &card_name_to_solver("club_6", false), "B", Some("flip")),
                     create_move("A", &card_name_to_solver("club_ace", false), "B", Some("flip"))];

    for mut mv in moves {
        let mv_copy = mv.clone();
        let mut copy = game_state.clone();
        perform_state_move(&mut mv, &mut copy, false);
        perform_state_move(&mut mv, &mut copy, true);
        assert_eq!(game_state, copy, "move/undo mismatch for {:?}", mv);
        assert_eq!(mv, mv_copy, "move/undo mismatch for {:?}", mv);
    }
}

trait Game: Send + Sync {
    fn new_game(cards: Vec<Card>, grid: &gtk::Grid) -> Self where Self: Sized;
    fn verify_drag(&self, bottom_card: &Card, from_stack: &CardStack) -> bool;
    fn verify_drop(&self, bottom_card: &Card, to_stack: &CardStack) -> bool;
    fn on_drag_completed(&self, origin_stack: &CardStack);
    fn on_drop_completed(&self, recipient_stack: &CardStack);
    fn pre_undo_drag(&self, previous_origin_stack: &CardStack, previous_destination_stack: &CardStack);
    fn on_double_click(&self, card: &Card);
    fn on_slot_click(&self, slot: &CardStack);
    fn get_automoves_ranked(&self, state: &IndexMap<String, Vec<u8>>) -> Vec<SolverMove>;
    fn solver_on_move(&self, move_option: &SolverMove, state: &mut IndexMap<String, Vec<u8>>, undo: bool);
    fn get_priority(&self, state: &IndexMap<String, Vec<u8>>) -> u32;
    fn is_won(&self, state: &IndexMap<String, Vec<u8>>) -> bool;
}