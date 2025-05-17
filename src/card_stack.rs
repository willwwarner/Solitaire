/* card_stack.rs
 *
 * Copyright 2025 Shbozz
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

use gtk::{glib, gdk};
use adw::prelude::*;
use adw::subclass::prelude::*;
use crate::renderer;

glib::wrapper! {
    pub struct CardStack(ObjectSubclass<imp::CardStack>)
        @extends gtk::Fixed, gtk::Widget;
}

pub fn calculate_offset(stack_height: i32, num_cards: u32, card_height: i32) -> u32 {
    if num_cards == 0 {
        return 0;
    }

    // Calculate the overlap percentage (e.g., 50% of card height visible)
    let max_spacing = (card_height / 2) as u32;

    // Calculate the offset without exceeding the stack height
    std::cmp::min((stack_height / num_cards as i32) as u32, max_spacing)
}

mod imp {
    use gtk::Snapshot;
    use super::*;

    #[derive(Default)]
    pub struct CardStack {
        pub is_stackable: bool,
        // Store row for layout calculations
        pub row: u8,
        pub col: u8,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CardStack {
        const NAME: &'static str = "CardStack";
        type Type = super::CardStack;
        type ParentType = gtk::Fixed;
    }
    impl ObjectImpl for CardStack {}
    impl WidgetImpl for CardStack {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            // Call the parent implementation to ensure default behavior
            self.parent_size_allocate(width, height, baseline);

            let widget = self.obj();
            let children = widget.observe_children();
            let child_count = children.n_items();

            // Don't bother with empty stacks
            if child_count == 0 {
                return;
            }

            // Calculate a size that maintains aspect ratio for cards
            let card_width = width;
            let card_height = (card_width as f32 * renderer::ASPECT) as i32;

            // Calculate vertical spacing between cards
            let total_cards = child_count;
            let vertical_offset = calculate_offset(height, total_cards, card_height);

            // Position each card with proper spacing
            for i in 0..child_count {
                if let Some(child) = children.item(i) {
                    if let Ok(image) = child.downcast::<gtk::Image>() {
                        // Set explicit size request for the image
                        image.set_size_request(card_width, card_height);

                        // Position the card vertically with proper offset
                        // The formula ensures cards are properly staggered with the calculated offset
                        let y_pos = (i * vertical_offset) as f64;
                        widget.move_(&image, 0.0, y_pos);
                    }
                }
            }
        }
    }
    impl FixedImpl for CardStack {}
}

impl CardStack {
    pub fn new() -> Self {
        glib::Object::new()
    }
    // Most stack methods could use these
    pub fn get_card(&self, card_name: &str) -> Result<gtk::Image, glib::Error> {
        // Attempt to locate the child with the given card name
        let children = self.observe_children();
        let total_children = children.n_items();

        // Loop through all the children widgets to find the matching card
        for i in 0..total_children {
            let child = children.item(i).expect("Failed to get child from CardStack");
            let image = child.downcast::<gtk::Image>().expect("Child is not a gtk::Image (find)");
            if image.widget_name() == card_name {
                return Ok(image);
            }
        }

        Err(glib::Error::new(glib::FileError::Exist, format!("Card named '{}' was not found in the stack.", card_name).as_str()))
    }

    pub fn get_card_and_children(&self, card_name: &str) -> Result<(gtk::Image, crate::gio::ListModel, u32), glib::Error> {
        // Attempt to locate the child with the given card name
        let children = self.observe_children();
        let total_children = children.n_items();

        // Loop through all the children widgets to find the matching card
        for i in 0..total_children {
            let child = children.item(i).expect("Failed to get child from CardStack");
            let image = child.downcast::<gtk::Image>().expect("Child is not a gtk::Image (find)");
            if image.widget_name() == card_name {
                return Ok((image, children, total_children));
            }
        }

        Err(glib::Error::new(glib::FileError::Exist, format!("Card named '{}' was not found in the stack.", card_name).as_str()))
    }

    pub fn enable_drop(&self) {
        let drop_target = gtk::DropTarget::new(glib::Type::OBJECT, gdk::DragAction::MOVE);
        //drop_target.set_highlight(false); or something, the highlighting isn't ideal 
        let stack_clone = self.clone();
        drop_target.connect_drop(move |_, val, height, _| {
            let stack = val.get::<CardStack>().expect("Failed to get CardStack from DropTarget");
            let children = stack.observe_children();
            let child_count = children.n_items();

            for i in (child_count..0).rev() {
                let child = children.item(i).expect("Failed to get child from CardStack");
                let image = child.downcast::<gtk::Image>().expect("Child is not a gtk::Image (drop)");
                stack.remove(&image);
                stack_clone.add_card(&image, height as i32);
            }
            true
        });
        self.add_controller(drop_target);
    }

    pub fn split_to_new_on(&self, card_name: &str) -> Result<CardStack, glib::Error> {
        // Attempt to locate the child with the given card name
        let children = self.observe_children();
        let total_children = children.n_items();
        let mut cards_to_move = Vec::new();

        // First find the starting index
        let mut start_index = None;
        for i in 0..total_children {
            let child = children.item(i).expect("Failed to get child from CardStack");
            let image = child.downcast::<gtk::Image>().expect("Child is not a gtk::Image (split:1)");
            if image.widget_name() == card_name {
                start_index = Some(i);
                break;
            }
        }

        // If we found the card, collect all cards from that index onwards
        if let Some(start_idx) = start_index {
            let new_stack = CardStack::new();
        
            // First collect all the cards we want to move
            for j in start_idx..total_children {
                let child = children.item(j).expect("Failed to get child from CardStack");
                let image = child.downcast::<gtk::Image>().expect("Child is not a gtk::Image (split:2)");
                cards_to_move.push(image);
            }

            // Then remove and add them to the new stack
            for image in cards_to_move {
                self.remove(&image);
                new_stack.add_card(&image, image.height());
            }

            return Ok(new_stack);
        }

        // If the card is not found, return an error
        Err(glib::Error::new(
            glib::FileError::Exist,
            format!("Card named '{}' was not found in the stack.", card_name).as_str()
        ))
    }

    pub fn merge_stack(&self, stack: &CardStack) {
        let items = stack.observe_children().n_items();
        for i in 0..items {
            let child = stack.first_child().expect("Failed to get first child from CardStack");
            let image = child.downcast::<gtk::Image>().expect("Child is not a gtk::Image (dissolve)");
            stack.remove(&image);
            self.add_card(&image, image.height());
        }
    }

    pub fn add_card(&self, card_image: &gtk::Image, height: i32) {
        // Only add the image if it doesn't already have a parent
        if card_image.parent().is_none() {
            // Calculate vertical offset based on the current number of children
            let child_count = self.observe_children().n_items();
            let offset = (child_count as i32 * height) as f64 * 0.5;
            println!("Adding card at offset: {}", offset);
            // Add the card to the stack
            self.put(card_image, 0.0, offset);

        } else {
            // If the image already has a parent, log a warning
            eprintln!("Warning: Attempted to add a widget that already has a parent");
        }
    }
    
    pub fn dissolve_to_row(self, grid: &gtk::Grid, row: i32) {
        let items = self.observe_children().n_items();
        for i in 0..items {
            let child = self.first_child().expect("Failed to get first child from CardStack");
            let image = child.downcast::<gtk::Image>().expect("Child is not a gtk::Image (dissolve)");
            self.remove(&image);
            grid.attach(&image, i as i32, row, 1, 1);
            image.set_height_request(1);
        }
        grid.remove(&self);
        self.unrealize();
    }
    
    pub fn focus_card(&self, card_name: &str) {
        self.get_card(card_name).expect("Couldn't get card").grab_focus();
    }
    
    pub fn col(&self) -> u8 {
        self.imp().col
    }
    
    pub fn row(&self) -> u8 {
        self.imp().row
    }
}