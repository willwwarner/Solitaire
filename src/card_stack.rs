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

glib::wrapper! {
    pub struct CardStack(ObjectSubclass<imp::CardStack>)
        @extends gtk::Fixed, gtk::Widget;
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct CardStack {}

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

            // Custom resizing logic
            println!("Resizing CardStack: width = {}, height = {}", width, height);

            // Iterate over the children of the CardStack
            let widget = self.obj();
            let children = widget.observe_children();
            let child_count = children.n_items();

        }
    }
    impl FixedImpl for CardStack {}
}

impl CardStack {
    pub fn new() -> Self {
        glib::Object::new()
    }
    
    pub fn enable_drop(&self) {
        let drop_target = gtk::DropTarget::new(glib::Type::OBJECT, gdk::DragAction::MOVE);
        let stack_clone = self.clone();
        drop_target.connect_drop(move |_, val, _, _| {
            let stack = val.get::<CardStack>().unwrap();
            let children = stack.observe_children();
            let child_count = children.n_items();

            for i in child_count..0 {
                let child = children.item(i).expect("Failed to get child from CardStack");
                let image = child.downcast::<gtk::Image>().unwrap();
                stack.remove(&image);
                stack_clone.add_card(&image);
            }
            true
        });
        self.add_controller(drop_target);
    }
    
    pub fn split_to_new_on(&self, card_name: &str) -> Result<CardStack, glib::Error> {
        // Attempt to locate the child with the given card name
        let children = self.observe_children();
        let total_children = children.n_items();

        // Loop through all the children widgets to find the matching card
        for i in 0..total_children {
            let mut child = children.item(i).expect("Failed to get child from CardStack");
            let image = child.downcast::<gtk::Image>().unwrap();
            if image.widget_name() == card_name {
                let new_stack = CardStack::new();
                for j in i..total_children {
                    child = children.item(j).expect("Failed to get child from CardStack");
                    let image = child.downcast::<gtk::Image>().unwrap();
                    self.remove(&image);
                    new_stack.add_card(&image);
                }
                return Ok(new_stack);
            }
        }

        // If the card is not found, return an error
        Err(glib::Error::new(glib::FileError::Exist, format!("Card named '{}' was not found in the stack.", card_name).as_str()))
    }


    pub fn add_card(&self, card_image: &gtk::Image) {
        // Calculate vertical offset based on the current number of children
        let child_count = self.observe_children().n_items();
        let offset = child_count * 16;

        // Add the card to the stack
        self.put(card_image, 0.0, offset as f64)
    }
}
