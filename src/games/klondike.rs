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

use crate::{renderer, runtime, card_stack::CardStack, card_stack};
use gtk::prelude::{Cast, GridExt, WidgetExt, ListModelExt, BoxExt};
use gtk::glib;

pub struct Klondike;

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
                        else {
                            card_stack.add_drag_to_card_full(&picture, card_stack::DragCompletedAction::FlipTopCard);
                            runtime::connect_click(&picture, || distribute());
                        }
                    }
                } else {
                    glib::g_error!("solitaire", "Failed to get child from grid");
                }
                n_cards -= 1;
            }

            grid.attach(&card_stack, i, 1, 1, 1);

            // Card Stacks must have no layout manager to work correctly
            card_stack.set_layout_manager(None::<gtk::LayoutManager>);
            card_stack.set_vexpand(true);
            card_stack.enable_drop();
        }

        for i in 3..7 {
            let card_stack = CardStack::new();
            card_stack.set_widget_name(format!("foundation_{i}").as_str());
            card_stack.set_fan_cards(false);
            grid.attach(&card_stack, i, 0, 1, 1);
            card_stack.set_layout_manager(None::<gtk::LayoutManager>);
            card_stack.enable_drop();
        }

        let waste = CardStack::new();
        waste.set_widget_name("waste");
        waste.set_fan_cards(false);
        grid.attach(&waste, 1, 0, 1, 1);
        waste.set_layout_manager(None::<gtk::LayoutManager>); // Card Stacks must have no layout manager to work correctly

        let stock = CardStack::new();
        stock.set_widget_name("stock");
        stock.set_fan_cards(false);
        while n_cards > 0 {
            if let Some(obj) = cards.item(glib::random_int_range(0, n_cards) as u32) {
                if let Ok(picture) = obj.downcast::<gtk::Picture>() {
                    grid.remove(&picture);
                    stock.add_card(&picture);
                    renderer::flip_card_full(&picture, &renderer);
                    let card_clone = picture.clone();
                    runtime::connect_click(&picture, move || {
                        let stock = card_clone.parent().unwrap().downcast::<CardStack>().unwrap();
                        stock.remove(&card_clone);
                        let waste = runtime::get_child(&stock.parent().unwrap(), "waste").unwrap().downcast::<CardStack>().unwrap();
                        waste.add_card(&card_clone);
                        renderer::flip_card(&card_clone);
                        waste.add_drag_to_card(&card_clone);
                        runtime::remove_click(&card_clone); // TODO: one click to distribute card
                    });
                }
            } else {
                glib::g_error!("solitaire", "Failed to get child from grid");
            }
            n_cards -= 1;
        }
        grid.attach(&stock, 0, 0, 1, 1);
        stock.set_layout_manager(None::<gtk::LayoutManager>); // Card Stacks must have no layout manager to work correctly
        
        Self
    }
}

fn distribute() {
    // TODO: Implement distribute logic
    // Uncomment when current_name is available
    // if current_name.contains("_b") {
    //
    // }
}