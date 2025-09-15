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

use crate::{runtime, card::Card, card_stack::CardStack};
use gtk::{prelude::*, subclass::prelude::*};
use gtk::glib;

pub struct Klondike {}

impl Klondike {}
impl super::Game for Klondike {
    fn new_game(mut cards: Vec<Card>, grid: &gtk::Grid) -> Self {
        let mut n_cards = cards.len() as i32;

        for i in 0..7 {
            let card_stack = CardStack::new();
            card_stack.set_widget_name(format!("tableau_{i}").as_str());
            card_stack.set_aspect(2.8); // 2 card heights

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
            let card_stack = CardStack::new();
            card_stack.set_widget_name(format!("foundation_{i}").as_str());
            grid.attach(&card_stack, i + 3, 0, 1, 1);
            card_stack.enable_drop();
        }

        let waste = CardStack::new();
        waste.set_widget_name("waste");
        grid.attach(&waste, 1, 0, 1, 1);

        let stock = CardStack::new();
        stock.add_click_to_slot();
        stock.set_widget_name("stock");
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
        let stack_name = to_stack.widget_name();
        if stack_name.starts_with("tableau") {
            if to_stack.is_empty() && bottom_card.get_rank() == "king" { return true }
            else if to_stack.is_empty() { return false }
            let top_card = to_stack.last_card().unwrap();
            if (!bottom_card.is_similar_suit(&top_card)) && top_card.is_one_rank_above(&bottom_card) { return true }
            else { return false }
        }
        else if stack_name.starts_with("foundation") {
            if to_stack.is_empty() && bottom_card.get_rank() == "ace" { return true }
            else if to_stack.is_empty() { return false }
            let top_card = to_stack.last_card().unwrap();
            if bottom_card.is_same_suit(&top_card) && bottom_card.is_one_rank_above(&top_card) { return true }
            else { false }
        }
        else { false }
    }

    fn on_drag_completed(&self, origin_stack: &CardStack) {
        if origin_stack.widget_name().starts_with("tableau") {
            origin_stack.face_up_top_card(); // This returns if the stack is empty or not
        }
    }

    fn on_drop_completed(&self, recipient_stack: &CardStack) {
        if recipient_stack.widget_name() == "waste" {
            recipient_stack.face_up_top_card();
        }
    }

    fn pre_undo_drag(&self, origin_stack: &CardStack, dropped_stack: &CardStack) {
        if origin_stack.widget_name().starts_with("tableau") {
            origin_stack.face_down_top_card(); // This returns if the stack is empty or not
        } else if origin_stack.widget_name() == "stock" {
            dropped_stack.face_down_top_card();
        }
    }

    fn on_double_click(&self, card: &Card) {
        let card_stack = card.get_stack().unwrap();
        if card_stack.widget_name().starts_with("foundation") {
            return
        } else {
            try_distribute(card, &card_stack);
            self.on_drag_completed(&card_stack);
        }
    }

    fn undo_deal(&self, stock: &CardStack) {
        todo!()
    }

    fn on_slot_click(&self, slot: &CardStack) {
        if slot.widget_name() == "stock" {
            let grid = runtime::get_grid().unwrap();
            let waste = runtime::get_stack("waste").unwrap();

            if slot.is_empty() {
                for _i in 0..waste.observe_children().n_items() {
                    let card = waste.last_card().unwrap();
                    waste.remove_card(&card);
                    slot.add_card(&card);
                    card.flip();
                }
                runtime::add_to_history(runtime::Move { origin_stack: "waste".to_string(), 
                                                               card_name: slot.first_card().unwrap().widget_name().to_string(),
                                                               destination_stack: slot.widget_name().to_string(),
                                                               instruction: Some("flip".to_string()) });
            } else {
                let waste = runtime::get_stack("waste").unwrap();
                let card = slot.last_card().unwrap();
                slot.remove_card(&card);
                card.flip();
                waste.add_card(&card);
                waste.add_drag_to_card(&card);
                runtime::add_to_history(runtime::Move { origin_stack: slot.widget_name().to_string(), 
                                                               card_name: card.widget_name().to_string(), 
                                                               destination_stack: "waste".to_string(),
                                                               instruction: None });
            }
        }
    }

    fn is_won(&self) -> bool {
        let grid = runtime::get_grid().unwrap();
        for i in 0..4 {
            let stack = runtime::get_stack(format!("foundation_{i}").as_str()).unwrap();
            if let Some(last_card) = stack.last_card() {
                if !last_card.widget_name().ends_with("king") {
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

fn try_distribute(card: &Card, parent: &CardStack) {
    if !card.imp().is_face_up.get() { return }
    if &parent.last_card().unwrap() != card { return }

    let grid = runtime::get_grid().unwrap();
    for i in 0..4 {
        let stack = runtime::get_stack(format!("foundation_{i}").as_str()).unwrap();
        if let Some(last_card) = stack.last_card() {
            if last_card.is_same_suit(card) && card.is_one_rank_above(&last_card) {
                parent.remove_card(card);
                stack.add_card(card);
                runtime::add_to_history(runtime::Move { origin_stack: parent.widget_name().to_string(),
                                                               card_name: card.widget_name().to_string(), 
                                                               destination_stack: stack.widget_name().to_string(), 
                                                               instruction: None });
                return
            }
        } else {
            if card.get_rank() == "ace" {
                parent.remove_card(card);
                stack.add_card(card);
                runtime::add_to_history(runtime::Move { origin_stack: parent.widget_name().to_string(),
                                                               card_name: card.widget_name().to_string(),
                                                               destination_stack: stack.widget_name().to_string(),
                                                               instruction: None });
                return
            }
        }
    }
}