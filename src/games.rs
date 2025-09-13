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

use indexmap::IndexSet;
use std::collections::{HashMap, VecDeque};
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

fn perform_state_move(move_option: &mut runtime::Move, game_state: &mut HashMap<String, Vec<glib::GString>>, undo: bool) {
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
    let card_index = origin_stack.iter().position(|x| *x == move_option.card_name).expect(format!("Couldn't find card {} in {origin_stack:?} undo: {undo}", move_option.card_name).as_str());
    if move_option.instruction == Some("flip".to_string()) {
        let new_card = origin_stack.last().unwrap().to_string();
        move_option.card_name = new_card;
        for i in (card_index..origin_stack.len()).rev() {
            let card = origin_stack.remove(i);
            if card.ends_with("_b") {
                destination_stack.push(glib::GString::from(card.replace("_b", "")));
            } else {
                destination_stack.push(glib::GString::from(card.to_string() + "_b"));
            }
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
    move_option: runtime::Move,
    state_key: usize,
}

fn get_stack_keyset(game_state: &HashMap<String, Vec<glib::GString>>, stack_names: &Vec<String>, stacks: &mut HashMap<u32, Vec<u8>>) -> Vec<u32> {
    let mut stack_keys = Vec::new();
    for name in stack_names.iter() {
        let stack = game_state.get(name).unwrap();
        let stack_shortened = stack.iter().map(|x| card_name_to_id(x)).collect::<Vec<u8>>();
        let mut stack_key:u32 = 0x811C9DC5;
        let stack_iter = stack_shortened.iter().rev();
        for card in stack_iter {
            stack_key ^= *card as u32;
            stack_key = stack_key.wrapping_mul(0x01000193);
        }
        let stack_key = stack_key;
        stacks.entry(stack_key).or_insert(stack_shortened);
        stack_keys.push(stack_key);
    }

    stack_keys
}

fn card_id_to_name(id: u8) -> glib::GString {
    if id > 54 { return glib::GString::default() }
    match id {
        53 => return glib::GString::from("joker_red"),
        54 => return glib::GString::from("joker_black"),
        _ => (),
    }
    let suite_index = (id / 13) as usize;
    let rank_index = (id % 13) as usize;
    glib::GString::from(format!("{}_{}", SUITES[suite_index], RANKS[rank_index]))
}

pub fn card_name_to_id(name: &glib::GString) -> u8 {
    let mut name_parts = name.split("_");
    let suite_name =name_parts.next().unwrap();
    let rank_name = name_parts.next().unwrap();
    let suite_index = SUITES.iter().position(|x| x == &suite_name).unwrap();
    let rank_index = RANKS.iter().position(|x| x == &rank_name).unwrap();
    ((suite_index * 13) + rank_index) as u8
}

pub fn is_one_rank_above(card_lower: &glib::GString, card_higher: &glib::GString) -> bool {
    let lower_rank = card_lower.split("_").nth(1).unwrap();
    let higher_rank = card_higher.split("_").nth(1).unwrap();
    let lower_index = RANKS.iter().position(|x| x == &lower_rank).unwrap();
    let higher_index = RANKS.iter().position(|x| x == &higher_rank).unwrap();

    if lower_index + 1 == higher_index {
        true
    } else {
        false
    }
}

pub fn is_same_suit(card_1: &glib::GString, card_2: &glib::GString) -> bool {
    (card_1.starts_with("heart")   && card_2.starts_with("heart")   ) ||
        (card_1.starts_with("diamond") && card_2.starts_with("diamond") ) ||
        (card_1.starts_with("club")    && card_2.starts_with("club")    ) ||
        (card_1.starts_with("spade")   && card_2.starts_with("spade")   )
}

pub fn is_similar_suit(card_1: &glib::GString, card_2: &glib::GString) -> bool {
    (card_1.starts_with("heart") || card_1.starts_with("diamond")) ==
        (card_2.starts_with("heart") || card_2.starts_with("diamond"))
}


pub fn solve_game() -> Option<Vec<runtime::Move>> {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        glib::g_message!("solitaire", "solver: starting");
        // Get the game state
        let mut game_state = HashMap::new();
        let mut stack_names = Vec::new();
        let grid = runtime::get_grid().unwrap();
        let stacks = grid.observe_children();
        for i in 0..stacks.n_items() {
            let stack = stacks.item(i).unwrap().downcast::<CardStack>().unwrap();
            game_state.insert(stack.widget_name().to_string(), stack.get_solver_stack());
            stack_names.push(stack.widget_name().to_string());
        }
        let stack_names = stack_names;
        let mut states:IndexSet<Vec<u32>> = IndexSet::new();
        let mut stacks:HashMap<u32, Vec<u8>> = HashMap::new();
        let mut nodes:Vec<SolverNode> = Vec::new();
        let mut queues:Vec<VecDeque<usize>> = vec![VecDeque::new(); 50];
        let mut n_q_expand = 0;
        let mut last_q_idx = 0;
        let mut expanded = 0;

        let moves = game.get_automoves_ranked(&game_state);
        for mut move_option in moves {
            perform_state_move(&mut move_option, &mut game_state, false);
            let stack_keys = get_stack_keyset(&game_state, &stack_names, &mut stacks);
            let outs = game.get_priority(&game_state) as usize;
            perform_state_move(&mut move_option, &mut game_state, true);
            if !states.contains(&stack_keys) {
                states.insert(stack_keys);
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
        println!("Initial Nodes: {}", nodes.len());

        while expanded < 200_000 {
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
            let stack_keys = &states[node.state_key];
            for stack_key in stack_keys {
                let stack = stacks.get(stack_key).unwrap();
                let mut new_stack = Vec::new();
                for card_id in stack {
                    let card_name = card_id_to_name(*card_id);
                    new_stack.push(card_name);
                }
                game_state.insert(names_iter.next().unwrap().to_owned(), new_stack);
            }
            if game.is_won(&game_state) {
                glib::g_message!("solitaire", "solver: found solution");
                let mut history = Vec::new();
                let mut node = node;
                history.push(node.move_option.to_owned());
                while let Some(node_index) = node.parent {
                    node = nodes.get(node_index).unwrap();
                    let move_option = node.move_option.to_owned();
                    history.push(move_option);
                }
                history.reverse();
                return Some(history);
            }
            let moves = game.get_automoves_ranked(&game_state);
            for mut move_option in moves {
                perform_state_move(&mut move_option, &mut game_state, false);
                let stack_keys = get_stack_keyset(&game_state, &stack_names, &mut stacks);
                let outs = game.get_priority(&game_state) as usize;
                perform_state_move(&mut move_option, &mut game_state, true);
                if !states.contains(&stack_keys) {
                    states.insert(stack_keys);
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
    let mut game_state = HashMap::new();
    let stack_names = vec![String::from("A"), String::from("B")];
    let mut stack_a = Vec::new();
    for i in 0..10 { stack_a.push(card_id_to_name(i)); }
    game_state.insert(stack_names[0].clone(), stack_a);
    let stack_b = Vec::new();
    game_state.insert(stack_names[1].clone(), stack_b);
    let mut stacks:HashMap<u32, Vec<u8>> = HashMap::new();

    // ROUND TRIP CHECK
    let keys = get_stack_keyset(&game_state, &stack_names, &mut stacks); // Vec<u32>
    let mut rec = HashMap::new();
    let mut it = stack_names.iter();
    for k in &keys {
        let compact = stacks.get(k).unwrap();
        let mut v = Vec::new();
        for &id in compact { v.push(card_id_to_name(id)); }
        rec.insert(it.next().unwrap().to_owned(), v);
    }
    assert_eq!(game_state, rec, "Round-trip state mismatch!");

    // SAMPLE MOVE UNDO CHECK
    let moves = vec![runtime::create_move("A", "club_6", "B", None),
                     runtime::create_move("A", "club_ace", "B", None),
                     runtime::create_move("A", "club_6", "B", Some("flip")),
                     runtime::create_move("A", "club_ace", "B", Some("flip"))];
    for mut mv in moves {
        let mv_copy = mv.clone();
        let mut copy = game_state.clone();
        perform_state_move(&mut mv, &mut copy, false);
        perform_state_move(&mut mv, &mut copy, true);
        assert_eq!(game_state, copy, "move/undo mismatch for {:?}", mv);
        assert_eq!(mv, mv_copy, "move/undo mismatch for {:?}", mv);
    }
}

pub trait Game: Send + Sync {
    fn new_game(cards: Vec<Card>, grid: &gtk::Grid) -> Self where Self: Sized;
    fn verify_drag(&self, bottom_card: &Card, from_stack: &CardStack) -> bool;
    fn verify_drop(&self, bottom_card: &Card, to_stack: &CardStack) -> bool;
    fn on_drag_completed(&self, origin_stack: &CardStack);
    fn on_drop_completed(&self, recipient_stack: &CardStack);
    fn pre_undo_drag(&self, previous_origin_stack: &CardStack, previous_destination_stack: &CardStack);
    fn on_double_click(&self, card: &Card);
    fn undo_deal(&self, stock: &CardStack);
    fn on_slot_click(&self, slot: &CardStack);
    fn get_automoves_ranked(&self, state: &HashMap<String, Vec<glib::GString>>) -> Vec<runtime::Move>;
    fn get_priority(&self, state: &HashMap<String, Vec<glib::GString>>) -> u32;
    fn is_won(&self, state: &HashMap<String, Vec<glib::GString>>) -> bool;
}