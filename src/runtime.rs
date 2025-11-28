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
    pub instruction: MoveInstruction,
    pub flip_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveInstruction {
    Flip,
    None,
}

pub fn create_move(origin_stack: &str, card_name: &str, destination_stack: &str, instruction: MoveInstruction) -> Move {
    Move {
        origin_stack: origin_stack.to_string(),
        card_name: card_name.to_string(),
        destination_stack: destination_stack.to_string(),
        instruction,
        flip_index: None,
    }
}

pub fn move_from_strings(origin_stack: String, card_name: String, destination_stack: String, instruction: MoveInstruction) -> Move {
    Move {
        origin_stack,
        card_name,
        destination_stack,
        instruction,
        flip_index: None,
    }
}

use std::cell::RefCell;
thread_local! {
    static STACK_NAMES: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static STACKS: RefCell<Vec<CardStack>> = RefCell::new(Vec::new());
    static HISTORY: RefCell<Vec<Move>> = RefCell::new(Vec::new());
    static UNDO_HISTORY: RefCell<Vec<Move>> = RefCell::new(Vec::new());
    static CARDS: RefCell<Vec<Card>> = RefCell::new(Vec::new());
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

pub fn perform_move(move_: &mut Move) {
    let origin_stack = get_stack(&move_.origin_stack).unwrap();
    let destination_stack = get_stack(&move_.destination_stack).unwrap();
    perform_move_with_stacks(move_, &origin_stack, &destination_stack);
}

pub fn perform_move_with_stacks(move_: &mut Move, origin_stack: &CardStack, destination_stack: &CardStack) {
    match move_.instruction {
        MoveInstruction::Flip => { //Fixme
            let origin_children = origin_stack.observe_children();
            let split_index = crate::card_stack::get_index(&*move_.card_name, &origin_children).unwrap();
            move_.card_name = origin_stack.last_child().unwrap().widget_name().to_string();
            for _ in split_index..origin_children.n_items() {
                let card = origin_stack.last_child().unwrap().downcast::<Card>().unwrap();
                card.flip();
                card.unparent();
                destination_stack.add_card(&card);
                card.remove_css_class("highlight");
            }
            return
        },
        MoveInstruction::None => {},
    }
    let transfer_stack = origin_stack.split_to_new_on(&*move_.card_name);
    destination_stack.merge_stack(&transfer_stack);
}

pub fn add_to_history(move_: Move) {
    // Remove invalidated undo entries
    let window = crate::window::SolitaireWindow::get_window().unwrap();
    UNDO_HISTORY.with(|undos| undos.borrow_mut().clear());

    update_redo_actions(&window);
    HISTORY.with(|h| h.borrow_mut().push(move_));
}

pub fn undo_last_move() {
    if HISTORY.with(|h| h.borrow().is_empty()) { return; }
    HISTORY.with(|history| {
        let mut history = history.borrow_mut();
        let mut last_entry = history.pop().unwrap();
        let destination_stack = get_stack(&last_entry.origin_stack).unwrap();
        let origin_stack = get_stack(&last_entry.destination_stack).unwrap();
        games::pre_undo_drag(&destination_stack, &origin_stack, &mut last_entry);
        perform_move_with_stacks(&mut last_entry, &origin_stack, &destination_stack);
        UNDO_HISTORY.with(|undos| undos.borrow_mut().push(last_entry));
    });
}

pub fn redo_first_move() {
    if UNDO_HISTORY.with(|undos| undos.borrow().is_empty()) { return; }
    UNDO_HISTORY.with(|undos| {
        let mut first_entry = undos.borrow_mut().pop().unwrap();
        let origin_stack = get_stack(&first_entry.origin_stack).unwrap();
        let destination_stack = get_stack(&first_entry.destination_stack).unwrap();
        perform_move(&mut first_entry);
        games::on_drag_completed(&origin_stack, &destination_stack, &mut first_entry);
        HISTORY.with(|history| history.borrow_mut().push(first_entry));
    });
}

pub fn update_redo_actions(window: &crate::window::SolitaireWindow) {
    let undo_action = window.lookup_action("undo").unwrap().downcast::<gio::SimpleAction>().unwrap();
    let redo_action = window.lookup_action("redo").unwrap().downcast::<gio::SimpleAction>().unwrap();
    HISTORY.with(|h| undo_action.set_enabled(!h.borrow().is_empty()));
    UNDO_HISTORY.with(|undos| redo_action.set_enabled(!undos.borrow().is_empty()));
}

pub fn clear_history_and_moves() {
    HISTORY.with(|h| h.borrow_mut().clear());
    UNDO_HISTORY.with(|undos| undos.borrow_mut().clear());
}

pub fn get_stack(name: &str) -> Option<CardStack> {
    let position = STACK_NAMES.with(|names| names.borrow().iter().position(|n| n == name))?;
    STACKS.with(|stacks| stacks.borrow().get(position).cloned())
}

pub fn add_stack(name: &str, stack: &CardStack) {
    STACK_NAMES.with(|names| names.borrow_mut().push(name.to_string()));
    STACKS.with(|stacks| stacks.borrow_mut().push(stack.clone()));
}

pub fn clear_state() {
    STACK_NAMES.with(|names| names.borrow_mut().clear());
    STACKS.with(|stacks| stacks.borrow_mut().clear());
}

pub fn set_cards(cards: Vec<Card>) {
    CARDS.set(cards);
}

pub fn get_cards() -> Vec<Card> {
    CARDS.with(|cards| cards.borrow().to_owned())
}