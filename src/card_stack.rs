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
use crate::{card::Card, games, renderer, runtime};

glib::wrapper! {
    pub struct CardStack(ObjectSubclass<imp::CardStack>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

glib::wrapper! {
    pub struct TransferCardStack(ObjectSubclass<imp::TransferCardStack>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

pub fn get_index(card_name: &str, children: &ListModel) -> Result<u32, glib::Error> {
    // Attempt to locate the child with the given card name
    let total_children = children.n_items();

    // Loop through all the children widgets to find the matching card
    for i in 0..total_children {
        let child = children.item(i).expect("Failed to get child from CardStack");
        let card = child.downcast::<Card>().expect("Child is not a Card (find)");
        if card.widget_name() == card_name {
            return Ok(i);
        }
    }

    Err(glib::Error::new(glib::FileError::Exist, format!("Card named '{}' was not found in the stack.", card_name).as_str()))
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct CardStack {
        pub aspect: Cell<f32>,
        pub v_offset: Cell<u32>,
        pub stack_type: Cell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CardStack {
        const NAME: &'static str = "CardStack";
        type Type = super::CardStack;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for CardStack {
        fn constructed(&self) {
            self.obj().add_css_class("card-stack");
        }
    }

    impl WidgetImpl for CardStack {
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            if for_size == 0 { panic!("solitaire: card_stack: for_size == 0!") }
            // If orientation == horizontal, then for_size is the height
            if for_size == -1 {
                if orientation == gtk::Orientation::Horizontal {
                    return (25, 250, -1, -1);
                } else { // orientation == gtk::Orientation::Vertical
                    let aspect = self.aspect.get();
                    return (25 * aspect as i32, (250.0 * aspect) as i32, -1, -1);
                }
            } else {
                let aspect = self.aspect.get();
                if orientation == gtk::Orientation::Horizontal {
                    return (25, (for_size as f32 / aspect) as i32, -1, -1);
                } else { // orientation == gtk::Orientation::Vertical
                    return (25 * aspect as i32, (for_size as f32 * aspect) as i32, -1, -1);
                }
            }
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            let widget = self.obj();
            let children = widget.observe_children();
            let child_count = children.n_items();
            let stack_aspect = self.aspect.get();
            // Don't bother with empty stacks
            if child_count == 0 {
                return;
            }

            let allocation_width;
            let allocation_height;
            if stack_aspect != 1.4 {
                let max_height = (width as f32 * stack_aspect).floor() as i32;
                if child_count == 1 {
                    if height > max_height {
                        widget.first_child().unwrap().allocate(width, (width as f32 * renderer::ASPECT) as i32, -1, None);
                    } else {
                        let max_width = (height as f32 / stack_aspect) as i32;
                        widget.first_child().unwrap().allocate(max_width, (max_width as f32 * renderer::ASPECT) as i32, -1, None);
                    }
                    return;
                }

                let vertical_offset;

                if height > max_height {
                    allocation_height = (width as f32 * renderer::ASPECT).floor() as i32;
                    vertical_offset = std::cmp::min((max_height - allocation_height) / (child_count as i32 - 1), allocation_height / 5) as u32;
                    allocation_width = width;
                } else {
                    allocation_width = (height as f32 / stack_aspect).floor() as i32;
                    allocation_height = (allocation_width as f32 * renderer::ASPECT).floor() as i32;
                    vertical_offset = std::cmp::min((height - allocation_height) / (child_count as i32 - 1), allocation_height / 5) as u32;
                }

                // Position each card with proper spacing
                for i in 0..child_count {
                    if let Some(child) = children.item(i) {
                        if let Ok(card) = child.downcast::<gtk::Widget>() {
                            let y_pos = (i * vertical_offset) as f32;
                            card.allocate(allocation_width, allocation_height, -1, Some(gsk::Transform::new().translate(&gtk::graphene::Point::new(0.0, y_pos))));
                        }
                    }
                }
                self.v_offset.set(vertical_offset);
            } else {
                let max_card_height = (width as f32 * renderer::ASPECT).floor() as i32;
                if height > max_card_height {
                    allocation_width = width;
                    allocation_height = max_card_height;
                } else {
                    allocation_width = (height as f32 / renderer::ASPECT).floor() as i32;
                    allocation_height = height;
                }

                for i in 0..child_count {
                    if let Some(child) = children.item(i) {
                        if let Ok(card) = child.downcast::<gtk::Widget>() {
                            card.allocate(allocation_width, allocation_height, -1, None);
                        }
                    }
                }
            }
        }
    }

    #[derive(Default)]
    pub struct TransferCardStack {
        pub origin_name: Cell<String>,
        pub v_offset: Cell<u32>,
        pub card_width: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TransferCardStack {
        const NAME: &'static str = "TransferCardStack";
        type Type = super::TransferCardStack;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for TransferCardStack {}
    impl WidgetImpl for TransferCardStack {
        fn size_allocate(&self, _width: i32, height: i32, _baseline: i32) {
            let widget = self.obj();
            let children = widget.observe_children();
            let child_count = children.n_items();
            let card_width = self.card_width.get();

            if child_count == 0 {
                return;
            }

            if child_count == 1 {
                widget.first_child().unwrap().allocate(card_width, (card_width as f32 * renderer::ASPECT) as i32, -1, None);
                return;
            }

            let card_height = (card_width as f32 * renderer::ASPECT).floor() as i32;
            if height < card_height {
                panic!("solitaire: transfer_card_stack height is is less than card_height, height: {height}");
            }

            let vertical_offset = self.v_offset.get();

            // Position each card with proper spacing
            for i in 0..child_count {
                if let Some(child) = children.item(i) {
                    if let Ok(card) = child.downcast::<gtk::Widget>() {
                        let y_pos = (i * vertical_offset) as f32;
                        card.allocate(card_width, card_height, -1, Some(gsk::Transform::new().translate(&gtk::graphene::Point::new(0.0, y_pos))));
                    }
                }
            }
        }
    }
}

impl CardStack {
    pub fn new(aspect: f32, stack_type: &str, n_of_type: i32) -> Self {
        let this:CardStack = glib::Object::new();
        if aspect < 1.4 { panic!("solitaire: set_aspect() called with aspect < 1.4 \nRTL stacks are not currently supported."); }
        this.imp().aspect.set(aspect);
        this.imp().stack_type.set(stack_type.to_string());
        if n_of_type < 0 {
            this.set_widget_name(stack_type);
        } else {
            this.set_widget_name(&*format!("{}_{}", stack_type, n_of_type));
        }
        runtime::add_stack(&*this.widget_name(), &this);

        this
    }

    pub fn set_aspect(&self, aspect: f32) {
        if aspect < 1.4 { panic!("solitaire: set_aspect() called with aspect < 1.4 \nRTL stacks are not currently supported."); }
        self.imp().aspect.set(aspect);
    }

    pub fn get_type(&self) -> String {
        let value = self.imp().stack_type.take();
        self.imp().stack_type.set(value.clone());
        value
    }

    pub fn enable_drop(&self) {
        let drop_target = gtk::DropTarget::new(glib::Type::OBJECT, gdk::DragAction::MOVE);
        drop_target.connect_drop(|drop, val, _x, _y| {
            let to_stack = drop.widget().unwrap().downcast::<CardStack>().unwrap();
            if let Ok(transfer_stack) = val.get::<TransferCardStack>() {
                let first_card = transfer_stack.first_child().unwrap().downcast::<Card>().unwrap();
                if games::verify_drop(&first_card, &to_stack) {
                    to_stack.merge_stack(&transfer_stack);
                    let mut move_ = runtime::create_move(&transfer_stack.get_origin_name(),
                                                         &first_card.widget_name(),
                                                         &to_stack.widget_name(),
                                                         runtime::MoveInstruction::None);
                    games::on_drag_completed(&runtime::get_stack(&*transfer_stack.get_origin_name()).unwrap(), &to_stack, &mut move_);
                    //FIXME: do not use widget_name
                    runtime::add_to_history(move_);
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
        new_stack.imp().card_width.set(self.first_child().unwrap().width());

        // First, find the starting index
        let start_index = get_index(card_name, &children).expect("Couldn't get card");
        for _i in start_index..total_children {
            let child = children.item(start_index).expect("Failed to get child from CardStack");
            let card = child.downcast::<Card>().expect("Child is not a Card (split:1)");
            self.remove_card(&card);
            new_stack.add_card(&card);
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
            let card = child.downcast::<Card>().expect("Child is not a Card (split:1)");
            self.remove_card(&card);
            new_stack.add_card(&card);
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
            let card = child.downcast::<Card>().expect("Child is not a Card (merge)");
            stack.remove_card(&card);
            self.add_card(&card);
        }
        self.imp().size_allocate(self.width(), self.height(), self.baseline());
        stack.unrealize();
    }

    pub fn add_card(&self, card: &Card) {
        // Only add the card if it doesn't already have a parent
        if card.parent().is_none() {
            card.insert_before(self, None::<&gtk::Widget>);
        }  else {
            // If the card already has a parent, log a warning
            glib::g_warning!("solitaire", "Attempted to add a widget that already has a parent");
        }
    }
    
    pub fn destroy_and_return_cards(self, cards: &mut Vec<Card>) {
        let items = self.observe_children().n_items();
        for _ in 0..items {
            let child = self.first_child().expect("Failed to get first child from CardStack");
            let card = child.downcast::<Card>().expect("Child is not a Card (dissolve)");
            self.remove_card(&card);
            card.flip_to_face();
            cards.push(card);
        }
        self.unparent();
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
            let card = widget.downcast::<Card>().expect("Child is not a Card (flip)");
            card.flip_to_face();
            return false; // The stack is not empty
        }
        true // The stack is empty
    }

    pub fn face_down_top_card(&self) -> bool {
        if let Some(widget) = self.last_child() {
            let card = widget.downcast::<Card>().expect("Child is not a Card (flip)");
            card.flip_to_back();
            return false; // The stack is not empty
        }
        true // The stack is empty
    }

    pub fn add_drag_to_card(&self, card: &Card) {
        let drag_source = DragSource::builder()
            .actions(gdk::DragAction::MOVE)  // allow moving the stack
            .build();

        drag_source.connect_prepare(move |src, _x, _y| {
            let stack = src.widget().unwrap().parent().unwrap().downcast::<CardStack>().unwrap();
            if games::verify_drag(&src.widget().unwrap().downcast().unwrap(), &stack) {
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
                let origin = runtime::get_stack(&*drag_stack.get_origin_name()).unwrap();
                origin.merge_stack(&drag_stack);
            }
            true
        });

        card.add_controller(drag_source);
    }
    
    pub fn get_card_names(&self) -> Vec<String> {
        let mut card_names = Vec::new();
        let children = self.observe_children();
        let total_children = children.n_items();
        for i in 0..total_children {
            let child = children.item(i).expect("Failed to get child from CardStack");
            let card = child.downcast::<Card>().expect("Child is not a Card (get_card_names)");
            card_names.push(card.widget_name().to_string());
        }
        card_names
    }
    
    pub fn focus_card(&self, card_name: String) {
        runtime::get_child(self, &*card_name).expect("Couldn't get card").grab_focus();
    }

    pub fn remove_card(&self, card: &Card) {
        card.unparent();
    }

    pub fn is_empty(&self) -> bool {
        self.first_child().is_none()
    }

    pub fn last_card(&self) -> Option<Card> {
        self.last_child()?.downcast::<Card>().ok()
    }

    pub fn first_card(&self) -> Option<Card> {
        self.first_child()?.downcast::<Card>().ok()
    }

    pub fn n_cards(&self) -> usize {
        self.observe_children().n_items() as usize
    }

    pub fn get_card(&self, index: usize) -> Option<Card> {
        self.observe_children().item(index as u32)?.downcast::<Card>().ok()
    }
}

impl TransferCardStack {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn add_card(&self, card: &Card) {
        // Only add the card if it doesn't already have a parent
        if card.parent().is_none() {
            card.insert_before(self, None::<&gtk::Widget>);
        } else {
            // If the card already has a parent, log a warning
            glib::g_warning!("solitaire", "Attempted to add a widget that already has a parent");
        }
    }

    pub fn get_origin_name(&self) -> String {
        let name = self.imp().origin_name.take();
        self.imp().origin_name.set(name.clone());
        name
    }

    pub fn remove_card(&self, card: &Card) {
        card.unparent();
    }
}