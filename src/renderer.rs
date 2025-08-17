/* renderer.rs
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

use adw::prelude::*;
use gtk::gdk::*;
use crate::games;

thread_local! {
    static TEXTURES: std::cell::RefCell<Vec<MemoryTexture>> = std::cell::RefCell::new(Vec::new());
    static BACK_TEXTURE: std::cell::RefCell<Option<MemoryTexture>> = std::cell::RefCell::new(None);
}

pub const ASPECT:f32 = 1.4;

pub fn draw_card(name: &str, renderer: &rsvg::CairoRenderer) -> MemoryTexture {
    let surface = cairo::ImageSurface::
        create(cairo::Format::ARgb32, 250, 350)
        .expect("Couldn't create surface");

    let cr = cairo::Context::new(&surface).expect("Couldn't create cairo context");
    // Render a single SVG layer, marked by a <g>
    renderer
        .render_element(&cr, Some(&format!("#{name}")), &cairo::Rectangle::new(0.0, 0.0, 250.0, 350.0))
        .expect(&format!("Failed to render layer {name}"));

    drop(cr);
    let stride = surface.stride() as usize;
    let data = surface.take_data().expect("Failed to get data from surface");
    // Create a texture from the surface
    let bytes = glib::Bytes::from(&data[..]);
    MemoryTexture::new(
        250,
        350,
        MemoryFormat::B8g8r8a8Premultiplied, // Match ARGB32 surface
        &bytes,
        stride,
    )
}

pub fn set_and_return_texture(name: &str, renderer: &rsvg::CairoRenderer) -> MemoryTexture {
    let texture = draw_card(name, renderer);
    TEXTURES.with(|t| { t.borrow_mut().push(texture.to_owned()) });
    texture
}

pub fn set_back_texture(renderer: &rsvg::CairoRenderer) {
    let texture = draw_card("back", renderer);
    BACK_TEXTURE.with(|t| { t.borrow_mut().replace(texture) });
}

fn get_texture_index(name: &str) -> usize {
    let mut name_parts = name.split('_');
    let suit = name_parts.next().unwrap();
    let rank = name_parts.next().unwrap();
    let suit_index = games::SUITES.iter().position(|x| x == &suit).unwrap();
    let rank_index = games::RANKS.iter().position(|x| x == &rank).unwrap();

    (suit_index * 13) + rank_index
}

pub fn flip_card(card: &gtk::Picture) {
    // It's pretty simple, the state is stored in the widget name
    let current_name = card.widget_name();
    if current_name.ends_with("_b") {
        card.set_widget_name(&current_name.replace("_b", ""));
        let texture = TEXTURES.with(|t| { t.borrow_mut().get(get_texture_index(&card.widget_name())).unwrap().to_owned() });
        card.set_paintable(Some(texture.upcast_ref::<Paintable>()));
    }
    else {
        card.set_widget_name((current_name.to_string() + "_b").as_str());
        let texture = BACK_TEXTURE.with(|t| { t.borrow().to_owned().unwrap() });
        card.set_paintable(Some(texture.upcast_ref::<Paintable>()));
    }
}

pub fn flip_to_face(card: &gtk::Picture) {
    // It's pretty simple, the state is stored in the widget name
    let current_name = card.widget_name();
    if current_name.ends_with("_b") {
        card.set_widget_name(&current_name.replace("_b", ""));
        let texture = TEXTURES.with(|t| { t.borrow().get(get_texture_index(&card.widget_name())).unwrap().to_owned() });
        card.set_paintable(Some(texture.upcast_ref::<Paintable>()));
    }
}

pub fn flip_to_back(card: &gtk::Picture) {
    // It's pretty simple, the state is stored in the widget name
    let current_name = card.widget_name();
    if !current_name.ends_with("_b") {
        card.set_widget_name((current_name.to_string() + "_b").as_str());
        let texture = BACK_TEXTURE.with(|t| { t.borrow().to_owned().unwrap() });
        card.set_paintable(Some(texture.upcast_ref::<Paintable>()));
    }
}

pub fn flip_card_full(card: &gtk::Picture, renderer: &rsvg::CairoRenderer) {
    // It's pretty simple, the state is stored in the widget name
    let current_name = card.widget_name();
    if current_name.ends_with("_b") {
        card.set_widget_name(&current_name.replace("_b", ""));
        let texture = draw_card(&card.widget_name(), &renderer);
        card.set_paintable(Some(texture.upcast_ref::<Paintable>()));
    }
    else {
        card.set_widget_name((current_name.to_string() + "_b").as_str());
        let texture = draw_card("back", &renderer);
        card.set_paintable(Some(texture.upcast_ref::<Paintable>()));
    }
}