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

pub const JOKERS: [&str; 2] = ["joker_red", "joker_black"];
pub const SUITES: [&str; 4] = ["club", "diamond", "heart", "spade"];
pub const RANKS: [&str; 13] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king"]; // We use 1 instead of ace for AisleRiot compat
pub const GAMES: [&str; 3] = ["Klondike", "Spider", "Freecell"];

// Links to all the included games
pub fn load_game(game: &str) {

}

pub fn load_recent() {

}
