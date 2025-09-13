/* klondike.rs
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

use crate::{runtime, runtime::MoveInstruction, card::Card, card_stack::CardStack};
use gtk::{prelude::*, subclass::prelude::*};
use gtk::glib;
use super::*;

pub struct Klondike {}

impl Klondike {}

const FOUNDATION:&[usize] = &[7, 8, 9, 10];

impl Game for Klondike {
    fn new_game(mut cards: Vec<Card>, grid: &gtk::Grid) -> Self {
        let mut n_cards = cards.len() as i32;

        for i in 0..7 {
            let card_stack = CardStack::new(2.8 /* 2 card heights */, "tableau", i);

            for j in 0..(i + 1) {
                let random_card = glib::random_int_range(0, n_cards) as usize;
                if let Some(card) = cards.get(random_card) {
                    card_stack.add_card(&card);
                    if j < i { card.flip() }
                    card_stack.add_drag_to_card(&card);
                    runtime::connect_double_click(&card);
                    cards.remove(random_card);
                } else {
                    glib::g_error!("solitaire", "Failed to get card");
                }
                n_cards -= 1;
            }

            grid.attach(&card_stack, i, 1, 1, 2);

            card_stack.enable_drop();
        }

        for i in 0..4 {
            let card_stack = CardStack::new(1.4, "foundation", i);
            grid.attach(&card_stack, i + 3, 0, 1, 1);
            card_stack.enable_drop();
        }

        let waste = CardStack::new(1.4, "waste", -1);
        grid.attach(&waste, 1, 0, 1, 1);

        let stock = CardStack::new(1.4, "stock", -1);
        stock.add_click_to_slot();
        while n_cards > 0 {
            let random_card = glib::random_int_range(0, n_cards) as usize;
            if let Some(card) = cards.get(random_card) {
                stock.add_card(&card);
                card.flip();
                runtime::connect_double_click(&card);
                cards.remove(random_card);
            } else {
                glib::g_error!("solitaire", "Failed to get card from grid");
            }
            n_cards -= 1;
        }
        grid.attach(&stock, 0, 0, 1, 1);

        Self {}
    }
    fn verify_drag(&self, bottom_card: &Card, _from_stack: &CardStack) -> bool {
        if !bottom_card.imp().is_face_up.get() { false } else { true }
    }

    fn verify_drop(&self, bottom_card: &Card, to_stack: &CardStack) -> bool {
        let stack_type = to_stack.get_type();
        if stack_type == "tableau" {
            if to_stack.is_empty() && bottom_card.get_rank() == "king" { return true }
            else if to_stack.is_empty() { return false }
            let top_card = to_stack.last_card().unwrap();
            if (!bottom_card.is_similar_suit(&top_card)) && top_card.is_one_rank_above(&bottom_card) { return true }
            else { return false }
        } else if stack_type == "foundation" {
            if to_stack.is_empty() && bottom_card.get_rank() == "ace" { return true }
            else if to_stack.is_empty() { return false }
            let top_card = to_stack.last_card().unwrap();
            if bottom_card.is_same_suit(&top_card) && bottom_card.is_one_rank_above(&top_card) { return true }
            else { false }
        } else { false }
    }

    fn on_drag_completed(&self, origin_stack: &CardStack, destination_stack: &CardStack, move_: &mut runtime::Move) {
        if origin_stack.get_type() == "tableau" {
            if let Some(last_card) = origin_stack.last_card() {
                if !last_card.is_face_up() {
                    move_.flip_index = Some(origin_stack.n_cards() - 1);
                    last_card.flip();
                }
            }
        }
    }

    fn pre_undo_drag(&self, origin_stack: &CardStack, dropped_stack: &CardStack, move_: &mut runtime::Move) {
        if origin_stack.get_type() == "tableau" {
            if let Some(flip_index) = move_.flip_index {
                origin_stack.get_card(flip_index).unwrap().flip();
            }
        } else if origin_stack.get_type() == "stock" {
            origin_stack.face_down_top_card();
        }
    }

    fn on_double_click(&self, card: &Card) {
        let card_stack = card.get_stack().unwrap();
        if card_stack.get_type() == "foundation" {
            return
        } else {
            try_distribute(card, &card_stack, self);
        }
    }

    fn on_slot_click(&self, slot: &CardStack) {
        if slot.get_type() == "stock" {
            let waste = runtime::get_stack("waste").unwrap();

            if slot.is_empty() {
                let n_deals = runtime::get_deals();
                if n_deals >= 3 { return }
                runtime::update_deals(n_deals + 1);
                //Fixme: Don't use widget_name
                let mut move_ = runtime::create_move("waste",
                                                     &waste.first_card().unwrap().widget_name(),
                                                     "stock",
                                                     MoveInstruction::Flip);
                runtime::perform_move(&mut move_);
                runtime::add_to_history(move_);
            } else {
                let card = slot.last_card().unwrap();
                slot.remove_card(&card);
                card.flip();
                waste.add_card(&card);
                waste.add_drag_to_card(&card);
                card.remove_css_class("highlight");
                runtime::add_to_history(runtime::create_move(&slot.widget_name(),
                                                             &card.widget_name(),
                                                             "waste",
                                                             MoveInstruction::Flip));
            }
        }
    }
    fn get_move_generator(&self) -> Box<dyn FnMut(&mut solver::State)> {
        Box::new(generate_solver_moves)
    }

    fn get_is_won_fn(&self) -> Box<dyn FnMut(&mut solver::State) -> bool> {
        Box::new(is_won)
    }
}

fn is_won(state: &mut solver::State) -> bool {
    for i in 0..4 {
        let stack = state.get_stack(FOUNDATION[i]);
        if let Some(last_child) = stack.last() {
            if !(solver::get_rank(last_child) == "king") {
                return false;
            }
        } else {
            // If one of the foundations is empty, the game is not won
            return false;
        }
    }
    true
}

fn generate_solver_moves(state: &mut solver::State) {
    const STOCK:usize = 12;
    const WASTE:usize = 11;
    const TABLEAU:&[usize] = &[0, 1, 2, 3, 4, 5, 6];

    fn get_priority(state: &mut solver::State) -> usize {
        let mut outs = 0; // outs = number of cards that are out (in foundations)
        for (_i, outpile) in state.get_stacks(FOUNDATION) {
            outs += outpile.len();
        }
        outs
    }

    fn set_if_greater(a: &mut u8, b: &u8) {
        if *a < *b { *a = *b }
    }

    fn onmove(move_option: &mut solver::Move, state: &mut solver::State, undo: bool) {
        if TABLEAU.contains(&move_option.origin_stack) {
            let origin_stack = state.get_stack_mut(move_option.origin_stack);
            if undo {
                if let Some(flip_index) = move_option.flip_index {
                    let card = origin_stack.get_mut(flip_index).unwrap();
                    solver::flip(card);
                }
            } else {
                if let Some(card) = origin_stack.last_mut() {
                    if solver::is_flipped(&card) {
                        solver::flip(card);
                        move_option.flip_index = Some(origin_stack.len() - 1);
                    }
                }
            }
        }
    }

    // Check for moves to foundation
    for (i, tableau_card) in state.get_stacks_top(&[11, 0, 1, 2, 3, 4, 5, 6]) { // from waste and tableau
        if solver::is_flipped(&tableau_card) { continue } // this should never happen anyways
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
                if solver::get_rank(&tableau_card) == "ace" {
                    state.try_move(solver::create_move(i, &tableau_card, j, MoveInstruction::None), 100, get_priority, onmove);
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
                    state.try_move(move_option, 100, get_priority, onmove);
                    return; // for performance reasons we suggest only automoves, if we find one
                }
            } else {
                if rank_id <= max_black || rank_id < 2 {
                    state.try_move(move_option, 100, get_priority, onmove);
                    return; // for performance reasons we suggest only automoves, if we find one
                }
            }
            state.try_move(move_option, 3, get_priority, onmove);
        }
    }

    let stock = state.get_stack_owned(STOCK);
    if !stock.is_empty() {
        state.try_move(solver::create_move(STOCK, stock.last().unwrap(), WASTE, MoveInstruction::Flip), 5, get_priority, solver::no_onmove);
    }

    // Check where to put a king
    let mut first_empty_stack:Option<usize> = None;
    for (i, tableau_stack) in state.get_stacks(TABLEAU) {
        if tableau_stack.is_empty() {
            first_empty_stack = Some(i);
            break;
        }
    }

    for (i, from_stack) in state.get_stacks(&[0, 1, 2, 3, 4, 5, 6, 11]) { // from tableau and waste
        let mut len = from_stack.len();
        if i == 11 && len > 0 { len = 1 } // we can only draw from the top of waste (once)
        for from_card_i in 0..len {
            let from_card = if i == 11 { from_stack.last().unwrap() } else { &from_stack[from_card_i] }; // we can only draw from the top of waste
            if solver::is_flipped(&from_card) { continue }
            for (j, to_card) in state.get_stacks_top(TABLEAU) {
                if j == i { continue }
                if solver::is_one_rank_above(&from_card, &to_card) && !solver::is_similar_suit(&from_card, &to_card) {
                    if i == 11 {
                        state.try_move(solver::create_move(i, &from_card, j, MoveInstruction::None), 40, get_priority, onmove);
                    } else if from_card_i > 0 && solver::is_flipped(&from_stack[from_card_i - 1]) { // does the move flip a card?
                        state.try_move(solver::create_move(i, &from_card, j, MoveInstruction::None), 30, get_priority, onmove);
                    } else {
                        state.try_move(solver::create_move(i, &from_card, j, MoveInstruction::None), 1, get_priority, onmove);
                    }
                }
            }
            if solver::get_rank(&from_card) == "king" && first_empty_stack.is_some() {
                if from_card_i == 0 { continue } // don't move kings if they are placed
                if i == 11 {
                    state.try_move(solver::create_move(i, &from_card, first_empty_stack.unwrap(), MoveInstruction::None), 40, get_priority, onmove);
                } else if solver::is_flipped(&from_stack[from_card_i - 1]) { // does the move flip a card?
                    state.try_move(solver::create_move(i, &from_card, first_empty_stack.unwrap(), MoveInstruction::None), 30, get_priority, onmove);
                } else {
                    state.try_move(solver::create_move(i, &from_card, first_empty_stack.unwrap(), MoveInstruction::None), 1, get_priority, onmove);
                }
            }
        }
    }

    let waste = state.get_stack(WASTE);
    if stock.is_empty() && !waste.is_empty() {
        state.try_move(solver::create_move(WASTE, &waste[0], STOCK, MoveInstruction::Flip), 2, get_priority, solver::no_onmove);
    }
}

fn try_distribute(card: &Card, parent: &CardStack, game: &Klondike) {
    if !card.imp().is_face_up.get() { return }
    if &parent.last_card().unwrap() != card { return }

    for i in 0..4 {
        let stack = runtime::get_stack(format!("foundation_{i}").as_str()).unwrap();
        if let Some(last_card) = stack.last_card() {
            if last_card.is_same_suit(card) && card.is_one_rank_above(&last_card) {
                let mut move_ = runtime::create_move(&parent.widget_name(),
                                                     &card.widget_name(),
                                                     &stack.widget_name(),
                                                     MoveInstruction::None);
                runtime::perform_move_with_stacks(&mut move_, parent, &stack);
                game.on_drag_completed(parent, &stack, &mut move_);
                runtime::add_to_history(move_);
                return
            }
        } else {
            if card.get_rank() == "ace" {
                let mut move_ = runtime::create_move(&parent.widget_name(),
                                                     &card.widget_name(),
                                                     &stack.widget_name(),
                                                     MoveInstruction::None);
                runtime::perform_move_with_stacks(&mut move_, parent, &stack);
                game.on_drag_completed(parent, &stack, &mut move_);
                runtime::add_to_history(move_);
                return
            }
        }
    }
}
