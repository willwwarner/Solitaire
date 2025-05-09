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

use gtk::prelude::*;
use crate::card_stack::CardStack;
pub const JOKERS: [&str; 2] = ["joker_red", "joker_black"];
pub const SUITES: [&str; 4] = ["club", "diamond", "heart", "spade"];
pub const RANKS: [&str; 13] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king"]; // We use 1 instead of ace for AisleRiot compat
pub const GAMES: [&str; 3] = ["Klondike", "Spider", "Freecell"];

// Links to all the included games
pub fn load_game(game: &str, grid: &gtk::Grid) {
    // Get children from the grid
    let children = grid.observe_children(); // Observes all children currently present in the grid
    
    for i in 0..13 {
        // Create a new card stack for this iteration
        let card_stack = CardStack::new();
        let row = i / 7; // Calculate which row this stack belongs to
        let col = i % 7; // Calculate which column this stack belongs to
        
        for j in 0..4 {
            // Safely fetch and validate individual child widgets
            if let Some(child) = children.item(j) {
                // Attempt the downcast and handle types that are not `gtk::Image`
                let child_type = child.type_();
                if let Ok(image) = child.downcast::<gtk::Image>() {
                    // Successfully downcasted child to gtk::Image
                    grid.remove(&image); // Remove from grid
                    card_stack.add_card(&image); // Add to card stack
                } else {
                    // Log an error if the child isn't of type gtk::Image
                    eprintln!(
                        "Warning: Child at index {} is not a gtk::Image; skipping. Found type: {:?}",
                        j,
                        child_type
                    );
                }
            } else {
                // Log an error if no child was found at the given index
                eprintln!("Warning: No child found at index {} in the grid; skipping.", j);
            }
        }

        // Attach the card stack to the grid at the calculated position
        grid.attach(&card_stack, col, row, 1, 1);
    }
}

pub fn load_recent() {

}