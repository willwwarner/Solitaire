/* test.rs
 * A test game implementation
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

use crate::{card::Card, card_stack::CardStack, runtime::MoveInstruction::{None, Flip}, games::*, runtime};
use gtk::subclass::prelude::*;

pub struct Test {}

const FOUNDATION:usize = 2;

impl Game for Test {
    fn new_game(mut cards: Vec<Card>, grid: &gtk::Grid) -> Self {
        cards.sort_by(|a, b| a.imp().card_id.get().cmp(&b.imp().card_id.get()));
        let stock = CardStack::new(1.4, "stock", -1);
        for _ in 0..52 {
            let card = &cards[0];
            card.flip_to_back();
            runtime::connect_double_click(&card);
            stock.add_card(&card);
            cards.remove(0);
        }
        stock.add_click_to_slot();
        grid.attach(&stock, 0, 0, 1, 1);
        let waste = CardStack::new(1.4, "waste", -1);
        grid.attach(&waste, 1, 0, 1, 1);
        let foundation = CardStack::new(1.4, "foundation", -1);
        foundation.enable_drop();
        grid.attach(&foundation, 0, 1, 1, 1);
        Self {}
    }

    fn verify_drag(&self, bottom_card: &Card, _from_stack: &CardStack) -> bool {
        if bottom_card.is_face_up() { true } else { false }
    }

    fn verify_drop(&self, bottom_card: &Card, to_stack: &CardStack) -> bool {
        if to_stack.get_type() == "waste" {
            if let Some(top_card) = to_stack.last_card() {
                top_card.is_one_rank_above(&bottom_card)
            } else if bottom_card.get_rank() == "king" {
                true
            } else { false }
        } else if to_stack.get_type() == "foundation" {
            if let Some(top_card) = to_stack.last_card() {
                bottom_card.is_one_rank_above(&top_card)
            } else if bottom_card.get_rank() == "ace" {
                true
            } else { false }
        } else { false }
    }

    fn on_drag_completed(&self, _origin_stack: &CardStack, _destination_stack: &CardStack, _move: &mut runtime::Move) {}

    fn pre_undo_drag(&self, _previous_origin_stack: &CardStack, _previous_destination_stack: &CardStack, _move: &mut runtime::Move) {}

    fn on_double_click(&self, card: &Card) {
        let card_stack = card.get_stack().unwrap();
        if card_stack.get_type() == "waste" {
            if let Some(top_card) = card_stack.last_card() {
                let mut perform_move = false;
                let foundation = runtime::get_stack("foundation").unwrap();
                if top_card.get_rank() == "ace" { perform_move = true }
                else if let Some(foundation_top) = &foundation.last_card() {
                    if top_card.is_one_rank_above(foundation_top) { perform_move = true }
                }
                if perform_move {
                    let mut move_ = runtime::create_move("stock", &*top_card.widget_name(), "foundation", None);
                    runtime::perform_move_with_stacks(&mut move_, &card_stack, &foundation);
                    self.on_drag_completed(&card_stack, &foundation, &mut move_);
                    runtime::add_to_history(move_);
                }
            }
        }
    }

    fn on_slot_click(&self, slot: &CardStack) {
        if slot.get_type() == "stock" {
            let waste = runtime::get_stack("waste").unwrap();
            if let Some(top_card) = slot.last_card() {
                let mut move_ = runtime::create_move("stock", &*top_card.widget_name(), "waste", Flip);
                runtime::perform_move_with_stacks(&mut move_, slot, &waste);
                self.on_drag_completed(slot, &waste, &mut move_);
                runtime::add_to_history(move_);
                waste.add_drag_to_card(&top_card);
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
    if state.get_stack(FOUNDATION).len() > 51 { true }
    else { false }
}

fn generate_solver_moves(state: &mut solver::State) {
    const STOCK:usize = 0;
    const WASTE:usize = 1;

    fn get_priority(state: &mut solver::State) -> usize {
        state.get_stack(FOUNDATION).len()
    }

    // stock to waste moves
    if let Some(top_stock) = state.get_stack_owned(STOCK).last() {
        if let Some(top_waste) = state.get_stack(WASTE).last() {
            if solver::is_one_rank_above(&top_stock, &top_waste) {
                state.try_move(solver::create_move(STOCK, &top_stock, WASTE, Flip), 2, get_priority, |_, _, _| {});
            }
        }
        if solver::get_rank(&top_stock) == "king" {
            state.try_move(solver::create_move(STOCK, &top_stock, WASTE, Flip), 2, get_priority, |_, _, _| {});
        }
    }
    // waste to foundation moves
    if let Some(top_waste) = state.get_stack_owned(WASTE).last() {
        if let Some(top_foundation) = state.get_stack(FOUNDATION).last() {
            if solver::is_one_rank_above(&top_foundation, &top_waste) {
                state.try_move(solver::create_move(WASTE, &top_waste, FOUNDATION, None), 1, get_priority, |_, _, _| {});
            }
        }
        if solver::get_rank(&top_waste) == "ace" {
            state.try_move(solver::create_move(WASTE, &top_waste, FOUNDATION, None), 1, get_priority, |_, _, _| {});
        }
    }
}
