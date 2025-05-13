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

use adw::subclass::prelude::*;
use gtk::prelude::*;
use cairo::Context;
use gtk::gdk::*;
use rsvg::CairoRenderer;
use crate::card_stack::*;

// We can't afford to use safe variables here, because we are reading (& writing)
// multiple times a second
static mut GAME_HEIGHT: i32 = 0;
static mut GAME_WIDTH: i32 = 0;

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

fn calculate_card_size_from_grid_size(height: i32, width: i32) -> (i32, i32) {
    let num_cols = 7;
    let num_rows = 2;
    let card_heights_needed = 6; // 6 for tableau + 0 for foundation

    // Calculate maximum card width based on columns and rows
    let max_card_height_by_width = (((width * 90) / 100) / num_cols) as f32 * ASPECT;

    // Determine max card width based on height constraint and aspect ratio
    let max_card_height_by_height = ((height * 90) / 100) / card_heights_needed;

    // Use the more constraining dimension (smaller width)
    let card_height = std::cmp::min(max_card_height_by_width as i32, max_card_height_by_height);
    
    (card_height, (card_height as f32 / ASPECT) as i32)
}
fn update_geometry(grid: &gtk::Grid, height: i32, width: i32) {
    println!("Total height: {}, total width: {}", height, width);
    let (card_height, card_width) = calculate_card_size_from_grid_size(height, width);
    let tableau_row_height = card_height * 3; // Allocate 3 card heights for tableau

    // Log sizing information for debugging
    println!("Window dimensions: {}x{}", width, height);
    println!("Stack height: {}", tableau_row_height);

    // Process each child widget in the grid
    let stacks = grid.observe_children();
    for object in &stacks {
        let object = object.expect("Couldn't get object");
        if object.type_() == CardStack::static_type() {
            let stack = object.downcast::<CardStack>().expect("Couldn't downcast to stack");

            // Apply the calculated dimensions
            stack.imp().size_allocate(card_width, tableau_row_height, 0);
        }
    }
}

pub fn register_resize(card_grid: &gtk::Grid) {
    let window = card_grid.root().expect("Couldn't get window");
    unsafe {
        card_grid.add_tick_callback(move |grid, _frame| {
            let mut do_update = false;
            let height = grid.height();
            let width = window.width();
            if height != GAME_HEIGHT {
                println!("Height: {}, Width: {}", height, width);
                GAME_HEIGHT = height;
                do_update = true;
            }
            if width != GAME_WIDTH {
                println!("Height: {}, Width: {}", height, width);
                GAME_WIDTH = width;
                do_update = true;
            }
            if do_update {
                update_geometry(&grid, height, width);
            }
            glib::ControlFlow::Continue
        });
    }
}