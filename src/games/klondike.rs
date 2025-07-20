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

use std::sync::Mutex;
use crate::{renderer, runtime, card_stack::CardStack};
use gtk::prelude::{Cast, GridExt, WidgetExt, ListModelExt};
use gtk::glib;

pub struct Klondike {
    foundation_heart: Mutex<String>,
    foundation_diamond: Mutex<String>,
    foundation_club: Mutex<String>,
    foundation_spade: Mutex<String>,
}

impl Klondike {}
impl super::Game for Klondike {
    fn new_game(cards: gtk::gio::ListModel, grid: &gtk::Grid, renderer: &rsvg::CairoRenderer) -> Self {
        let mut n_cards = cards.n_items() as i32;

        for i in 0..7 {
            let card_stack = CardStack::new();
            card_stack.set_widget_name(format!("tableau_{i}").as_str());

            for j in 0..(i + 1) {
                if let Some(obj) = cards.item(glib::random_int_range(0, n_cards) as u32) {
                    if let Ok(picture) = obj.downcast::<gtk::Picture>() {
                        grid.remove(&picture);
                        card_stack.add_card(&picture);
                        if j < i { renderer::flip_card_full(&picture, &renderer) }
                        card_stack.add_drag_to_card(&picture);
                        runtime::connect_click(&picture);
                    }
                } else {
                    glib::g_error!("solitaire", "Failed to get child from grid");
                }
                n_cards -= 1;
            }

            grid.attach(&card_stack, i, 1, 1, 2);

            card_stack.enable_drop();
        }

        for i in 3..7 {
            let card_stack = CardStack::new();
            card_stack.set_widget_name(format!("foundation_{i}").as_str());
            card_stack.set_fan_cards(false);
            grid.attach(&card_stack, i, 0, 1, 1);
            card_stack.enable_drop();
        }

        let waste = CardStack::new();
        waste.set_widget_name("waste");
        waste.set_fan_cards(false);
        grid.attach(&waste, 1, 0, 1, 1);

        let stock = CardStack::new();
        stock.set_widget_name("stock");
        stock.set_fan_cards(false);
        while n_cards > 0 {
            if let Some(obj) = cards.item(glib::random_int_range(0, n_cards) as u32) {
                if let Ok(picture) = obj.downcast::<gtk::Picture>() {
                    grid.remove(&picture);
                    stock.add_card(&picture);
                    renderer::flip_card_full(&picture, &renderer);
                    runtime::connect_click(&picture);
                }
            } else {
                glib::g_error!("solitaire", "Failed to get child from grid");
            }
            n_cards -= 1;
        }
        grid.attach(&stock, 0, 0, 1, 1);

        Self { foundation_heart: Mutex::new(String::new()), foundation_diamond: Mutex::new(String::new()),
               foundation_club:  Mutex::new(String::new()), foundation_spade:   Mutex::new(String::new()) }
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
        if recipient_stack.widget_name().starts_with("foundation") {
            let top_card_name = recipient_stack.last_child().unwrap().widget_name();
            if top_card_name.starts_with("heart") {
                let mut heart = self.foundation_heart.lock().unwrap();
                *heart = top_card_name.to_string();
            }
            else if top_card_name.starts_with("diamond") {
                let mut diamond = self.foundation_diamond.lock().unwrap();
                *diamond = top_card_name.to_string();
            }
            else if top_card_name.starts_with("club") {
                let mut club = self.foundation_club.lock().unwrap();
                *club = top_card_name.to_string();
            }
            else if top_card_name.starts_with("spade") {
                let mut spade = self.foundation_spade.lock().unwrap();
                *spade = top_card_name.to_string();
            }
        }
    }

    fn on_card_click(&self, card: &gtk::Picture) {
        let card_stack = card.parent().unwrap().downcast::<CardStack>().unwrap();
        let grid = card_stack.parent().unwrap().downcast::<gtk::Grid>().unwrap();
        if card_stack.widget_name() == "stock" {
            let waste = runtime::get_child(&grid, "waste").unwrap().downcast::<CardStack>().unwrap();
            card_stack.remove_card(card);
            renderer::flip_card(card);
            waste.add_card(card);
            waste.add_drag_to_card(card);
        } else {
            println!("distribution time!!!");
        }
    }
}