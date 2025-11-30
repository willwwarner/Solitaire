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

use std::sync::Mutex;
use adw::prelude::*;
use gtk::{gio, glib};
use gettextrs::gettext;
use crate::{renderer, card::Card, card_stack::CardStack, runtime};

#[cfg(debug_assertions)]
mod test;
mod klondike;

pub const JOKERS: [&str; 2] = ["joker_red", "joker_black"];
pub const SUITES: [&str; 4] = ["club", "heart", "spade", "diamond"];
pub const RANKS: [&str; 13] = ["ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king"];
static CURRENT_GAME: Mutex<Option<Box<dyn Game>>> = Mutex::new(None);

pub fn load_game(game_name: &str, grid: &gtk::Grid) {
    let window = crate::window::SolitaireWindow::get_window().unwrap();
    window.lookup_action("undo").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(false);
    window.lookup_action("redo").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(false);
    window.set_hint_drop_enabled(false);

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
    match game_name {
        #[cfg(debug_assertions)]
        "Test" => *game = Some(Box::new(test::Test::new_game(cards, &grid))),
        "Klondike" => *game = Some(Box::new(klondike::Klondike::new_game(cards, &grid))),
        _ => panic!("Unknown game: {}", game_name),
    }
    

    // Log game loading
    println!("Loaded game: {}", game_name);
}

pub fn unload(grid: &gtk::Grid) {
    let mut game = CURRENT_GAME.lock().unwrap();
    *game = None;
    runtime::clear_history_and_moves();
    runtime::clear_state();
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
    vec![
        #[cfg(debug_assertions)] gettext("Test"),
        gettext("Klondike")
    ] //, "Spider", "FreeCell", "Tri-Peaks", "Pyramid", "Yukon"] not yet :)
}

pub fn get_game_description(game_name: &str) -> String {
    match game_name {
        #[cfg(debug_assertions)]
        "Test" => gettext("Test Game"),
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

pub fn on_drag_completed(origin_stack: &CardStack, destination_stack: &CardStack, move_: &mut runtime::Move) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_drag_completed(origin_stack, destination_stack, move_);
    }
}

pub fn pre_undo_drag(origin_stack: &CardStack, dropped_stack: &CardStack, move_: &mut runtime::Move) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.pre_undo_drag(origin_stack, dropped_stack, move_);
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

pub fn get_is_won_fn() -> Box<dyn FnMut(&mut solver::State) -> bool> {
    let mut game = CURRENT_GAME.lock().unwrap();
    game.as_mut().unwrap().get_is_won_fn()
}

pub mod solver;

pub async fn try_game(game_name: &str, card_grid: &gtk::Grid) -> Option<Vec<runtime::Move>> {
    solver::set_should_stop(false);
    for _ in 0..3 {
        if solver::get_should_stop() { return None; }
        load_game(game_name, &card_grid);
        let mut stack_names = Vec::new();
        let stacks = card_grid.observe_children();
        let mut game_state = Vec::new();
        for i in 0..stacks.n_items() {
            let stack = stacks.item(i).unwrap().downcast::<CardStack>().unwrap();
            game_state.push(stack.get_solver_stack());
            stack_names.push(stack.widget_name().to_string());
        }
        #[cfg(feature = "solver-debug")]
        solver::solver_debug(&crate::window::SolitaireWindow::get_window().unwrap(), game_state.clone(), stack_names.clone());

        let (sender, receiver) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let mut game = CURRENT_GAME.lock().unwrap();
            if let Some(game) = game.as_mut() {
                let result = solver::solve(game_state, game.get_move_generator(), game.get_is_won_fn());
                sender.send_blocking(result).unwrap();
            }
        });
        while let Ok(result) = receiver.recv().await {
            if let Some(solver_history) = result {
                let mut history = Vec::new();
                for move_option in &solver_history {
                    history.push(runtime::Move {
                        origin_stack: stack_names[move_option.origin_stack].clone(),
                        card_name: solver::solver_card_to_name(move_option.card).to_string(),
                        destination_stack: stack_names[move_option.destination_stack].clone(),
                        instruction: move_option.instruction.clone(),
                        flip_index: move_option.flip_index
                    });
                }
                for move_option in &history {
                    println!("{:?}", move_option);
                }
                return Some(history);
            }
        }
        unload(&card_grid);
    }

    // Couldn't find a solution
    None
}

pub fn re_solve(stack_names: Vec<String>, game_state: Vec<Vec<u8>>) -> Option<Vec<runtime::Move>> {
    let (move_generator, is_won_fn);
    {
        let mut game = CURRENT_GAME.lock().unwrap();
        let game = game.as_mut().unwrap();
        move_generator = game.get_move_generator();
        is_won_fn = game.get_is_won_fn();
    }
    solver::set_should_stop(false);
    let result = solver::solve(game_state, move_generator, is_won_fn);
    let mut history = Vec::new();
    for move_option in result? {
        history.push(runtime::Move {
            origin_stack: stack_names[move_option.origin_stack].clone(),
            card_name: solver::solver_card_to_name(move_option.card).to_string(),
            destination_stack: stack_names[move_option.destination_stack].clone(),
            instruction: move_option.instruction.clone(),
            flip_index: move_option.flip_index
        });
    }
    Some(history)
}

pub fn test_solver_state() {
    use runtime::MoveInstruction::{None, Flip};
    let mut game_state = Vec::new();
    let mut stack_a = Vec::new();
    stack_a.push(solver::card_name_to_solver(&glib::GString::from("club_ace"), false));
    for i in 2..7 { stack_a.push(solver::card_name_to_solver(&glib::GString::from(format!("club_{i}")), false)); }
    for i in 7..9 {stack_a.push(solver::card_name_to_solver(&glib::GString::from(format!("club_{i}")), true)); }
    game_state.insert(0, stack_a);
    let stack_b = Vec::new();
    game_state.insert(1, stack_b);

    // SAMPLE MOVE UNDO CHECK
    let moves = vec![solver::create_move(0, &solver::card_name_to_solver("club_6", false), 1, None),
                     solver::create_move(0, &solver::card_name_to_solver("club_ace", false), 1, None),
                     solver::create_move(0, &solver::card_name_to_solver("club_6", false), 1, Flip),
                     solver::create_move(0, &solver::card_name_to_solver("club_ace", false), 1, Flip)];

    for mut mv in moves {
        let mv_copy = mv.clone();
        let mut copy = game_state.clone();
        solver::perform_state_move(&mut mv, &mut copy, false);
        solver::perform_state_move(&mut mv, &mut copy, true);
        assert_eq!(game_state, copy, "move/undo mismatch for {:?}", mv);
        assert_eq!(mv, mv_copy, "move/undo mismatch for {:?}", mv);
    }

    assert_eq!(solver::card_name_to_solver("club_ace", false), solver::card_flipped(&solver::card_name_to_solver("club_ace", true)));

    let card_name = "club_6";
    let card_id = solver::card_name_to_solver(card_name, false);
    assert_eq!(card_name, solver::solver_card_to_name(card_id), "Card name mismatch for {card_id}");
    assert_eq!(solver::card_flipped(&card_id), solver::card_name_to_solver(card_name, true), "Card ID mismatch for {card_name}");
}

trait Game: Send + Sync {
    fn new_game(cards: Vec<Card>, grid: &gtk::Grid) -> Self where Self: Sized;
    fn verify_drag(&self, bottom_card: &Card, from_stack: &CardStack) -> bool;
    fn verify_drop(&self, bottom_card: &Card, to_stack: &CardStack) -> bool;
    fn on_drag_completed(&self, origin_stack: &CardStack, destination_stack: &CardStack, move_: &mut runtime::Move);
    fn pre_undo_drag(&self, previous_origin_stack: &CardStack, previous_destination_stack: &CardStack, move_: &mut runtime::Move);
    fn on_double_click(&self, card: &Card);
    fn on_slot_click(&self, slot: &CardStack);
    fn get_move_generator(&self) -> Box<dyn FnMut(&mut solver::State)>;
    fn get_is_won_fn(&self) -> Box<dyn FnMut(&mut solver::State) -> bool>;
}
