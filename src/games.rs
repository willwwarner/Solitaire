/* games.rs
 *
 * Copyright 2025 Shbozz
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
use gtk::prelude::*;
use gtk::{DragSource, gdk::DragAction, gdk, glib};
use crate::card_stack::CardStack;
use crate::renderer;
use crate::window::*;

pub const JOKERS: [&str; 2] = ["joker_red", "joker_black"];
pub const SUITES: [&str; 4] = ["club", "diamond", "heart", "spade"];
pub const RANKS: [&str; 13] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king"]; // We use 1 instead of ace for AisleRiot compat
pub const GAMES: [&str; 3] = ["Klondike", "Spider", "Freecell"];
static CURRENT_GAME: Mutex<String> = Mutex::new(String::new());

// Links to all the included games
pub fn load_game(game: &str, grid: &gtk::Grid) {
    
    // Get children from the grid
    let children = grid.observe_children();

    for i in 0..14 {
        // Create a new card stack for this position
        let card_stack = CardStack::new();
        card_stack.set_vexpand(true);

        // Calculate layout position
        let row = i / 7;
        let col = i % 7;

        // Add cards to the stack, reusing available images
        for _j in 0..4 {
            // Always get the first item from the collection (index 0)
            // as the collection shifts when items are removed
            if let Some(obj) = children.item(0) {
                if let Ok(image) = obj.downcast::<gtk::Image>() {
                    grid.remove(&image);
                    add_drag_to_card(&image);
                    card_stack.add_card(&image, 50);
                }
            } else {
                gtk::glib::g_error!("Failed to get child from grid", "Solitaire");
            }
        }

        // Enable drag and drop for gameplay
        card_stack.enable_drop();

        // Attach the card stack to the grid at the calculated position
        grid.attach(&card_stack, col, row, 1, 1);
    }

    // Store the current game type
    CURRENT_GAME.lock().unwrap().push_str(game);

    // Setup resize handler for responsive layout
    renderer::setup_resize(grid);

    // Log game loading
    println!("Loaded game: {}", game);
}

pub fn unload(grid: &gtk::Grid) {
    CURRENT_GAME.lock().unwrap().clear();
    renderer::unregister_resize(grid);
}

pub fn load_recent() {

}

pub fn get_current_game() -> String {
    CURRENT_GAME.lock().unwrap().clone()
}

pub fn add_drag_to_card(card: &gtk::Image) {
    let card_clone = card.clone();
    let drag_source = DragSource::builder()
        .actions(DragAction::MOVE)  // allow moving the stack
        .build();

    // When a drag is about to start, supply the CardStack as the drag data:
    drag_source.connect_prepare(move |src, _x, _y| {
        // Convert the CardStack (a GObject) into a GValue, then a ContentProvider.
        let stack = card_clone.parent().unwrap().downcast::<CardStack>().unwrap();
        let move_stack = stack.split_to_new_on(&*card_clone.widget_name()).unwrap();
        let paintable = gtk::WidgetPaintable::new(Some(&move_stack));
        src.set_icon(Some(&paintable), 0, 0);
        let value = move_stack.upcast::<glib::Object>().to_value();
        let provider = gdk::ContentProvider::for_value(&value);
        src.set_content(Some(&provider));  // attach the data provider
        Some(provider)  // must return Some(provider) from prepare
    });
    
    card.add_controller(drag_source);
}