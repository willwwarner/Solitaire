/* games.rs
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

use std::sync::{Mutex, MutexGuard};
use std::clone::Clone;
use std::format;
use adw::{gio, glib};
use adw::gdk::Paintable;
use gtk::prelude::*;
use gettextrs::gettext;
use gtk::Picture;
use crate::card_stack::CardStack;
use crate::{card_stack, games, renderer, runtime};

mod klondike;

pub const JOKERS: [&str; 2] = ["joker_red", "joker_black"];
pub const SUITES: [&str; 4] = ["club", "diamond", "heart", "spade"];
pub const RANKS: [&str; 13] = ["ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king"];
static CURRENT_GAME: Mutex<Option<Box<dyn Game>>> = Mutex::new(None);

pub fn load_game(game_name: &str, grid: &gtk::Grid) {
    // Get children from the grid
    let cards = grid.observe_children();

    // Create the renderer for the game
    glib::g_message!("solitaire", "Loading SVG");
    let resource = gio::resources_lookup_data("/org/gnome/Solitaire/assets/anglo_poker.svg", gio::ResourceLookupFlags::NONE)
        .expect("Failed to load resource data");
    glib::g_message!("solitaire", "loaded resource data");
    let handle = rsvg::Loader::new()
        .read_stream(&gio::MemoryInputStream::from_bytes(&resource), None::<&gio::File>, None::<&gio::Cancellable>)
        .expect("Failed to load SVG");
    let renderer = rsvg::CairoRenderer::new(&handle);
    glib::g_message!("solitaire", "Done Loading SVG");

    for i in 0..cards.n_items() {
        let picture = cards.item(i).unwrap().downcast::<gtk::Picture>().unwrap();

        let suite_index = (i / 13) as usize;
        let rank_index = (i % 13) as usize;
        let card_name = format!("{}_{}", SUITES[suite_index], RANKS[rank_index]);

        picture.set_widget_name(card_name.as_str());
        picture.set_property("sensitive", true);
        let texture = renderer::draw_card(&card_name, &renderer);
        picture.set_paintable(Some(texture.upcast_ref::<Paintable>()));
    }

    // Store the current game type
    let mut game = CURRENT_GAME.lock().unwrap();
    *game = Some(Box::new(klondike::Klondike::new_game(cards, &grid, &renderer)));

    // Log game loading
    println!("Loaded game: {}", game_name);
}

pub fn unload(grid: &gtk::Grid) {
    let mut game = CURRENT_GAME.lock().unwrap();
    *game = None;
    let items = grid.observe_children().n_items();
    for i in 0..items {
        let child = grid.first_child().expect("Couldn't get child");
        let stack = child.downcast::<CardStack>().expect("Couldn't downcast child");
        stack.remove_child_controllers();
        stack.dissolve_to_row(&grid, i as i32);
    }
}

pub fn get_games() -> Vec<String> {
    vec![gettext("Klondike")] //, "Spider", "FreeCell", "Tri-Peaks", "Pyramid", "Yukon"]; not yet :)
}

pub fn on_card_click(card: &Picture) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_card_click(card);
    }
}

pub fn on_drag_completed(origin_stack: &CardStack) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_drag_completed(origin_stack)
    }
}

pub trait Game: Send + Sync {
    fn new_game(cards: gtk::gio::ListModel, grid: &gtk::Grid, renderer: &rsvg::CairoRenderer) -> Self where Self: Sized;
    fn verify_drag(&self, bottom_card: &gtk::Picture, from_stack: &CardStack) -> bool;
    fn verify_drop(&self, bottom_card: &gtk::Picture, to_stack: &CardStack) -> bool;
    fn on_drag_completed(&self, origin_stack: &CardStack);
    fn on_card_click(&self, card: &Picture);
}