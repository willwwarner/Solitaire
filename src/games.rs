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

use std::sync::Mutex;
use std::format;
use gtk::prelude::*;
use gtk::{gio, glib};
use gettextrs::gettext;
use crate::{renderer, card_stack::CardStack, runtime};

mod klondike;

pub const JOKERS: [&str; 2] = ["joker_red", "joker_black"];
pub const SUITES: [&str; 4] = ["club", "diamond", "heart", "spade"];
pub const RANKS: [&str; 13] = ["ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king"];
static CURRENT_GAME: Mutex<Option<Box<dyn Game>>> = Mutex::new(None);

pub fn load_game(game_name: &str, grid: &gtk::Grid) {
    let window = grid.root().unwrap().downcast::<gtk::Window>().unwrap().downcast::<crate::window::SolitaireWindow>().unwrap();
    window.lookup_action("undo").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(false);
    window.lookup_action("redo").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(false);
    window.lookup_action("hint").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(false);

    let cards = grid.observe_children();

    // Create the renderer for the game
    glib::g_message!("solitaire", "Loading SVG");
    let resource = gio::resources_lookup_data("/org/gnome/gitlab/wwarner/Solitaire/assets/anglo_poker.svg", gio::ResourceLookupFlags::NONE)
        .expect("Failed to load resource data");
    glib::g_message!("solitaire", "loaded resource data");
    let handle = rsvg::Loader::new()
        .read_stream(&gio::MemoryInputStream::from_bytes(&resource), None::<&gio::File>, None::<&gio::Cancellable>)
        .expect("Failed to load SVG");
    let renderer = rsvg::CairoRenderer::new(&handle);
    glib::g_message!("solitaire", "Done Loading SVG");

    for i in 0..grid.observe_children().n_items() {
        let picture = cards.item(i).unwrap().downcast::<gtk::Picture>().unwrap();

        let suite_index = (i / 13) as usize;
        let rank_index = (i % 13) as usize;
        let card_name = format!("{}_{}", SUITES[suite_index], RANKS[rank_index]);

        picture.set_widget_name(card_name.as_str());
        picture.set_property("sensitive", true);
        let texture = renderer::set_and_return_texture(&card_name, &renderer);
        picture.set_paintable(Some(&texture));
    }

    renderer::set_back_texture(&renderer);
    glib::g_message!("solitaire", "Done setting textures");

    // Store the current game type
    let mut game = CURRENT_GAME.lock().unwrap();
    *game = Some(Box::new(klondike::Klondike::new_game(cards, &grid, &renderer)));

    // Log game loading
    println!("Loaded game: {}", game_name);
}

pub fn unload(grid: &gtk::Grid) {
    let mut game = CURRENT_GAME.lock().unwrap();
    *game = None;
    runtime::clear_history_and_moves();
    let items = grid.observe_children().n_items();
    for i in 0..items {
        let child = grid.first_child().expect("Couldn't get child");
        let stack = child.downcast::<CardStack>().expect("Couldn't downcast child");
        stack.remove_child_controllers();
        stack.dissolve_to_row(&grid, i as i32 + 100);
    }
}

pub fn get_games() -> Vec<String> {
    vec![gettext("Klondike")] //, "Spider", "FreeCell", "Tri-Peaks", "Pyramid", "Yukon"]; not yet :)
}

pub fn on_double_click(card: &gtk::Picture) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_double_click(card);
    }
}

pub fn on_slot_click(slot: &CardStack) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_slot_click(slot);
    }
}

pub fn on_drag_completed(origin_stack: &CardStack) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_drag_completed(origin_stack);
    }
}

pub fn on_drop_completed(recipient_stack: &CardStack) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.on_drop_completed(recipient_stack);
    }
}

pub fn pre_undo_drag(origin_stack: &CardStack, dropped_stack: &CardStack) {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.pre_undo_drag(origin_stack, dropped_stack);
    }
}

pub fn verify_drag(bottom_card: &gtk::Widget, from_stack: &CardStack) -> bool {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.verify_drag(bottom_card, from_stack)
    } else {
        false
    }
}

pub fn verify_drop(bottom_card: &gtk::Widget, to_stack: &CardStack) -> bool {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.verify_drop(bottom_card, to_stack)
    } else {
        false
    }
}

pub fn get_best_next_move() -> Option<(String, String, String)> {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.get_best_next_move()
    } else {
        None
    }
}

pub fn is_winnable() -> bool {
    let mut game = CURRENT_GAME.lock().unwrap();
    if let Some(game) = game.as_mut() {
        game.is_winnable()
    } else {
        false
    }
}

pub trait Game: Send + Sync {
    fn new_game(cards: gio::ListModel, grid: &gtk::Grid, renderer: &rsvg::CairoRenderer) -> Self where Self: Sized;
    fn verify_drag(&self, bottom_card: &gtk::Widget, from_stack: &CardStack) -> bool;
    fn verify_drop(&self, bottom_card: &gtk::Widget, to_stack: &CardStack) -> bool;
    fn on_drag_completed(&self, origin_stack: &CardStack);
    fn on_drop_completed(&self, recipient_stack: &CardStack);
    fn pre_undo_drag(&self, origin_stack: &CardStack, dropped_stack: &CardStack);
    fn on_double_click(&self, card: &gtk::Picture);
    fn undo_deal(&self, stock: &CardStack);
    fn on_slot_click(&self, slot: &CardStack);
    fn is_won(&self) -> bool;
    fn get_best_next_move(&self) -> Option<(String, String, String)>;
    fn is_winnable(&self) -> bool;
}