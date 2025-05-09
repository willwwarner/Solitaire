/* renderer.rs
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

use gtk::prelude::*;
use cairo::Context;
use gtk::gdk::*;
use rsvg::CairoRenderer;
pub const ASPECT:f32 = 1.4;
pub fn draw_image(image: &gtk::Image, name: &str, renderer: &CairoRenderer) {
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
    let texture = MemoryTexture::new(
        250,
        350,
        MemoryFormat::B8g8r8a8Premultiplied, // Match ARGB32 surface
        &bytes,
        stride,
    );
    // Set the images size
    image.set_size_request(250, 350);
    // Set the image using the new method
    image.set_paintable(Some(texture.upcast_ref::<Paintable>()));
}

fn update_geometry () {
    
}
pub fn resize () {
    update_geometry();
}