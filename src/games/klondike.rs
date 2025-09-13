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

use std::collections::{HashMap};
use crate::{renderer, runtime, card_stack::CardStack};
use gtk::prelude::{Cast, GridExt, WidgetExt, ListModelExt};
use gtk::glib;

pub struct Klondike {}

impl super::Game for Klondike {
    fn new_game(cards: gtk::gio::ListModel, grid: &gtk::Grid, renderer: &rsvg::CairoRenderer) -> Self {
        let mut n_cards = cards.n_items() as i32;

        for i in 0..7 {
            let card_stack = CardStack::new();
            card_stack.set_widget_name(format!("tableau_{i}").as_str());
            card_stack.set_aspect(2.8); // 2 card heights

            for j in 0..(i + 1) {
                if let Some(obj) = cards.item(glib::random_int_range(0, n_cards) as u32) {
                    if let Ok(picture) = obj.downcast::<gtk::Picture>() {
                        grid.remove(&picture);
                        card_stack.add_card(&picture);
                        if j < i { renderer::flip_card_full(&picture, &renderer) }
                        card_stack.add_drag_to_card(&picture);
                        runtime::connect_double_click(&picture);
                    }
                } else {
                    glib::g_error!("solitaire", "Failed to get child from grid");
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
            if let Some(obj) = cards.item(glib::random_int_range(0, n_cards) as u32) {
                if let Ok(picture) = obj.downcast::<gtk::Picture>() {
                    grid.remove(&picture);
                    stock.add_card(&picture);
                    renderer::flip_card_full(&picture, &renderer);
                    runtime::connect_double_click(&picture);
                }
            } else {
                glib::g_error!("solitaire", "Failed to get child from grid");
            }
            n_cards -= 1;
        }
        grid.attach(&stock, 0, 0, 1, 1);

        Self {}
    }
    fn verify_drag(&self, bottom_card: &gtk::Widget, _from_stack: &CardStack) -> bool {
        if bottom_card.widget_name().ends_with("_b") { false }
        else { true }
    }

    fn verify_drop(&self, bottom_card: &gtk::Widget, to_stack: &CardStack) -> bool {
        let stack_name = to_stack.widget_name();
        let bottom_card_name = bottom_card.widget_name();

        if stack_name.starts_with("tableau") {
            if to_stack.first_child().is_none() && bottom_card_name.ends_with("king") { return true }
            else if to_stack.first_child().is_none() { return false }
            let top_card_name = to_stack.last_child().unwrap().widget_name();
            if (!runtime::is_similar_suit(&bottom_card_name, &top_card_name)) && runtime::is_one_rank_above(&bottom_card_name, &top_card_name) { return true }
            else { false }
        }
        else if stack_name.starts_with("foundation") {
            if to_stack.first_child().is_none() && bottom_card_name.ends_with("ace") { return true }
            else if to_stack.first_child().is_none() { return false }
            let top_card_name = to_stack.last_child().unwrap().widget_name();
            if runtime::is_same_suit (&bottom_card_name, &top_card_name) && runtime::is_one_rank_above(&top_card_name, &bottom_card_name) { return true }
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

    fn on_double_click(&self, card: &gtk::Picture) {
        let card_stack = card.parent().unwrap().downcast::<CardStack>().unwrap();
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
            let grid = slot.parent().unwrap().downcast::<gtk::Grid>().unwrap();
            let waste = runtime::get_child(&grid, "waste").unwrap().downcast::<CardStack>().unwrap();

            if slot.first_child().is_none() {
                if runtime::get_n_moves_from_stack("flip->waste") >= 3 { return }
                for _i in 0..waste.observe_children().n_items() {
                    let card = waste.last_child().unwrap().downcast::<gtk::Picture>().unwrap();
                    waste.remove_card(&card);
                    slot.add_card(&card);
                    renderer::flip_card(&card);
                }
                runtime::add_to_history("flip->waste", slot.first_child().unwrap().widget_name().as_str(), slot.widget_name().as_str());
            } else {
                let waste = runtime::get_child(&grid, "waste").unwrap().downcast::<CardStack>().unwrap();
                let card = slot.last_child().unwrap().downcast::<gtk::Picture>().unwrap();
                slot.remove_card(&card);
                renderer::flip_card(&card);
                waste.add_card(&card);
                waste.add_drag_to_card(&card);
                runtime::add_to_history(slot.widget_name().as_str(), card.widget_name().as_str(), "waste");
            }
        }
    }

    fn get_automoves_ranked(&self, state: &HashMap<glib::GString, Vec<glib::GString>>) -> Vec<String> {
        let mut moves = Vec::new();
        // Check if inner-tableau moves are possible
        for i in 0..7 {
            let stack = state.get(format!("tableau_{i}").as_str()).unwrap();
            for card in stack {
                if !card.ends_with("_b") {
                    for j in 0..7 {
                        if j != i {
                            if let Some(last_child) = state.get(format!("tableau_{j}").as_str()).unwrap().last() {
                                if runtime::is_one_rank_above(&last_child, &card) && runtime::is_similar_suit(&card, &last_child) {
                                    moves.push(format!("tableau_{i}&>{card}&>tableau_{j}"));
                                }
                            } else if card.ends_with("king") {
                                moves.push(format!("tableau_{i}&>{card}&>tableau_{j}"));
                            }
                        }
                    }
                }
            }
        }
        // Check if tableau to foundation moves are possible
        for i in 0..7 {
            if let Some(tableau_child) = state.get(format!("tableau_{i}").as_str()).unwrap().last() {
                for j in 0..4 {
                    let stack = state.get(format!("foundation_{j}").as_str()).unwrap();
                    if let Some(foundation_child) = stack.last() {
                        if runtime::is_same_suit(&foundation_child, &tableau_child) && runtime::is_one_rank_above(&foundation_child, &tableau_child) {
                            moves.push(format!("tableau_{i}&>{tableau_child}&>foundation_{j}"));
                        }
                    } else {
                        if tableau_child.ends_with("ace") {
                            moves.push(format!("tableau_{i}&>{tableau_child}&>foundation_{j}"));
                        }
                    }
                }
            }
        }
        //if moves.len() > 3 { return moves }
        // Check if the waste is empty, and handle it
        let waste = state.get("waste").unwrap();
        let stock = state.get("stock").unwrap();
        if waste.is_empty() {
            if stock.is_empty() { return moves }
            else {
                moves.push(format!("flip->stock&>{}&>waste", stock.last().unwrap()));
                return moves;
            }
        }
        // Check if a waste to tableau move is possible
        let card = waste.last().unwrap();
        for i in 0..7 {
            if let Some(tableau_child) = state.get(format!("tableau_{i}").as_str()).unwrap().last() {
                if runtime::is_similar_suit(card, tableau_child) && runtime::is_one_rank_above(tableau_child, card) {
                    moves.push(format!("waste&>{card}&>tableau_{i}"));
                }
            }
        }
        // Check if a waste to foundation move is possible
        let card = waste.last().unwrap();
        for i in 0..4 {
            if let Some(foundation_child) = state.get(format!("foundation_{i}").as_str()).unwrap().last() {
                if runtime::is_same_suit(card, foundation_child) && runtime::is_one_rank_above(foundation_child, card) {
                    moves.push(format!("waste&>{card}&>foundation_{i}"));
                }
            }
        }

        if stock.is_empty() {
            moves.push(format!("flip->waste&>{}&>stock", waste.first().unwrap()));
        } else {
            moves.push(format!("flip->stock&>{}&>waste", stock.last().unwrap()));
        }

        // Check if a foundation to tableau move is possible
        for i in 0..4 {
            let foundation = state.get(format!("foundation_{i}").as_str()).unwrap();
            if let Some(card) = foundation.last() {
                for j in 0..7 {
                    if let Some(tableau_child) = state.get(format!("tableau_{j}").as_str()).unwrap().last() {
                        if runtime::is_similar_suit(card, tableau_child) && runtime::is_one_rank_above(tableau_child, card) {
                            moves.push(format!("foundation_{i}&>{card}&>tableau_{j}"));
                        }
                    }
                }
            }
        }
        moves
    }

    fn get_priority(&self, state: &HashMap<glib::GString, Vec<glib::GString>>) -> u32 {
        let mut outs = 0;
        for i in 0..4 {
            let outpile = state.get(format!("foundation_{i}").as_str()).unwrap();
            outs += outpile.len() as u32;
        }
        for i in 0..7 {
            let tableau = state.get(format!("tableau_{i}").as_str()).unwrap();
            if let Some(last_child) = tableau.last() { if !last_child.ends_with("_b") { outs += 1; } }
        }
        outs
    }

    fn is_won(&self, state: &HashMap<glib::GString, Vec<glib::GString>>) -> bool {
        for i in 0..4 {
            let stack = state.get(format!("foundation_{i}").as_str()).unwrap();
            if let Some(last_child) = stack.last() {
                if !last_child.ends_with("king") {
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

fn try_distribute(card: &gtk::Picture, parent: &CardStack) {
    let card_name = card.widget_name();
    if card_name.ends_with("_b") { return }
    if &parent.last_child().unwrap() != card { return }

    let grid = runtime::get_grid().unwrap();
    let card_suit = card_name.split("_").nth(0).unwrap();
    for i in 0..4 {
        let stack = runtime::get_child(&grid, format!("foundation_{i}").as_str()).unwrap().downcast::<CardStack>().unwrap();
        if let Some(last_child) = stack.last_child() {
            if last_child.widget_name().starts_with(card_suit) && runtime::is_one_rank_above(&last_child.widget_name(), &card_name) {
                parent.remove_card(card);
                stack.add_card(card);
                runtime::add_to_history(parent.widget_name().as_str(), card.widget_name().as_str(), stack.widget_name().as_str());
                return
            }
        } else {
            if card_name.ends_with("ace") {
                parent.remove_card(card);
                stack.add_card(card);
                runtime::add_to_history(parent.widget_name().as_str(), card.widget_name().as_str(), stack.widget_name().as_str());
                return
            }
        }
    }
}