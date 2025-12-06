/* card.rs
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

use crate::{games, renderer, card_stack::CardStack};
use gtk::{glib, gdk};
use gtk::prelude::{Cast, WidgetExt};
use adw::{prelude::*, subclass::prelude::*};
use std::cell::Cell;

glib::wrapper! {
    pub struct Card(ObjectSubclass<imp::Card>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Card {
        pub texture: Cell<Option<gdk::MemoryTexture>>,
        pub is_face_up: Cell<bool>,
        pub card_id: Cell<u8>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Card {
        const NAME: &'static str = "Card";
        type Type = super::Card;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for Card {}
    impl WidgetImpl for Card {}
    impl BinImpl for Card {}
}

impl Card {
    pub fn new(name: &str, id: i32, renderer: &rsvg::CairoRenderer, card_theme: &renderer::CardTheme) -> Self {
        let this:Card = glib::Object::new();
        this.set_sensitive(true);
        this.set_can_focus(true);
        this.set_widget_name(name);
        this.imp().card_id.set(id as u8);
        let picture = gtk::Picture::new();
        let texture = renderer::draw_card(name, renderer, &card_theme, id % 13, id / 13);
        picture.set_paintable(Some(&texture));
        this.set_child(Some(&picture));
        this.imp().texture.set(Some(texture));
        this.imp().is_face_up.set(true);
        this
    }

    pub fn flip(&self) {
        let is_face_up = self.imp().is_face_up.get();
        let picture = self.child().unwrap().downcast::<gtk::Picture>().unwrap();
        if is_face_up {
            if let Some(back_texture) = renderer::BACK_TEXTURE.with(|t| { t.borrow().to_owned() }) {
                picture.set_paintable(Some(&back_texture));
            } else { glib::g_critical!("solitaire", "Tried to flip a card with no back texture"); }
        } else {
            if let Some(face_texture) = self.imp().texture.take() {
                picture.set_paintable(Some(&face_texture));
                self.imp().texture.set(Some(face_texture));
            } else { glib::g_critical!("solitaire", "Tried to flip a card with no face texture"); }
        }
        self.imp().is_face_up.set(!is_face_up);
    }

    pub fn flip_to_face(&self) {
        let is_face_up = self.imp().is_face_up.get();
        let picture = self.child().unwrap().downcast::<gtk::Picture>().unwrap();
        if !is_face_up {
            if let Some(face_texture) = self.imp().texture.take() {
                picture.set_paintable(Some(&face_texture));
                self.imp().texture.set(Some(face_texture));
            }
            self.imp().is_face_up.set(true);
        }
    }

    pub fn flip_to_back(&self) {
        let is_face_up = self.imp().is_face_up.get();
        let picture = self.child().unwrap().downcast::<gtk::Picture>().unwrap();
        if is_face_up {
            if let Some(back_texture) = renderer::BACK_TEXTURE.with(|t| { t.borrow().to_owned() }) {
                picture.set_paintable(Some(&back_texture));
            }
            self.imp().is_face_up.set(false);
        }
    }

    pub fn is_one_rank_above(&self, lower_card: &Card) -> bool {
        let self_rank = self.imp().card_id.get() % 13;
        let lower_rank = lower_card.imp().card_id.get() % 13;
        (lower_rank + 1) == self_rank
    }

    pub fn is_same_suit(&self, other_card: &Card) -> bool {
        (self.imp().card_id.get() / 13) == (other_card.imp().card_id.get() / 13)
    }

    pub fn is_similar_suit(&self, other_card: &Card) -> bool {
        let self_suit = self.imp().card_id.get() / 13;
        let other_suit = other_card.imp().card_id.get() / 13;
        (self_suit == 0 || self_suit == 3) == (other_suit == 0 || other_suit == 3)
    }

    pub fn get_rank(&self) -> &str {
        let rank = self.imp().card_id.get() % 13;
        games::RANKS[rank as usize]
    }
    
    pub fn get_stack(&self) -> Option<CardStack> {
        self.parent()?.downcast::<CardStack>().ok()
    }

    pub fn is_face_up(&self) -> bool {
        self.imp().is_face_up.get()
    }
}