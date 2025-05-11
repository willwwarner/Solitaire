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
use std::sync::atomic::AtomicI32;
use adw::subclass::prelude::*;
use gtk::prelude::*;
use cairo::Context;
use gtk::gdk::*;
use rsvg::CairoRenderer;
use crate::card_stack::*;

static GAME_HEIGHT: AtomicI32 = AtomicI32::new(0);
static GAME_WIDTH: AtomicI32 = AtomicI32::new(0);

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
    // Set the image using the new method
    image.set_paintable(Some(texture.upcast_ref::<Paintable>()));
}

fn update_geometry(grid: &gtk::Grid, height: i32, width: i32) {
    eprintln!("Total height: {}, total width: {}", height, width);
    let height_units = 6 + 1;
    let card_height = height / height_units;
    //let is_width_restricted = find_size_restrictions(total_height, total_width);
    let stack_height = card_height * 3; // This seems to be the standard
    let num_cols = 7;
    let num_rows = 2;
    let stacks = grid.observe_children();
    for object in &stacks {
        let object = object.expect("Couldn't get object");
        if object.type_() == CardStack::static_type() {
            let stack = object.downcast::<CardStack>().expect("Couldn't downcast to stack");
            stack.imp().size_allocate((stack_height as f64 * 0.6) as i32, stack_height, 0);
        } else {
            continue;
        }
    }
}

pub fn setup_resize(card_grid: &gtk::Grid) {
    card_grid.add_tick_callback(|grid, _frame| {
        let mut do_update = false;
        let height = grid.height();
        let width = grid.width();
        let previous_height = GAME_HEIGHT.load(std::sync::atomic::Ordering::Acquire);
        let previous_width = GAME_WIDTH.load(std::sync::atomic::Ordering::Acquire);
        if height != previous_height {
            println!("Height: {}, Width: {}", height, width);
            GAME_HEIGHT.store(height, std::sync::atomic::Ordering::Release);
            do_update = true;
        }
        if width != previous_width {
            println!("Height: {}, Width: {}", height, width);
            GAME_WIDTH.store(width, std::sync::atomic::Ordering::Release);
            do_update = true;
        }
        if do_update {
            update_geometry(&grid, height, width);
        }
        glib::ControlFlow::Continue
    });
}