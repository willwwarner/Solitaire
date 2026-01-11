/* games.rs
 *
 * Copyright 2025-2026 Will Warner
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

use crate::{
    card::Card,
    card_stack::{CardStack, TransferCardStack},
    game_board::GameBoard,
    renderer, runtime,
};
use adw::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use std::sync::Mutex;

mod klondike;
mod freecell;
#[cfg(debug_assertions)]
mod test;
mod tri_peaks;

pub const JOKERS: [&str; 2] = ["joker_red", "joker_black"];
pub const SUITES: [&str; 4] = ["club", "diamond", "heart", "spade"]; // Use this order because it is the AisleRiot card theme order
pub const RANKS: [&str; 13] = [
    "ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king",
];

static CURRENT_GAME: Mutex<Option<Box<dyn Game>>> = Mutex::new(None);

pub fn load_game(game_name: &str, game_board: &GameBoard) {
    let window = crate::window::SolitaireWindow::get_window().unwrap();
    window
        .lookup_action("undo")
        .unwrap()
        .downcast::<gio::SimpleAction>()
        .unwrap()
        .set_enabled(false);
    window
        .lookup_action("redo")
        .unwrap()
        .downcast::<gio::SimpleAction>()
        .unwrap()
        .set_enabled(false);
    window.set_hint_drop_enabled(false);

    let mut cards = runtime::get_cards();
    let theme_name = renderer::get_requested_theme();
    if theme_name != renderer::ACTIVE_THEME.with_borrow(|t| t.clone()) {
        cards.clear();
    }

    if cards.is_empty() {
        let card_theme = renderer::get_card_theme(&theme_name);
        renderer::create_cards(&card_theme, &mut cards);
        renderer::ACTIVE_THEME.set(theme_name);
    }

    // Store the current game type
    let mut game = CURRENT_GAME.lock().unwrap();
    match game_name {
        #[cfg(debug_assertions)]
        "Test" => *game = Some(Box::new(test::Test::new_game(cards, &game_board))),
        "Klondike" => *game = Some(Box::new(klondike::Klondike::new_game(cards, &game_board))),
        "FreeCell" => *game = Some(Box::new(freecell::FreeCell::new_game(cards, &game_board))),
        "Tri-Peaks" => *game = Some(Box::new(tri_peaks::TriPeaks::new_game(cards, &game_board))),
        _ => panic!("Unknown game: {}", game_name),
    }
}

pub fn unload(game_board: &GameBoard) {
    let mut game = CURRENT_GAME.lock().unwrap();
    *game = None;
    game_board.reset_positions();
    runtime::clear_history_and_moves();
    runtime::clear_state();
    runtime::update_deals(0);
    let items = game_board.observe_children().n_items();
    let mut cards = Vec::new();
    for _ in 0..items {
        let child = game_board.first_child().expect("Couldn't get child");
        child
            .downcast::<CardStack>()
            .unwrap()
            .destroy_and_return_cards(&mut cards);
    }
    runtime::set_cards(cards);
    game_board.reset_layout();
}

pub fn get_games() -> Vec<String> {
    vec![
        #[cfg(debug_assertions)]
        gettext("Test"),
        gettext("Klondike"),
        gettext("FreeCell"),
        gettext("Tri-Peaks"),
    ] //, "Spider", "Pyramid", "Yukon"] not yet :)
}

pub fn get_game_description(game_name: &str) -> String {
    match game_name {
        #[cfg(debug_assertions)]
        "Test" => gettext("Test Game"),
        "Klondike" => gettext("Classic Solitaire"),
        "FreeCell" => gettext("Build Foundations using Free Cells"),
        "Tri-Peaks" => gettext("Clear Three Peaks of Cards"),
        _ => "".to_string(),
    }
}

pub fn on_double_click(card: &Card) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.card_double_click(card);
    }
}

pub fn stack_click(stack: &CardStack) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.stack_click(stack);
    }
}

pub fn drag_completed(
    origin_stack: &CardStack,
    destination_stack: &CardStack,
    move_: &mut runtime::Move,
) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.drag_completed(origin_stack, destination_stack, move_);
    }
}

pub fn pre_undo_drag(
    origin_stack: &CardStack,
    dropped_stack: &CardStack,
    move_: &mut runtime::Move,
) {
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

pub fn verify_drop(transfer_stack: &TransferCardStack, to_stack: &CardStack) -> bool {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.verify_drop(transfer_stack, to_stack)
    } else {
        false
    }
}

pub fn is_won_fn() -> Box<dyn FnMut(&mut solver::State) -> bool> {
    let mut game = CURRENT_GAME.lock().unwrap();
    game.as_mut().unwrap().is_won_fn()
}

pub mod solver;

pub async fn try_game(game_name: &str, game_board: &GameBoard) -> Option<Vec<runtime::Move>> {
    solver::set_should_stop(false);
    for _ in 0..3 {
        if solver::get_should_stop() {
            return None;
        }
        load_game(game_name, &game_board);
        let mut stack_names = Vec::new();
        let stacks = game_board.observe_children();
        let mut game_state = Vec::new();
        for i in 0..stacks.n_items() {
            let stack = stacks.item(i).unwrap().downcast::<CardStack>().unwrap();
            game_state.push(stack.get_solver_stack());
            stack_names.push(stack.widget_name().to_string());
        }
        #[cfg(feature = "solver-debug")]
        solver::solver_debug(
            &crate::window::SolitaireWindow::get_window().unwrap(),
            game_state.clone(),
            stack_names.clone(),
        );

        let (sender, receiver) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let mut game = CURRENT_GAME.lock().unwrap();
            if let Some(game) = game.as_mut() {
                let result = solver::solve(game_state, game.move_generator(), game.is_won_fn());
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
                        flip_index: move_option.flip_index,
                    });
                }
                for move_option in &history {
                    println!("{:?}", move_option);
                }
                return Some(history);
            }
        }
        unload(&game_board);
    }

    // Couldn't find a solution
    None
}

pub fn re_solve(stack_names: Vec<String>, game_state: Vec<Vec<u8>>) -> Option<Vec<runtime::Move>> {
    let (move_generator, is_won_fn);
    {
        let mut game = CURRENT_GAME.lock().unwrap();
        let game = game.as_mut().unwrap();
        move_generator = game.move_generator();
        is_won_fn = game.is_won_fn();
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
            flip_index: move_option.flip_index,
        });
    }
    Some(history)
}

pub fn test_solver_state() {
    use runtime::MoveInstruction::{Flip, None};
    let mut game_state = Vec::new();
    let mut stack_a = Vec::new();
    stack_a.push(solver::card_name_to_solver(
        &glib::GString::from("club_ace"),
        false,
    ));
    for i in 2..7 {
        stack_a.push(solver::card_name_to_solver(
            &glib::GString::from(format!("club_{i}")),
            false,
        ));
    }
    for i in 7..9 {
        stack_a.push(solver::card_name_to_solver(
            &glib::GString::from(format!("club_{i}")),
            true,
        ));
    }
    game_state.insert(0, stack_a);
    let stack_b = Vec::new();
    game_state.insert(1, stack_b);

    // SAMPLE MOVE UNDO CHECK
    let moves = vec![
        solver::create_move(0, &solver::card_name_to_solver("club_6", false), 1, None),
        solver::create_move(0, &solver::card_name_to_solver("club_ace", false), 1, None),
        solver::create_move(0, &solver::card_name_to_solver("club_6", false), 1, Flip),
        solver::create_move(0, &solver::card_name_to_solver("club_ace", false), 1, Flip),
    ];

    for mut mv in moves {
        let mv_copy = mv.clone();
        let mut copy = game_state.clone();
        solver::perform_state_move(&mut mv, &mut copy, false);
        solver::perform_state_move(&mut mv, &mut copy, true);
        assert_eq!(game_state, copy, "move/undo mismatch for {:?}", mv);
        assert_eq!(mv, mv_copy, "move/undo mismatch for {:?}", mv);
    }

    assert_eq!(
        solver::card_name_to_solver("club_ace", false),
        solver::card_flipped(&solver::card_name_to_solver("club_ace", true))
    );

    let card_name = "club_6";
    let card_id = solver::card_name_to_solver(card_name, false);
    assert_eq!(
        card_name,
        solver::solver_card_to_name(card_id),
        "Card name mismatch for {card_id}"
    );
    assert_eq!(
        solver::card_flipped(&card_id),
        solver::card_name_to_solver(card_name, true),
        "Card ID mismatch for {card_name}"
    );
}

trait Game: Send + Sync {
    fn new_game(cards: Vec<Card>, game_board: &GameBoard) -> Self
    where
        Self: Sized;
    fn verify_drag(&self, bottom_card: &Card, from_stack: &CardStack) -> bool;
    fn verify_drop(&self, transfer_stack: &TransferCardStack, to_stack: &CardStack) -> bool;
    fn drag_completed(
        &self,
        origin_stack: &CardStack,
        destination_stack: &CardStack,
        move_: &mut runtime::Move,
    );
    fn pre_undo_drag(
        &self,
        previous_origin_stack: &CardStack,
        previous_destination_stack: &CardStack,
        move_: &mut runtime::Move,
    );
    fn card_double_click(&self, card: &Card);
    fn stack_click(&self, slot: &CardStack);
    fn move_generator(&self) -> Box<dyn FnMut(&mut solver::State)>;
    fn is_won_fn(&self) -> Box<dyn FnMut(&mut solver::State) -> bool>;
}
