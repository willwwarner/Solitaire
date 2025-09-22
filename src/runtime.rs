/* runtime.rs
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

use gtk::{DragSource, GestureClick, gio, glib};
use gtk::prelude::{IsA, ListModelExt, WidgetExt, Cast, ActionMapExt};
use crate::{games, card::Card, card_stack::CardStack};

#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    pub origin_stack: String,
    pub card_name: String,
    pub destination_stack: String,
    pub instruction: Option<String>,
}

thread_local! {
    static GRID: std::cell::RefCell<Option<gtk::Grid>> = std::cell::RefCell::new(None);
    static ACTION_HISTORY: std::cell::RefCell<Vec<Move>> = std::cell::RefCell::new(Vec::new());
    static HISTORY_INDEX: std::cell::RefCell<usize> = std::cell::RefCell::new(0);
    static N_DEALS: std::cell::RefCell<u8> = std::cell::RefCell::new(0);
    static CARDS: std::cell::RefCell<Vec<Card>> = std::cell::RefCell::new(Vec::new());
}

pub fn remove_drag(widget: &impl IsA<gtk::Widget>) {
    let controllers = widget.observe_controllers();
    for item in &controllers {
        let controller = item.unwrap();
        if let Ok(drag) = controller.downcast::<DragSource>() {
            widget.remove_controller(&drag);
        }
    }
}

pub fn get_child(from: &impl IsA<gtk::Widget>, name: &str) -> Result<gtk::Widget, glib::Error> {
    // Attempt to locate the child with the given card name
    let children = from.observe_children();

    // Loop through all the children widgets to find the matching card
    for i in 0..children.n_items() {
        let child = children.item(i).expect("Failed to get child from a Widget");
        let child_widget = child.downcast::<gtk::Widget>().expect("Failed to downcast child to a Widget");
        if child_widget.widget_name() == name {
            return Ok(child_widget);
        }
    }

    Err(glib::Error::new(glib::FileError::Exist, format!("Card named '{}' was not found in the stack.", name).as_str()))
}

pub fn connect_double_click(card: &Card) {
    let click = GestureClick::new();

    let card_clone = card.to_owned();
    click.connect_pressed(move |_click, n_press, _x, _y| {
        if n_press == 2 {
            games::on_double_click(&card_clone);
        }
    });
    card.add_controller(click);
}

pub fn remove_click(widget: &impl IsA<gtk::Widget>) {
    let controllers = widget.observe_controllers();
    for item in &controllers {
        let controller = item.unwrap();
        if let Ok(click) = controller.downcast::<GestureClick>() {
            widget.remove_controller(&click);
        }
    }
}

pub fn get_grid() -> Option<gtk::Grid> {
    GRID.with(|grid| grid.borrow().clone())
}

pub fn set_grid(grid: gtk::Grid) {
    GRID.set(Some(grid));
}

pub fn perform_move(move_: &Move) {
    let origin_stack = get_stack(&move_.origin_stack).unwrap();
    let destination_stack = get_stack(&move_.destination_stack).unwrap();
    perform_move_with_stacks(move_, &origin_stack, &destination_stack);
}

pub fn perform_move_with_stacks(move_: &Move, origin_stack: &CardStack, destination_stack: &CardStack) {
    if let Some(instruction) = &move_.instruction {
        match instruction.as_str() {
            "flip" => { //Fixme
                let origin_children = origin_stack.observe_children();
                for _ in 0..origin_children.n_items() {
                    let card = origin_stack.last_child().unwrap().downcast::<Card>().unwrap();
                    card.flip();
                    card.unparent();
                    destination_stack.add_card(&card);
                }
                return
            },
            _ => println!("Unknown instruction: {}", instruction),
        }
    }
    let transfer_stack = origin_stack.split_to_new_on(&*move_.card_name);
    destination_stack.merge_stack(&transfer_stack);
}

pub fn add_to_history(move_: Move) {
    let move_index = HISTORY_INDEX.with(|index| index.borrow().clone());
    // Remove invalidated entries
    if ACTION_HISTORY.with(|history| history.borrow().len() > move_index) {
        for _ in move_index..ACTION_HISTORY.with(|history| history.borrow().len()) {
            ACTION_HISTORY.with(|history| history.borrow_mut().pop());
            let window = get_grid().unwrap().root().unwrap().downcast::<gtk::Window>().unwrap().downcast::<crate::window::SolitaireWindow>().unwrap();
            window.lookup_action("redo").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(false);
        }
    }
    if move_index == 0 {
        let window = get_grid().unwrap().root().unwrap().downcast::<gtk::Window>().unwrap().downcast::<crate::window::SolitaireWindow>().unwrap();
        window.lookup_action("undo").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(true);
    }
    ACTION_HISTORY.with(|history| history.borrow_mut().push(move_));
    HISTORY_INDEX.set(move_index + 1);
}

pub fn undo_last_move() {
    let move_index = HISTORY_INDEX.with(|index| index.borrow().clone());
    if !(move_index == 0) {
        let last_entry = ACTION_HISTORY.with(|history| history.borrow().get(move_index - 1).cloned()).unwrap();
        let grid = get_grid().unwrap();
        let destination_stack = get_child(&grid, &last_entry.origin_stack).unwrap().downcast::<CardStack>().unwrap();
        let origin_stack = get_child(&grid, &last_entry.destination_stack).unwrap().downcast::<CardStack>().unwrap();
        games::pre_undo_drag(&destination_stack, &origin_stack);
        perform_move_with_stacks(&last_entry, &origin_stack, &destination_stack);
        HISTORY_INDEX.set(move_index - 1);
    }
}

pub fn redo_first_move() {
    let move_index = HISTORY_INDEX.with(|index| index.borrow().clone());
    if let Some(first_entry) = ACTION_HISTORY.with(|history| history.borrow().get(move_index).cloned()) {
        let grid = get_grid().unwrap();
        let origin_stack = get_child(&grid, &first_entry.origin_stack).unwrap().downcast::<CardStack>().unwrap();
        let destination_stack = get_child(&grid, &first_entry.destination_stack).unwrap().downcast::<CardStack>().unwrap();
        perform_move(&first_entry);
        games::on_drag_completed(&origin_stack);
        games::on_drop_completed(&destination_stack);
        HISTORY_INDEX.set(move_index + 1);
    }
}

pub fn update_redo_actions(window: &crate::window::SolitaireWindow) {
    let move_index = HISTORY_INDEX.with(|index| index.borrow().clone());
    let undo_action = window.lookup_action("undo").unwrap().downcast::<gio::SimpleAction>().unwrap();
    let redo_action = window.lookup_action("redo").unwrap().downcast::<gio::SimpleAction>().unwrap();
    undo_action.set_enabled(move_index > 0);
    redo_action.set_enabled(move_index < ACTION_HISTORY.with(|history| history.borrow().len()));
}

pub fn clear_history_and_moves() {
    ACTION_HISTORY.set(Vec::new());
    HISTORY_INDEX.set(0);
}

pub fn get_stack(name: &str) -> Option<CardStack> {
    let grid = get_grid().unwrap();
    if let Ok(widget) = get_child(&grid, name) {
        if let Ok(stack) = widget.downcast::<CardStack>() {
            return Some(stack);
        }
    }
    None
}

pub fn set_cards(cards: Vec<Card>) {
    CARDS.set(cards);
}

pub fn get_cards() -> Vec<Card> {
    CARDS.with(|cards| cards.borrow().to_owned())
}

pub fn get_deals() -> u8 {
    N_DEALS.with(|n| n.borrow().to_owned())
}

pub fn update_deals(n: u8) {
    N_DEALS.set(n);
}