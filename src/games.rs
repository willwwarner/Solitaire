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
use crate::card_stack::CardStack;
use crate::renderer;

pub const JOKERS: [&str; 2] = ["joker_red", "joker_black"];
pub const SUITES: [&str; 4] = ["club", "diamond", "heart", "spade"];
pub const RANKS: [&str; 13] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king"]; // We use 1 instead of ace for AisleRiot compat
pub const GAMES: [&str; 3] = ["Klondike", "Spider", "Freecell"];
static CURRENT_GAME: Mutex<String> = Mutex::new(String::new());

// Links to all the included games
pub fn load_game(game: &str, grid: &gtk::Grid) {
    // Get children from the grid
    let children = grid.observe_children();

    // Game-specific configuration
    let (rows, columns) = match game {
        "klondike" => (2, 7),   // Standard Klondike layout
        "spider" => (2, 10),    // Spider layout with 10 columns
        "freecell" => (2, 8),   // FreeCell layout
        _ => {
            eprintln!("Unknown game type: {}, defaulting to Klondike", game);
            (2, 7)              // Default to Klondike layout
        }
    };

    // Create card stacks based on game type
    let total_stacks = rows * columns;
    for i in 0..total_stacks {
        // Create a new card stack for this position
        let card_stack = CardStack::new();

        // Calculate layout position
        let row = i / columns;
        let col = i % columns;

        // Initial number of cards for this stack depends on game type and position
        let card_count = match game {
            "klondike" => {
                if col < 7 && row == 1 { col + 1 } else { 0 }
            },
            "spider" => {
                if col < 10 && row == 1 {
                    if col < 4 { 6 } else { 5 }
                } else { 0 }
            },
            "freecell" => 0,    // FreeCell starts with no cards in play stacks
            _ => 0
        };

        // Add cards to the stack, reusing available images
        for j in 0..card_count.min(children.n_items() as i32) {
            if let Some(obj) = children.item(j as u32) {
                if let Ok(image) = obj.downcast::<gtk::Image>() {
                    grid.remove(&image);
                    card_stack.add_card(&image, 50);
                }
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


pub fn load_recent() {

}

pub fn get_current_game() -> String {
    CURRENT_GAME.lock().unwrap().clone()
}
