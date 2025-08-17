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
        if !bottom_card.imp().is_face_up.get() { false }
        else { true }
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
                runtime::add_to_history(runtime::create_move(&slot.widget_name(),
                                                             &card.widget_name(),
                                                             "waste",
                                                             MoveInstruction::Flip));
            }
        }
    }

    fn is_won(&self) -> bool {
        for i in 0..4 {
            let stack = runtime::get_stack(format!("foundation_{i}").as_str()).unwrap();
            if let Some(last_card) = stack.last_card() {
                if !(last_card.get_rank() == "king") {
                    return false;
                }
            }
        }
        true
    }

    fn get_best_next_move(&self) -> Option<(String, String, String)> {
        todo!()
    }

    fn is_winnable(&self) -> bool {
        todo!()
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