/* card_stack.rs
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

use adw::gio::ListModel;
use gtk::{glib, gdk, gsk, DragSource};
use adw::prelude::*;
use adw::subclass::prelude::*;
use std::cell::Cell;
use std::cmp::PartialEq;
use crate::{games, renderer, runtime};

glib::wrapper! {
    pub struct CardStack(ObjectSubclass<imp::CardStack>)
        @extends gtk::Box, gtk::Widget;
}

glib::wrapper! {
    pub struct TransferCardStack(ObjectSubclass<imp::TransferCardStack>)
        @extends gtk::Box, gtk::Widget;
}

pub fn get_index(card_name: &str, children: &ListModel) -> Result<u32, glib::Error> {
    // Attempt to locate the child with the given card name
    let total_children = children.n_items();

    // Loop through all the children widgets to find the matching card
    for i in 0..total_children {
        let child = children.item(i).expect("Failed to get child from CardStack");
        let picture = child.downcast::<gtk::Picture>().expect("Child is not a gtk::Picture (find)");
        if picture.widget_name() == card_name {
            return Ok(i);
        }
    }

    Err(glib::Error::new(glib::FileError::Exist, format!("Card named '{}' was not found in the stack.", card_name).as_str()))
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct CardStack {
        pub fan_cards: Cell<bool>,
        pub v_offset: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CardStack {
        const NAME: &'static str = "CardStack";
        type Type = super::CardStack;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for CardStack {
        fn constructed(&self) {
            self.fan_cards.set(true);
        }
    }

    impl WidgetImpl for CardStack {
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            if for_size == 0 { panic!("solitaire: card_stack: for_size == 0!") }
            // If orientation == horizontal, then for_size is the height
            if for_size == -1 {
                if orientation == gtk::Orientation::Horizontal {
                    return (20, 30, -1, -1);
                } else if orientation == gtk::Orientation::Vertical {
                    return (60, 90, -1, -1);
                }
            }
            if orientation == gtk::Orientation::Horizontal {
                return (20, for_size / 3, -1, -1);
            } else if orientation == gtk::Orientation::Vertical {
                return (60, for_size * 3, -1, -1);
            } else { panic!("solitaire: orientation is not vertical or horizontal"); }
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            let widget = self.obj();
            let children = widget.observe_children();
            let child_count = children.n_items();
            // Don't bother with empty stacks
            if child_count == 0 {
                return;
            }

            if child_count == 1 {
                widget.first_child().unwrap().allocate(width, (width as f32 * renderer::ASPECT) as i32, -1, None);
                return;
            }

            let card_height = (width as f32 * renderer::ASPECT).floor() as i32; // Use floor() because the lower height means spacing is not messed up
            if height <= card_height {
                panic!("solitaire: card_stack height is is less than card_height, height: {height}");
            }

            if self.fan_cards.get() == true {
                let vertical_offset;
                let max_height = width * 4;
                if height > max_height {
                    vertical_offset = std::cmp::min((max_height - card_height) / (child_count as i32 - 1), card_height / 3) as u32;
                } else {
                    vertical_offset = std::cmp::min((height - card_height) / (child_count as i32 - 1), card_height / 3) as u32;
                }

                // Position each card with proper spacing
                for i in 0..child_count {
                    if let Some(child) = children.item(i) {
                        if let Ok(picture) = child.downcast::<gtk::Widget>() {
                            // Position the card vertically with the proper offset
                            // The formula ensures cards are properly staggered with the calculated offset
                            let y_pos = (i * vertical_offset) as f32;
                            picture.allocate(width, card_height, -1, Some(gsk::Transform::new().translate(&gtk::graphene::Point::new(0.0, y_pos))));
                        }
                    }
                }
                self.v_offset.set(vertical_offset);
            } else {
                for i in 0..child_count {
                    if let Some(child) = children.item(i) {
                        if let Ok(picture) = child.downcast::<gtk::Widget>() {
                            picture.allocate(width, card_height, -1, None);
                        }
                    }
                }
            }
        }
    }
    
    impl BoxImpl for CardStack {}

    #[derive(Default)]
    pub struct TransferCardStack {
        pub origin_name: Cell<String>,
        pub v_offset: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TransferCardStack {
        const NAME: &'static str = "TransferCardStack";
        type Type = super::TransferCardStack;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for TransferCardStack {}
    impl WidgetImpl for TransferCardStack {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            let widget = self.obj();
            let children = widget.observe_children();
            let child_count = children.n_items();
            // Don't bother with empty stacks
            if child_count == 0 {
                return;
            }

            if child_count == 1 {
                widget.first_child().unwrap().allocate(width, (width as f32 * renderer::ASPECT) as i32, -1, None);
                return;
            }

            let card_height = (width as f32 * renderer::ASPECT).floor() as i32; // Use floor() because the lower height means spacing is not messed up
            if height <= card_height {
                panic!("solitaire: card_stack height is is less than card_height, height: {height}");
            }

            let vertical_offset = self.v_offset.get();

            // Position each card with proper spacing
            for i in 0..child_count {
                if let Some(child) = children.item(i) {
                    if let Ok(picture) = child.downcast::<gtk::Widget>() {
                        // Position the card vertically with the proper offset
                        // The formula ensures cards are properly staggered with the calculated offset
                        let y_pos = (i * vertical_offset) as f32;
                        picture.allocate(width, card_height, -1, Some(gsk::Transform::new().translate(&gtk::graphene::Point::new(0.0, y_pos))));
                    }
                }
            }
        }
    }
    impl BoxImpl for TransferCardStack {}
}

impl CardStack {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_fan_cards(&self, fan_cards: bool) {
        self.imp().fan_cards.set(fan_cards);
    }

    pub fn enable_drop(&self) {
        let drop_target = gtk::DropTarget::new(glib::Type::OBJECT, gdk::DragAction::MOVE);
        let stack_clone = self.clone();
        drop_target.connect_drop(move |_, val, _, _| {
            let Ok(drop_stack) = val.get::<TransferCardStack>() else {
                glib::g_warning!("Tried to drop a non-TransferCardStack onto a CardStack", "Solitaire");
                return false;
            };
            stack_clone.merge_stack(&drop_stack);
            true
        });
        self.add_controller(drop_target);
    }

    // FIXME: this causes "Broken accounting of active state for widget" when the top card is moved (occasionally)
    pub fn split_to_new_on(&self, card_name: &str) -> TransferCardStack {
        // Attempt to locate the child with the given card name
        let children = self.observe_children();
        let total_children = children.n_items();
        let new_stack = TransferCardStack::new();
        new_stack.imp().v_offset.set(self.imp().v_offset.get());
        new_stack.imp().origin_name.set(self.widget_name().to_string());

        // First, find the starting index
        let start_index = get_index(card_name, &children).expect("Couldn't get card");
        for _i in start_index..total_children {
            let child = children.item(start_index).expect("Failed to get child from CardStack");
            let picture = child.downcast::<gtk::Picture>().expect("Child is not a gtk::Picture (split:1)");
            self.remove(&picture);
            new_stack.add_card(&picture);
        }
        self.imp().size_allocate(self.width(), self.height(), self.baseline());
        new_stack.set_height_request(self.height());
        new_stack.set_width_request(self.width());
        
        new_stack
    }

    pub fn merge_stack(&self, stack: &TransferCardStack) {
        let items = stack.observe_children().n_items();
        for _i in 0..items {
            let child = stack.first_child().expect("Failed to get first child from CardStack");
            let picture = child.downcast::<gtk::Picture>().expect("Child is not a gtk::Picture (merge)");
            stack.remove(&picture);
            self.add_card(&picture);
        }
        self.imp().size_allocate(self.width(), self.height(), self.baseline());
        stack.unrealize();
    }

    pub fn add_card(&self, card_picture: &gtk::Picture) {
        // Only add the picture if it doesn't already have a parent
        if card_picture.parent().is_none() {
            self.append(card_picture);
        } else {
            // If the picture already has a parent, log a warning
            glib::g_warning!("solitaire", "Attempted to add a widget that already has a parent");
        }
    }
    
    pub fn dissolve_to_row(self, grid: &gtk::Grid, row: i32) {
        let items = self.observe_children().n_items();
        for i in 0..items {
            let child = self.first_child().expect("Failed to get first child from CardStack");
            let picture = child.downcast::<gtk::Picture>().expect("Child is not a gtk::Picture (dissolve)");
            self.remove(&picture);
            grid.attach(&picture, i as i32, row, 1, 1);
        }
        grid.remove(&self);
        self.unrealize();
    }

    pub fn remove_child_controllers(&self) {
        let controllers = self.observe_controllers();
        for obj in &controllers {
            if let Ok(controller) = obj.unwrap().downcast::<gtk::EventController>() {
                self.remove_controller(&controller);
            }
        }
    }

    pub fn face_up_top_card(&self) {
        let card = self.last_child().expect("Failed to get last child from CardStack").downcast::<gtk::Picture>().expect("Child is not a gtk::Picture (flip)");
        renderer::flip_to_face(&card);
    }

    pub fn add_drag_to_card(&self, card: &gtk::Picture) {
        let drag_source = DragSource::builder()
            .actions(gdk::DragAction::MOVE)  // allow moving the stack
            .build();

        drag_source.connect_prepare(move |src, _x, _y| {
            let stack = src.widget().unwrap().parent().unwrap().downcast::<CardStack>().unwrap();
            let move_stack = stack.split_to_new_on(&*src.widget().unwrap().widget_name());
            move_stack.set_layout_manager(None::<gtk::LayoutManager>);
            // Convert the CardStack (a GObject) into a GValue, then a ContentProvider.
            let value = move_stack.upcast::<glib::Object>().to_value();
            let provider = gdk::ContentProvider::for_value(&value);
            src.set_content(Some(&provider));  // attach the data provider
            Some(provider)  // must return Some(provider) from prepare
        });

        drag_source.connect_drag_begin(|src, drag| {
            let icon = gtk::DragIcon::for_drag(drag);
            let provider = src.content().unwrap();
            let value = provider.value(glib::Type::OBJECT).unwrap();
            // I'd rather have no DnD icon instead of a crash
            if let Ok(obj) = value.get::<glib::Object>() {
                if let Ok(original_stack) = obj.downcast::<TransferCardStack>() {
                    let stack_clone = original_stack.clone();
                    icon.set_child(Some(&stack_clone));
                    stack_clone.allocate(original_stack.width_request(), original_stack.height_request(), 0, None);
                }
            }
        });

        drag_source.connect_drag_cancel(|src, _drag, _reason| {
            let provider = src.content().unwrap();
            let value = provider.value(glib::Type::OBJECT).unwrap();
            if let Ok(obj) = value.get::<glib::Object>() {
                let drag_stack = obj.downcast::<TransferCardStack>().unwrap();
                let origin = runtime::get_child(&runtime::get_grid().unwrap(), drag_stack.get_origin_name().as_str()).unwrap().downcast::<CardStack>().unwrap();
                origin.merge_stack(&drag_stack);
            }
            true
        });

        drag_source.connect_drag_end(|src, _drag, _result| {
            let value = src.content().unwrap().value(glib::Type::OBJECT).unwrap();
            let stack = value.get::<TransferCardStack>().unwrap();
            let origin = runtime::get_child(&runtime::get_grid().unwrap(), stack.get_origin_name().as_str()).unwrap();
            games::on_drag_completed(&origin.downcast::<CardStack>().unwrap());
        });

        card.add_controller(drag_source);
    }
    
    pub fn focus_card(&self, card_name: &str) {
        crate::runtime::get_child(self, card_name).expect("Couldn't get card").grab_focus();
    }

    pub fn remove_card(&self, picture: &gtk::Picture) {
        self.remove(picture);
    }
}

impl TransferCardStack {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn add_card(&self, card_picture: &gtk::Picture) {
        // Only add the picture if it doesn't already have a parent
        if card_picture.parent().is_none() {
            self.append(card_picture);
        } else {
            // If the picture already has a parent, log a warning
            glib::g_warning!("solitaire", "Attempted to add a widget that already has a parent");
        }
    }

    pub fn get_origin_name(&self) -> String {
        let name = self.imp().origin_name.take();
        self.imp().origin_name.set(name.clone());
        name
    }
}