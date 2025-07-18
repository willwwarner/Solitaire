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
use cairo::Context;
use gtk::gdk::*;

pub const ASPECT:f32 = 1.4;
pub fn draw_card(name: &str, renderer: &rsvg::CairoRenderer) -> MemoryTexture {
    let surface = cairo::ImageSurface::
        create(cairo::Format::ARgb32, 250, 350)
        .expect("Couldn't create surface");

    let cr = Context::new(&surface).expect("Couldn't create cairo context");
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

pub fn flip_card(card: &gtk::Picture) {
    // There has to be a better way to do this
    glib::g_message!("solitaire", "Loading SVG");
    let resource = gio::resources_lookup_data("/org/gnome/Solitaire/assets/anglo_poker.svg", gio::ResourceLookupFlags::NONE)
        .expect("Failed to load resource data");
    glib::g_message!("solitaire", "loaded resource data");
    let handle = rsvg::Loader::new()
        .read_stream(&gio::MemoryInputStream::from_bytes(&resource), None::<&gio::File>, None::<&gio::Cancellable>)
        .expect("Failed to load SVG");
    let renderer = rsvg::CairoRenderer::new(&handle);
    glib::g_message!("solitaire", "Done Loading SVG");
    
    flip_card_full(card, &renderer);
}

pub fn flip_to_face(card: &gtk::Picture) {
    // It's pretty simple, the state is stored in the widget name
    let current_name = card.widget_name();
    if current_name.ends_with("_b") {
        // There has to be a better way to do this
        glib::g_message!("solitaire", "Loading SVG");
        let resource = gio::resources_lookup_data("/org/gnome/Solitaire/assets/anglo_poker.svg", gio::ResourceLookupFlags::NONE)
            .expect("Failed to load resource data");
        glib::g_message!("solitaire", "loaded resource data");
        let handle = rsvg::Loader::new()
            .read_stream(&gio::MemoryInputStream::from_bytes(&resource), None::<&gio::File>, None::<&gio::Cancellable>)
            .expect("Failed to load SVG");
        let renderer = rsvg::CairoRenderer::new(&handle);
        glib::g_message!("solitaire", "Done Loading SVG");
    
        card.set_widget_name(&current_name.replace("_b", ""));
        let texture = draw_card(&card.widget_name(), &renderer);
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