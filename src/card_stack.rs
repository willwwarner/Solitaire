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
use gtk::{glib, gdk, gsk, DragSource, GestureClick};
use adw::prelude::*;
use adw::subclass::prelude::*;
use std::cell::Cell;
use std::cmp::PartialEq;
use crate::{games, renderer, runtime};

glib::wrapper! {
    pub struct CardStack(ObjectSubclass<imp::CardStack>)
        @extends gtk::Widget;
}

glib::wrapper! {
    pub struct TransferCardStack(ObjectSubclass<imp::TransferCardStack>)
        @extends gtk::Widget;
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
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for CardStack {
        fn constructed(&self) {
            self.fan_cards.set(true);
            self.obj().add_css_class("card-stack-marker");
        }
    }

    impl WidgetImpl for CardStack {
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            if for_size == 0 { panic!("solitaire: card_stack: for_size == 0!") }
            // If orientation == horizontal, then for_size is the height
            // if for_size == -1 {
            if orientation == gtk::Orientation::Horizontal {
                return (25, 250, -1, -1);
            } else { // orientation == gtk::Orientation::Vertical
                if self.fan_cards.get() == true {
                    return (140, 1400, -1, -1)
                } else {
                    return (35, 350, -1, -1)
                }
            }
            // }
            // if self.fan_cards.get() == true {
            //     println!("fan_cards: true");
            //     if orientation == gtk::Orientation::Horizontal {
            //         return ((for_size / 4) - 10, 250, -1, -1);
            //     } else { // orientation == gtk::Orientation::Vertical
            //         return ((for_size * 4) - 10, 1400, 0, 0);
            //     }
            // } else {
            //     if orientation == gtk::Orientation::Horizontal {
            //         return (20, 250, -1, -1);
            //     } else { // orientation == gtk::Orientation::Vertical
            //         let min_height = (for_size as f32 * renderer::ASPECT) as i32 - 10;
            //         return (min_height, min_height, 0, 0);
            //     }
            // }
        }

        // A size_allocate that keeps the correct Aspect ratio for each stack
        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            let widget = self.obj();
            let children = widget.observe_children();
            let child_count = children.n_items();
            // Don't bother with empty stacks
            if child_count == 0 {
                return;
            }

            let allocation_width;
            let allocation_height;
            if self.fan_cards.get() == true {
                let max_height = width * 4;
                if child_count == 1 {
                    if height > max_height {
                        widget.first_child().unwrap().allocate(width, (width as f32 * renderer::ASPECT) as i32, -1, None);
                    } else {
                        widget.first_child().unwrap().allocate(height / 4, ((height / 4) as f32 * renderer::ASPECT) as i32, -1, None);
                    }
                    return;
                }

                let vertical_offset;

                if height > max_height {
                    allocation_height = (width as f32 * renderer::ASPECT).floor() as i32;
                    vertical_offset = std::cmp::min((max_height - allocation_height) / (child_count as i32 - 1), allocation_height / 3) as u32;
                    allocation_width = width;
                } else {
                    allocation_height = ((height / 4) as f32 * renderer::ASPECT).floor() as i32;
                    vertical_offset = std::cmp::min((height - allocation_height) / (child_count as i32 - 1), allocation_height / 3) as u32;
                    allocation_width = height / 4;
                }

                // Position each card with proper spacing
                for i in 0..child_count {
                    if let Some(child) = children.item(i) {
                        if let Ok(picture) = child.downcast::<gtk::Widget>() {
                            let y_pos = (i * vertical_offset) as f32;
                            picture.allocate(allocation_width, allocation_height, -1, Some(gsk::Transform::new().translate(&gtk::graphene::Point::new(0.0, y_pos))));
                        }
                    }
                }
                self.v_offset.set(vertical_offset);
            } else {
                let card_height = (width as f32 * renderer::ASPECT).floor() as i32;
                if height > card_height {
                    allocation_width = width;
                    allocation_height = card_height;
                } else {
                    allocation_width = (height as f32 / renderer::ASPECT) as i32;
                    allocation_height = height;
                }

                for i in 0..child_count {
                    if let Some(child) = children.item(i) {
                        if let Ok(picture) = child.downcast::<gtk::Widget>() {
                            picture.allocate(allocation_width, allocation_height, -1, None);
                        }
                    }
                }
            }
        }

        // fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        //     let widget = self.obj();
        //     let children = widget.observe_children();
        //     let child_count = children.n_items();
        //     // Don't bother with empty stacks
        //     if child_count == 0 {
        //         return;
        //     }
        //
        //     if child_count == 1 {
        //         widget.first_child().unwrap().allocate(width, (width as f32 * renderer::ASPECT) as i32, -1, None);
        //         return;
        //     }
        //     let card_height = (width as f32 * renderer::ASPECT).floor() as i32;
        //
        //     if self.fan_cards.get() == true {
        //         let max_height = width * 4;
        //         let vertical_offset;
        //
        //         if height > max_height {
        //             vertical_offset = std::cmp::min((max_height - card_height) / (child_count as i32 - 1), card_height / 3) as u32;
        //         } else {
        //             vertical_offset = std::cmp::min((height - card_height) / (child_count as i32 - 1), card_height / 3) as u32;
        //         }
        //
        //         // Position each card with proper spacing
        //         for i in 0..child_count {
        //             if let Some(child) = children.item(i) {
        //                 if let Ok(picture) = child.downcast::<gtk::Widget>() {
        //                     let y_pos = (i * vertical_offset) as f32;
        //                     picture.allocate(width, card_height, -1, Some(gsk::Transform::new().translate(&gtk::graphene::Point::new(0.0, y_pos))));
        //                 }
        //             }
        //         }
        //         self.v_offset.set(vertical_offset);
        //     } else {
        //         for i in 0..child_count {
        //             if let Some(child) = children.item(i) {
        //                 if let Ok(picture) = child.downcast::<gtk::Widget>() {
        //                     picture.allocate(width, card_height, -1, None);
        //                 }
        //             }
        //         }
        //     }
        // }
    }

    #[derive(Default)]
    pub struct TransferCardStack {
        pub origin_name: Cell<String>,
        pub v_offset: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TransferCardStack {
        const NAME: &'static str = "TransferCardStack";
        type Type = super::TransferCardStack;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for TransferCardStack {}
    impl WidgetImpl for TransferCardStack {
        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            let widget = self.obj();
            let children = widget.observe_children();
            let child_count = children.n_items();

            if child_count == 0 {
                return;
            }

            if child_count == 1 {
                widget.first_child().unwrap().allocate(width, (width as f32 * renderer::ASPECT) as i32, -1, None);
                return;
            }

            let card_height = (width as f32 * renderer::ASPECT).floor() as i32; // Use floor() because the lower height means spacing is not messed up
            if height < card_height {
                panic!("solitaire: transfer_card_stack height is is less than card_height, height: {height}");
            }

            let vertical_offset = self.v_offset.get();

            // Position each card with proper spacing
            for i in 0..child_count {
                if let Some(child) = children.item(i) {
                    if let Ok(picture) = child.downcast::<gtk::Widget>() {
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
        drop_target.connect_drop(|drop, val, _x, _y| {
            let to_stack = drop.widget().unwrap().downcast::<CardStack>().unwrap();
            if let Ok(transfer_stack) = val.get::<TransferCardStack>() {
                let first_card = transfer_stack.first_child().unwrap();
                if games::verify_drop(&first_card, &to_stack) {
                    to_stack.merge_stack(&transfer_stack);
                    games::on_drop_completed(&to_stack);
                    runtime::add_to_history(transfer_stack.get_origin_name().as_str(), first_card.widget_name().as_str(), to_stack.widget_name().as_str());
                    return true;
                }
                else { return false; }
            } else {
                let type_name = drop.value_type().to_string();
                glib::g_warning!("solitaire", "Tried to drop a non-TransferCardStack onto a CardStack type: {type_name}");
                return false;
            }
        });
        self.add_controller(drop_target);
    }

    pub fn add_click_to_slot(&self) {
        let click = GestureClick::new();
        let slot_clone = self.to_owned();
        click.connect_begin(move |_click, _sequence| {
            games::on_slot_click(&slot_clone);
        });
        self.add_controller(click);
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
            self.remove_card(&picture);
            new_stack.add_card(&picture);
        }
        self.imp().size_allocate(self.width(), self.height(), self.baseline());
        new_stack.set_height_request(self.height());
        new_stack.set_width_request(self.width());
        
        new_stack
    }

    pub fn try_split_to_new_on(&self, card_name: &str) -> Result<TransferCardStack, glib::Error> {
        // Attempt to locate the child with the given card name
        let children = self.observe_children();
        let total_children = children.n_items();
        let new_stack = TransferCardStack::new();
        new_stack.imp().v_offset.set(self.imp().v_offset.get());
        new_stack.imp().origin_name.set(self.widget_name().to_string());

        // First, find the starting index
        let start_index = get_index(card_name, &children)?;
        for _i in start_index..total_children {
            let child = children.item(start_index).expect("Failed to get child from CardStack");
            let picture = child.downcast::<gtk::Picture>().expect("Child is not a gtk::Picture (split:1)");
            self.remove_card(&picture);
            new_stack.add_card(&picture);
        }
        self.imp().size_allocate(self.width(), self.height(), self.baseline());
        new_stack.set_height_request(self.height());
        new_stack.set_width_request(self.width());

        Ok(new_stack)
    }

    pub fn merge_stack(&self, stack: &TransferCardStack) {
        let items = stack.observe_children().n_items();
        for _i in 0..items {
            let child = stack.first_child().expect("Failed to get first child from CardStack");
            let picture = child.downcast::<gtk::Picture>().expect("Child is not a gtk::Picture (merge)");
            stack.remove_card(&picture);
            self.add_card(&picture);
        }
        self.imp().size_allocate(self.width(), self.height(), self.baseline());
        stack.unrealize();
    }

    pub fn add_card(&self, card_picture: &gtk::Picture) {
        // Only add the picture if it doesn't already have a parent
        if card_picture.parent().is_none() {
            card_picture.insert_before(self, None::<&gtk::Widget>);
            self.remove_css_class("card-stack-marker");
        }  else {
            // If the picture already has a parent, log a warning
            glib::g_warning!("solitaire", "Attempted to add a widget that already has a parent");
        }
    }
    
    pub fn dissolve_to_row(self, grid: &gtk::Grid, row: i32) {
        let items = self.observe_children().n_items();
        for i in 0..items {
            let child = self.first_child().expect("Failed to get first child from CardStack");
            let picture = child.downcast::<gtk::Picture>().expect("Child is not a gtk::Picture (dissolve)");
            self.remove_card(&picture);
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

    pub fn face_up_top_card(&self) -> bool {
        if let Some(widget) = self.last_child() {
            let card = widget.downcast::<gtk::Picture>().expect("Child is not a gtk::Picture (flip)");
            renderer::flip_to_face(&card);
            return false; // The stack is not empty
        }
        true // The stack is empty
    }

    pub fn face_down_top_card(&self) -> bool {
        if let Some(widget) = self.last_child() {
            let card = widget.downcast::<gtk::Picture>().expect("Child is not a gtk::Picture (flip)");
            renderer::flip_to_back(&card);
            return false; // The stack is not empty
        }
        true // The stack is empty
    }

    pub fn add_drag_to_card(&self, card: &gtk::Picture) {
        let drag_source = DragSource::builder()
            .actions(gdk::DragAction::MOVE)  // allow moving the stack
            .build();

        drag_source.connect_prepare(move |src, _x, _y| {
            let stack = src.widget().unwrap().parent().unwrap().downcast::<CardStack>().unwrap();
            if games::verify_drag(&src.widget().unwrap(), &stack) {
                let move_stack = stack.split_to_new_on(&*src.widget().unwrap().widget_name());
                // Convert the CardStack (a GObject) into a GValue, then a ContentProvider.
                let value = move_stack.upcast::<glib::Object>().to_value();
                let provider = gdk::ContentProvider::for_value(&value);
                src.set_content(Some(&provider));  // attach the data provider
                Some(provider)  // must return Some(provider) from prepare
            } else {
                None
            }
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
        runtime::get_child(self, card_name).expect("Couldn't get card").grab_focus();
    }

    pub fn remove_card(&self, picture: &gtk::Picture) {
        if self.first_child().expect("Tried to remove a card from a stack that has no children") == *picture {
            self.add_css_class("card-stack-marker");
        }
        picture.unparent();
    }
}

impl TransferCardStack {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn add_card(&self, card_picture: &gtk::Picture) {
        // Only add the picture if it doesn't already have a parent
        if card_picture.parent().is_none() {
            card_picture.insert_before(self, None::<&gtk::Widget>);
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

    pub fn remove_card(&self, picture: &gtk::Picture) {
        picture.unparent();
    }
}