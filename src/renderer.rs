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

use gtk::gdk::*;

thread_local! {
    pub static BACK_TEXTURE: std::cell::RefCell<Option<MemoryTexture>> = std::cell::RefCell::new(None);
}

pub const ASPECT:f32 = 1.4;

pub fn draw_card(name: &str, renderer: &rsvg::CairoRenderer, width: i32, height: i32) -> MemoryTexture {
    let surface = cairo::ImageSurface::
        create(cairo::Format::ARgb32, width, height)
        .expect("Couldn't create surface");

    let cr = cairo::Context::new(&surface).expect("Couldn't create cairo context");
    // Render a single SVG layer, marked by a <g>
    renderer
        .render_element(&cr, Some(&format!("#{name}")), &cairo::Rectangle::new(0.0, 0.0, 250.0, 360.0))
        .expect(&format!("Failed to render layer {name}"));

    drop(cr);
    let stride = surface.stride() as usize;
    let data = surface.take_data().expect("Failed to get data from surface");
    // Create a texture from the surface
    let bytes = glib::Bytes::from(&data[..]);
    MemoryTexture::new(
        width,
        height,
        MemoryFormat::B8g8r8a8Premultiplied, // Match ARGB32 surface
        &bytes,
        stride,
    )
}

pub fn set_back_texture(renderer: &rsvg::CairoRenderer) {
    let texture = draw_card("back", renderer, 250, 350); //TODO: More card sizes
    BACK_TEXTURE.with(|t| { t.borrow_mut().replace(texture) });
}