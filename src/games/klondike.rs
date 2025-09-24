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

use crate::{games::*, runtime, card::Card, card_stack::CardStack};
use gtk::{glib, subclass::prelude::*};

pub struct Klondike {}

impl Klondike {}
impl Game for Klondike {
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

    fn on_slot_click(&self, slot: &CardStack) {
        if slot.widget_name() == "stock" {
            let waste = runtime::get_stack("waste").unwrap();

            if slot.is_empty() {
                let n_deals = runtime::get_deals();
                if n_deals >= 3 { return }
                runtime::update_deals(n_deals + 1);
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

    fn get_solver_moves_ranked(&self, state: &IndexMap<String, Vec<u8>>) -> Vec<(SolverMove, usize)> {
        let mut moves = Vec::new();
        // Check if inner-tableau moves are possible
        for i in 0..7 {
            let stack = state.get(&format!("tableau_{i}")).unwrap();
            for card in stack {
                if !is_flipped(card) {
                    for j in 0..7 {
                        if j != i {
                            if let Some(last_child) = state.get(&format!("tableau_{j}")).unwrap().last() {
                                if is_one_rank_above(&last_child, &card) && is_similar_suit(&card, &last_child) {
                                    moves.push((move_from_strings(format!("tableau_{i}"), card,format!("tableau_{j}"), None), 4));
                                }
                            } else if get_rank(card) == "king" {
                                moves.push((move_from_strings(format!("tableau_{i}"), card,format!("tableau_{j}"), None), 4));
                            }
                        }
                    }
                }
            }
        }
        // Check if tableau to foundation moves are possible
        for i in 0..7 {
            if let Some(tableau_child) = state.get(&format!("tableau_{i}")).unwrap().last() {
                if is_flipped(&tableau_child) { continue }
                for j in 0..4 {
                    let stack = state.get(&format!("foundation_{j}")).unwrap();
                    if let Some(foundation_child) = stack.last() {
                        if is_same_suit(&foundation_child, &tableau_child) && is_one_rank_above(&foundation_child, &tableau_child) {
                            moves.push((move_from_strings(format!("tableau_{i}"), tableau_child,format!("foundation_{j}"), None), 3));
                        }
                    } else {
                        if get_rank(tableau_child) == "ace" {
                            moves.push((move_from_strings(format!("tableau_{i}"), tableau_child,format!("foundation_{j}"), None), 3));
                        }
                    }
                }
            }
        }
        // Check if a foundation to tableau move is possible
        for i in 0..4 {
            let foundation = state.get(&format!("foundation_{i}")).unwrap();
            if let Some(card) = foundation.last() {
                for j in 0..7 {
                    if let Some(tableau_child) = state.get(&format!("tableau_{j}")).unwrap().last() {
                        if is_similar_suit(card, tableau_child) && is_one_rank_above(tableau_child, card) && !is_flipped(tableau_child) {
                            moves.push((move_from_strings(format!("foundation_{i}"), card,format!("tableau_{j}"), None), 2));
                        }
                    }
                }
            }
        }
        // Check if the waste is empty, and handle it
        let waste = state.get("waste").unwrap();
        let stock = state.get("stock").unwrap();
        if waste.is_empty() {
            if stock.is_empty() { return moves }
            else {
                moves.push((create_move("stock", stock.last().unwrap(), "waste", Some("flip")), 2));
                return moves;
            }
        }
        // Check if a waste to tableau move is possible
        let card = waste.last().unwrap();
        for i in 0..7 {
            if let Some(tableau_child) = state.get(&format!("tableau_{i}")).unwrap().last() {
                if is_similar_suit(card, tableau_child) && is_one_rank_above(tableau_child, card) {
                    moves.push((create_move("waste", card, format!("tableau_{i}").as_str(), None), 2));

                }
            } else if get_rank(card) == "king" {
                moves.push((create_move("waste", card,format!("tableau_{i}").as_str(), None), 4));
            }
        }
        // Check if a waste to foundation move is possible
        let card = waste.last().unwrap();
        for i in 0..4 {
            if let Some(foundation_child) = state.get(&format!("foundation_{i}")).unwrap().last() {
                if is_same_suit(card, foundation_child) && is_one_rank_above(foundation_child, card) {
                    moves.push((create_move("waste", card, format!("foundation_{i}").as_str(), None), 2));
                }
            } else {
                if get_rank(card) == "ace" {
                    moves.push((create_move("waste", card,format!("foundation_{i}").as_str(), None), 2));
                }
            }
        }

        if stock.is_empty() {
            moves.push((create_move("waste", waste.first().unwrap(),"stock", Some("flip")), 1));
        } else {
            moves.push((create_move("stock", stock.last().unwrap(), "waste", Some("flip")), 1));
        }
        moves
    }

    fn solver_on_move(&self, move_option: &SolverMove, state: &mut IndexMap<String, Vec<u8>>, undo: bool) {
        if move_option.origin_stack.starts_with("tableau") {
            let origin_stack = state.get_mut(&move_option.origin_stack).unwrap();
            if let Some(card_index) = origin_stack.iter().position(|x| *x == move_option.card) {
                flip(origin_stack.get_mut(card_index).unwrap());
            }
        }
    }

    fn get_priority(&self, state: &IndexMap<String, Vec<u8>>) -> u32 {
        let mut outs = 0;
        for i in 0..4 {
            let outpile = state.get(&format!("foundation_{i}")).unwrap();
            outs += outpile.len() as u32;
        }
        outs
    }

    fn is_won(&self, state: &IndexMap<String, Vec<u8>>) -> bool {
        for i in 0..4 {
            let stack = state.get(&format!("foundation_{i}")).unwrap();
            if let Some(last_child) = stack.last() {
                if !(get_rank(last_child) == "king") {
                    return false;
                }
            } else {
                // If one of the foundations is empty, the game is not won
                return false;
            }
        }
        true
    }
}

fn try_distribute(card: &Card, parent: &CardStack) {
    if !card.imp().is_face_up.get() { return }
    if &parent.last_card().unwrap() != card { return }

    for i in 0..4 {
        let stack = runtime::get_stack(format!("foundation_{i}").as_str()).unwrap();
        if let Some(last_card) = stack.last_card() {
            if last_card.is_same_suit(card) && card.is_one_rank_above(&last_card) {
                parent.remove_card(card);
                stack.add_card(card);
                runtime::add_to_history(runtime::create_move(&parent.widget_name(), &card.widget_name(), &stack.widget_name(), None));
                return
            }
        } else {
            if card.get_rank() == "ace" {
                parent.remove_card(card);
                stack.add_card(card);
                runtime::add_to_history(runtime::create_move(&parent.widget_name(), &card.widget_name(), &stack.widget_name(), None));
                return
            }
        }
    }
}