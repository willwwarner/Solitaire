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

use std::cell::{Cell, RefCell};
use std::time::Duration;

thread_local! {
    static STACK_NAMES: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static STACKS: RefCell<Vec<CardStack>> = RefCell::new(Vec::new());
    static SOLUTION_MOVES: RefCell<Vec<Move>> = RefCell::new(Vec::new());
    static HISTORY: RefCell<Vec<Move>> = RefCell::new(Vec::new());
    static UNDO_HISTORY: RefCell<Vec<Move>> = RefCell::new(Vec::new());
    static N_DEALS: Cell<u8> = Cell::new(0);
    static CARDS: RefCell<Vec<Card>> = RefCell::new(Vec::new());
    static IS_WON_FN: RefCell<Option<Box<dyn FnMut(&mut games::solver::State) -> bool>>> = RefCell::new(None);
    // Re-solve multithreading
    static LATEST_SOLVING: Cell<usize> = Cell::new(0);
    static FIRST_UNSOLVABLE: Cell<usize> = Cell::new(usize::MAX);
    static FIRST_UNSOLVABLE_HISTORY: RefCell<Vec<Move>> = RefCell::new(Vec::new());
    static NOTIFY_UNSOLVABLE: Cell<bool> = Cell::new(true);
    static SOLVER_THREADS: RefCell<Vec<std::thread::JoinHandle<()>>> = RefCell::new(Vec::new());
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
    HISTORY.with(|h| h.borrow_mut().push(move_.clone()));
    let (stack_names, game_state) = get_solver_state();
    let mut ghost_solver_state = games::solver::new_ghost_state(game_state.to_owned());
    if IS_WON_FN.with_borrow_mut(|f| f.as_mut().unwrap()(&mut ghost_solver_state)) {
        window.won_dialog();
        return;
    }
    if let Some(solution_move) = get_hint() {
        if solution_move == move_ {
            SOLUTION_MOVES.with(|s| s.borrow_mut().remove(0));
            return;
        }
    }
    re_solve_threaded(&window, stack_names, game_state);
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
        if SOLUTION_MOVES.with(|s| !s.borrow().is_empty()) { // Fixme: This will make won games be re-solved
            SOLUTION_MOVES.with(|s| s.borrow_mut().insert(0, last_entry.clone()));
        }
        UNDO_HISTORY.with(|undos| undos.borrow_mut().push(last_entry));
    });
}

fn undo_many(last_index: usize) {
    if HISTORY.with(|h| h.borrow().is_empty()) { return; }
    HISTORY.with(|history| {
        let mut history = history.borrow_mut();
        for _ in last_index..history.len() {
            let mut last_entry = history.pop().unwrap();
            let destination_stack = get_stack(&last_entry.origin_stack).unwrap();
            let origin_stack = get_stack(&last_entry.destination_stack).unwrap();
            games::pre_undo_drag(&destination_stack, &origin_stack, &mut last_entry);
            perform_move_with_stacks(&mut last_entry, &origin_stack, &destination_stack);
            UNDO_HISTORY.with(|undos| undos.borrow_mut().push(last_entry));
        }
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
        HISTORY.with(|history| history.borrow_mut().push(first_entry.clone()));
        if let Some(solution_move) = get_hint() {
            if solution_move == first_entry {
                SOLUTION_MOVES.with(|s| s.borrow_mut().remove(0));
                return;
            }
        }
        let (stack_names, game_state) = get_solver_state();
        re_solve_threaded(&crate::window::SolitaireWindow::get_window().unwrap(), stack_names, game_state);
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
    SOLUTION_MOVES.with(|s| s.borrow_mut().clear());
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

pub fn get_solver_state() -> (Vec<String>, Vec<Vec<u8>>) {
    let names = STACK_NAMES.with(|names| names.borrow().to_owned());
    let stacks = STACKS.with(|stacks| {
        let mut solver_stacks = Vec::new();
        for stack in stacks.borrow().iter() {
            solver_stacks.push(stack.get_solver_stack())
        }
        solver_stacks
    });
    (names, stacks)
}

pub fn set_won_fn<F: FnMut(&mut games::solver::State) -> bool + 'static>(f: F) {
    IS_WON_FN.set(Some(Box::new(f)));
}

pub fn set_cards(cards: Vec<Card>) {
    CARDS.set(cards);
}

pub fn get_cards() -> Vec<Card> {
    CARDS.with(|cards| cards.borrow().to_owned())
}

pub fn get_deals() -> u8 {
    N_DEALS.get()
}

pub fn update_deals(n: u8) {
    N_DEALS.set(n);
}

pub fn get_hint() -> Option<Move> {
    SOLUTION_MOVES.with(|moves| moves.borrow().first().cloned())
}
pub fn set_solution(moves: Vec<Move>) {
    SOLUTION_MOVES.set(moves);
    NOTIFY_UNSOLVABLE.set(true);
}

pub fn drop() {
    let solution = SOLUTION_MOVES.with(|s| s.borrow().clone());
    glib::spawn_future_local(
        async move {
            for mut move_ in solution {
                let origin_stack = get_stack(&move_.origin_stack).unwrap();
                let destination_stack = get_stack(&move_.destination_stack).unwrap();
                perform_move_with_stacks(&mut move_, &origin_stack, &destination_stack);
                games::on_drag_completed(&origin_stack, &destination_stack, &mut move_);
                add_to_history(move_);
                glib::timeout_future(Duration::from_millis(300)).await;
            }
        }
    );
}

pub fn set_can_drop(can_drop: bool) {
    crate::window::SolitaireWindow::get_window().unwrap().set_can_drop(can_drop);
}

fn re_solve_threaded(window: &crate::window::SolitaireWindow, stack_names: Vec<String>, game_state: Vec<Vec<u8>>) {
    fn clear_and_abort_threads() {
        games::solver::set_should_stop(true);
        SOLVER_THREADS.with_borrow_mut(|t| t.clear());
    }

    window.set_hint_drop_enabled(false);
    glib::spawn_future_local(glib::clone!(
        #[weak]
        window,
        async move {
            let discarded_solver_history = SOLUTION_MOVES.with(|s| s.borrow().clone());
            let move_index = HISTORY.with(|h| h.borrow().len());
            let (sender, receiver) = async_channel::bounded(1);
            let t = std::thread::spawn(move || {
                let result = games::re_solve(stack_names, game_state);
                sender.send_blocking(result).unwrap();
            });
            SOLVER_THREADS.with_borrow_mut(|s| s.push(t));
            while let Ok(result) = receiver.recv().await {
                if let Some(history) = result {
                    if move_index < HISTORY.with_borrow(|h| h.len()) { continue; }
                    clear_and_abort_threads();
                    set_solution(history);
                    FIRST_UNSOLVABLE.set(usize::MAX);
                    if SOLUTION_MOVES.with(|s| s.borrow().is_empty()) { continue; }
                    window.set_hint_drop_enabled(true);
                } else {
                    if move_index < FIRST_UNSOLVABLE.get() && !games::solver::get_should_stop() {
                        FIRST_UNSOLVABLE.set(move_index);
                        FIRST_UNSOLVABLE_HISTORY.set(discarded_solver_history.clone());
                    }
                    let first_unsolvable = FIRST_UNSOLVABLE.get();
                    if move_index == HISTORY.with_borrow(|h| h.len()) {
                        if NOTIFY_UNSOLVABLE.get() {
                            let window = window.clone();
                            crate::window::SolitaireWindow::incompatible_move_dialog(move |_dialog, _response| { // Undo Button
                                undo_many(first_unsolvable - 1);
                                update_redo_actions(&window);
                                clear_and_abort_threads();
                                let first_unsolvable_h = FIRST_UNSOLVABLE_HISTORY.take();
                                if !first_unsolvable_h.is_empty() { window.set_hint_drop_enabled(true); }
                                SOLUTION_MOVES.set(first_unsolvable_h);
                            }, move |_dialog, _response| { // Keep Playing button
                                NOTIFY_UNSOLVABLE.set(false);
                                clear_and_abort_threads();
                            });
                        } else { clear_and_abort_threads(); }
                        FIRST_UNSOLVABLE.set(usize::MAX);
                    }
                }
            }
        }
    ));
}
