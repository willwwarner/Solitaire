/* freecell.rs
 *
 * Copyright 2026 Will Warner
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
use crate::{
    card::Card, card_stack::CardStack, game_board::GameBoard, runtime, runtime::MoveInstruction,
};
use gtk::glib;
use gtk::{prelude::*, subclass::prelude::*};

pub struct FreeCell {}

impl FreeCell {}

const FOUNDATION: &[usize] = &[4, 5, 6, 7];

impl Game for FreeCell {
    fn new_game(mut cards: Vec<Card>, game_board: &GameBoard) -> Self {
        let mut n_cards = cards.len() as i32;

        for i in 0..4 {
            let card_stack = CardStack::new("cell", i, false);
            game_board.add(&card_stack, i, 0, 1, 1);
            card_stack.enable_drop();
        }

        for i in 0..4 {
            let card_stack = CardStack::new("foundation", i, false);
            game_board.add(&card_stack, i + 5, 0, 1, 1);
            card_stack.enable_drop();
        }

        for i in 0..8 {
            let card_stack = CardStack::new("tableau", i, true);
            let cards_needed = if i < 4 { 7 } else { 6 };

            for _ in 0..cards_needed {
                let random_card = glib::random_int_range(0, n_cards) as usize;
                if let Some(card) = cards.get(random_card) {
                    card_stack.add_card(&card);
                    card_stack.add_drag_to_card(&card);
                    runtime::connect_double_click(&card);
                    cards.remove(random_card);
                } else {
                    glib::g_error!("solitaire", "Failed to get card");
                }
                n_cards -= 1;
            }

            game_board.add_float(&card_stack, i as f64 + 0.5, 1.0, 1.0, 2.5);

            card_stack.enable_drop();
        }

        Self {}
    }

    fn verify_drag(&self, bottom_card: &Card, from_stack: &CardStack) -> bool {
        if from_stack.stack_type() == "tableau" {
            let mut card = from_stack.last_card().unwrap();
            let mut last_card: Option<Card> = None;
            loop {
                if let Some(above) = last_card {
                    if above.is_similar_suit(&card) || !card.is_one_rank_above(&above) {
                        return false;
                    }
                }

                if &card == bottom_card {
                    break;
                }
                last_card = Some(card.clone());
                card = card.prev_sibling().unwrap().downcast::<Card>().unwrap();
            }
        }
        true
    }

    fn verify_drop(&self, transfer_stack: &TransferCardStack, to_stack: &CardStack) -> bool {
        let stack_type = to_stack.stack_type();
        if stack_type == "foundation" && transfer_stack.n_cards() == 1 {
            let bottom_card = transfer_stack.first_card();
            if to_stack.is_empty() && bottom_card.rank() == "ace" {
                return true;
            } else if to_stack.is_empty() {
                return false;
            }
            let top_card = to_stack.last_card().unwrap();
            if bottom_card.is_same_suit(&top_card) && bottom_card.is_one_rank_above(&top_card) {
                return true;
            }
        } else if stack_type == "cell" {
            if to_stack.n_cards() == 0 && transfer_stack.n_cards() == 1 {
                return true;
            }
        } else if stack_type == "tableau" {
            if to_stack.is_empty() {
                return true;
            }
            let top_card = to_stack.last_card().unwrap();
            let bottom_card = transfer_stack.first_card();
            if (!bottom_card.is_similar_suit(&top_card)) && top_card.is_one_rank_above(&bottom_card)
            {
                return true;
            }
        }
        false
    }

    fn drag_completed(
        &self,
        _origin_stack: &CardStack,
        _destination_stack: &CardStack,
        _move: &mut runtime::Move,
    ) {
    }

    fn pre_undo_drag(
        &self,
        _origin_stack: &CardStack,
        _dropped_stack: &CardStack,
        _move: &mut runtime::Move,
    ) {
    }

    fn card_double_click(&self, card: &Card) {
        let card_stack = card.stack().unwrap();
        if card_stack.stack_type() == "foundation" {
            return;
        } else {
            try_distribute(card, &card_stack, self);
        }
    }

    fn stack_click(&self, _slot: &CardStack) {}

    fn move_generator(&self) -> Box<dyn FnMut(&mut solver::State)> {
        Box::new(generate_solver_moves)
    }

    fn is_won_fn(&self) -> Box<dyn FnMut(&mut solver::State) -> bool> {
        Box::new(is_won)
    }
}

fn is_won(state: &mut solver::State) -> bool {
    for i in 0..4 {
        let stack = state.get_stack(FOUNDATION[i]);
        if let Some(last_child) = stack.last() {
            if !(solver::card_rank(last_child) == "king") {
                return false;
            }
        } else {
            // If one of the foundations is empty, the game is not won
            return false;
        }
    }
    true
}

#[rustfmt::skip]
fn generate_solver_moves(state: &mut solver::State) {
    const CELLS: &[usize] = &[0, 1, 2, 3];
    const TABLEAU: &[usize] = &[8, 9, 10, 11, 12, 13, 14, 15];

    fn get_priority(state: &mut solver::State) -> usize {
        let mut outs = 0; // outs = number of cards that are out (in foundations)
        for (_i, outpile) in state.get_stacks(FOUNDATION) {
            outs += outpile.len();
        }
        for (_i, cell) in state.get_stacks(CELLS) {
            outs = outs.saturating_sub(cell.len());
        }
        outs
    }

    fn set_if_greater(a: &mut u8, b: &u8) {
        if *a < *b {
            *a = *b
        }
    }

    // Check for moves to foundation
    for (i, tableau_card) in state.get_stacks_top(&[0, 1, 2, 3, 8, 9, 10, 11, 12, 13, 14, 15]) {
        // from waste and tableau
        let mut max_red = 0;
        let mut max_black = 0;
        let mut consider_moves = Vec::new();
        for (j, foundation_stack) in state.get_stacks(FOUNDATION) {
            if let Some(foundation_card) = foundation_stack.last() {
                let rank_id = solver::solver_card_to_id(&foundation_card) % 13;
                if solver::is_red(&foundation_card) { set_if_greater(&mut max_black, &(rank_id + 1)) }
                else { set_if_greater(&mut max_red, &(rank_id + 1)) }
                if solver::is_same_suit(&foundation_card, &tableau_card) && solver::is_one_rank_above(&foundation_card, &tableau_card) {
                    consider_moves.push(solver::create_move(i, &tableau_card, j, MoveInstruction::None));
                }
            } else {
                if solver::card_rank(&tableau_card) == "ace" {
                    state.try_move(solver::create_move(i, &tableau_card, j, MoveInstruction::None), 100, get_priority, solver::no_onmove);
                    return; // for performance reasons we suggest only automoves, if we find one
                }
            }
        }
        // Make sure automoves are safe
        for move_option in consider_moves {
            let card = move_option.card;
            let rank_id = solver::solver_card_to_id(&card) % 13;
            if solver::is_red(&card) {
                // if the card rank is less than 3, moving it is probably not consequential
                if rank_id <= max_red || rank_id < 2 {
                    state.try_move(move_option, 100, get_priority, solver::no_onmove);
                    return; // for performance reasons we suggest only automoves, if we find one
                }
            } else {
                if rank_id <= max_black || rank_id < 2 {
                    state.try_move(move_option, 100, get_priority, solver::no_onmove);
                    return; // for performance reasons we suggest only automoves, if we find one
                }
            }
            state.try_move(move_option, 3, get_priority, solver::no_onmove);
        }
    }

    // Check for an open tableau
    let mut first_empty_tableau: Option<usize> = None;
    for (i, tableau_stack) in state.get_stacks(TABLEAU) {
        if tableau_stack.is_empty() {
            first_empty_tableau = Some(i);
            break;
        }
    }

    // Check for an open cell
    let mut first_empty_cell: Option<usize> = None;
    for (i, cell_stack) in state.get_stacks(CELLS) {
        if cell_stack.is_empty() {
            first_empty_cell = Some(i);
            break;
        }
    }

    // Check for moves to tableau & cells
    for (i, from_stack) in state.get_stacks(&[0, 1, 2, 3, 8, 9, 10, 11, 12, 13, 14, 15]) {
        for from_card_i in 0..from_stack.len() {
            let from_card = &from_stack[from_card_i];
            'outer: for (j, to_card) in state.get_stacks_top(TABLEAU) {
                if j == i { continue }
                if solver::is_one_rank_above(&from_card, &to_card) && !solver::is_similar_suit(&from_card, &to_card) {
                    if i < 8 {
                        state.try_move(solver::create_move(i, &from_card, j, MoveInstruction::None), 40, get_priority, solver::no_onmove);
                    } else {
                        let mut last_card: Option<&u8> = None;
                        for card in from_stack.iter().rev() {
                            if let Some(above) = last_card {
                                if solver::is_similar_suit(&above, &card) || !solver::is_one_rank_above(&above, &card) {
                                    continue 'outer;
                                }
                            }

                            if card == from_card { break; }
                            last_card = Some(card);
                        }
                        state.try_move(solver::create_move(i, &from_card, j, MoveInstruction::None), 1, get_priority, solver::no_onmove);
                    }
                } else if first_empty_tableau.is_some() {
                    if from_card_i == 0 {
                        continue;
                    }
                    if i < 8 {
                        state.try_move(solver::create_move(i, &from_card, first_empty_tableau.unwrap(), MoveInstruction::None), 40, get_priority, solver::no_onmove);
                    } else {
                        state.try_move(solver::create_move(i, &from_card, first_empty_tableau.unwrap(), MoveInstruction::None), 1, get_priority, solver::no_onmove);
                    }
                }
            }
        }
        if i > 7 && first_empty_cell.is_some() {
            if let Some(last) = from_stack.last() {
                state.try_move(solver::create_move(i, &last, first_empty_cell.unwrap(), MoveInstruction::None), 1, get_priority, solver::no_onmove);
            }
        }
    }
}

fn try_distribute(card: &Card, parent: &CardStack, game: &FreeCell) {
    if !card.imp().is_face_up.get() {
        return;
    }
    if &parent.last_card().unwrap() != card {
        return;
    }

    for i in 0..4 {
        let stack = runtime::get_stack(format!("foundation_{i}").as_str()).unwrap();
        if let Some(last_card) = stack.last_card() {
            if last_card.is_same_suit(card) && card.is_one_rank_above(&last_card) {
                let mut move_ = runtime::create_move(
                    &parent.widget_name(),
                    &card.widget_name(),
                    &stack.widget_name(),
                    MoveInstruction::None,
                );
                runtime::perform_move_with_stacks(&mut move_, parent, &stack);
                game.drag_completed(parent, &stack, &mut move_);
                runtime::add_to_history(move_);
                return;
            }
        } else {
            if card.rank() == "ace" {
                let mut move_ = runtime::create_move(
                    &parent.widget_name(),
                    &card.widget_name(),
                    &stack.widget_name(),
                    MoveInstruction::None,
                );
                runtime::perform_move_with_stacks(&mut move_, parent, &stack);
                game.drag_completed(parent, &stack, &mut move_);
                runtime::add_to_history(move_);
                return;
            }
        }
    }
}
