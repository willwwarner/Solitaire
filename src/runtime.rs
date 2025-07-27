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
use crate::{games, card_stack::CardStack};

thread_local! {
    static GRID: std::cell::RefCell<Option<gtk::Grid>> = std::cell::RefCell::new(None);
    static ACTION_HISTORY: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    static HISTORY_INDEX: std::cell::RefCell<usize> = std::cell::RefCell::new(0);
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

pub fn get_child(widget: &impl IsA<gtk::Widget>, name: &str) -> Result<gtk::Widget, glib::Error> {
    // Attempt to locate the child with the given card name
    let children = widget.observe_children();

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

pub fn connect_click(picture: &gtk::Picture) {
    let click = GestureClick::new();

    let picture_clone = picture.to_owned();
    click.connect_pressed(move |_click, _n_press, _x, _y| {
        games::on_card_click(&picture_clone);
    });
    picture.add_controller(click);
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

pub fn perform_move(destination_stack:&CardStack, card_name: &str, origin_stack: &CardStack) {
    let transfer_stack = origin_stack.try_split_to_new_on(card_name).unwrap_or_else(|_| origin_stack.split_to_new_on(&*(card_name.to_owned() + "_b")));
    destination_stack.merge_stack(&transfer_stack);
}

pub fn perform_move_complex(destination_stack:&CardStack, card_name: &str, origin_stack: &CardStack, instruction: &str) {
    if instruction == "flip" {
        let origin_children = origin_stack.observe_children();
        for i in 0..origin_children.n_items() {
            let card = origin_stack.last_child().unwrap().downcast::<gtk::Picture>().unwrap();
            crate::renderer::flip_card(&card);
            card.unparent();
            destination_stack.add_card(&card);
        }
    } else {
        let transfer_stack = origin_stack.try_split_to_new_on(card_name).unwrap_or_else(|_| origin_stack.split_to_new_on(&*(card_name.to_owned() + "_b")));
        destination_stack.merge_stack(&transfer_stack);
    }
}

pub fn is_one_rank_above(card_lower: &glib::GString, card_higher: &glib::GString) -> bool {
    let lower_rank = card_lower.split("_").nth(1).unwrap();
    let higher_rank = card_higher.split("_").nth(1).unwrap();
    let lower_index = games::RANKS.iter().position(|x| x == &lower_rank).unwrap();
    let higher_index = games::RANKS.iter().position(|x| x == &higher_rank).unwrap();

    if lower_index + 1 == higher_index {
        true
    } else {
        false
    }
}

pub fn is_same_suit(card_1: &glib::GString, card_2: &glib::GString) -> bool {
    (card_1.starts_with("heart")   && card_2.starts_with("heart")   ) ||
    (card_1.starts_with("diamond") && card_2.starts_with("diamond") ) ||
    (card_1.starts_with("club")    && card_2.starts_with("club")    ) ||
    (card_1.starts_with("spade")   && card_2.starts_with("spade")   )
}

pub fn is_similar_suit(card_1: &glib::GString, card_2: &glib::GString) -> bool {
    (card_1.starts_with("heart") || card_1.starts_with("diamond")) ==
    (card_2.starts_with("heart") || card_2.starts_with("diamond"))
}

pub fn add_to_history(origin_stack: &str, card_name: &str, destination_stack: &str) {
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

    if card_name.ends_with("_b") {
        ACTION_HISTORY.with(|history| history.borrow_mut().push(format!("{}&>{}&>{}", origin_stack, card_name.replace("_b", ""), destination_stack)));
    } else {
        ACTION_HISTORY.with(|history| history.borrow_mut().push(format!("{}&>{}&>{}", origin_stack, card_name, destination_stack)));
    }
    HISTORY_INDEX.set(move_index + 1);
}

pub fn get_n_move(n: usize) -> (String, String, String) { // origin_stack, card_name, destination_stack
    let last_entry = ACTION_HISTORY.with(|history| history.borrow().iter().nth(n).unwrap().clone());
    let mut last_entry_parts = last_entry.splitn(3, "&>");
    (
        last_entry_parts.next().unwrap().to_string(),
        last_entry_parts.next().unwrap().to_string(),
        last_entry_parts.next().unwrap().to_string()
    )
}

pub fn undo_last_move() {
    let move_index = HISTORY_INDEX.with(|index| index.borrow().clone());
    if !(move_index == 0) {
        let (origin_stack, card_name, destination_stack) = get_n_move(move_index - 1);
        let grid = get_grid().unwrap();
        let destination_stack_widget = get_child(&grid, &destination_stack).unwrap().downcast::<CardStack>().unwrap();
        if origin_stack.contains("->") {
            let mut origin_parts = origin_stack.split("->");
            let instruction = origin_parts.next().unwrap();
            let destination_stack = origin_parts.next().unwrap().to_string();
            let origin_stack_widget = get_child(&grid, &destination_stack).unwrap().downcast::<CardStack>().unwrap();
            perform_move_complex(&origin_stack_widget, &card_name, &destination_stack_widget, instruction);
        } else {
            let origin_stack_widget = get_child(&grid, &origin_stack).unwrap().downcast::<CardStack>().unwrap();
            games::pre_undo_drag(&origin_stack_widget, &destination_stack_widget);
            perform_move(&origin_stack_widget, &card_name, &destination_stack_widget);
        }
        HISTORY_INDEX.set(move_index - 1);
    }
}

pub fn redo_first_move() {
    let move_index = HISTORY_INDEX.with(|index| index.borrow().clone());
    if let Some(first_entry) = ACTION_HISTORY.with(|history| history.borrow().get(move_index).cloned()) {
        let mut first_entry_parts = first_entry.splitn(3, "&>");
        let destination_stack = first_entry_parts.next().unwrap().to_string();
        let card_name = first_entry_parts.next().unwrap().to_string();
        let origin_stack = first_entry_parts.next().unwrap().to_string();

        let grid = get_grid().unwrap();
        let origin_stack_widget = get_child(&grid, &origin_stack).unwrap().downcast::<CardStack>().unwrap();
        if destination_stack.contains("->") {
            let mut destination_parts = destination_stack.split("->");
            let instruction = destination_parts.next().unwrap().to_string();
            let destination_stack = destination_parts.next().unwrap().to_string();
            let destination_stack_widget = get_child(&grid, &destination_stack).unwrap().downcast::<CardStack>().unwrap();
            perform_move_complex(&origin_stack_widget, &card_name, &destination_stack_widget, &instruction);
            games::on_drag_completed(&destination_stack_widget);
        } else {
            let destination_stack_widget = get_child(&grid, &destination_stack).unwrap().downcast::<CardStack>().unwrap();
            perform_move(&origin_stack_widget, &card_name, &destination_stack_widget);
            games::on_drag_completed(&destination_stack_widget);
        }
        games::on_drop_completed(&origin_stack_widget);
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