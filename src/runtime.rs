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

use gtk::{DragSource, GestureClick, glib, prelude::Cast};
use gtk::prelude::{IsA, ListModelExt, WidgetExt};

thread_local! {
    static GRID: std::cell::RefCell<Option<gtk::Grid>> = std::cell::RefCell::new(None);
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
    click.connect_released(move |_click, _n_press, _x, _y| {
        /* Having separate actions for double-clicks and single-clicks will be a pain.
         * That's why the plan for this is to have dragging the cards separate from click actions,
         * unlike in other solitaire games. Single-clicking will auto-move cards (in the future). */
        crate::games::on_card_click(&picture_clone);
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