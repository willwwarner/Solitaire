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

use crate::{card::Card, games};
use gtk::gdk::*;

thread_local! {
    pub static BACK_TEXTURE: std::cell::RefCell<Option<MemoryTexture>> = std::cell::RefCell::new(None);
    pub static ASPECT:std::cell::Cell<f32> = std::cell::Cell::new(0.0);
    pub static ACTIVE_THEME:std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

pub const THEME_NAMES: [&str; 3] = ["anglo_poker", "minimum", "minimum_dark"];

pub fn get_requested_theme() -> String {
    use gtk::prelude::*;
    let settings = gtk::gio::Settings::new(crate::APP_ID);
    settings.get::<String>("theme")
}

pub fn draw_theme_preview(
    name: &str,
    card_theme: &CardTheme,
    renderer: &rsvg::CairoRenderer,
    picture: &gtk::Picture,
) {
    let width = card_theme.theme_width as i32;
    let height = card_theme.theme_height as i32;
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height)
        .expect("Couldn't create surface");

    let cr = cairo::Context::new(&surface).expect("Couldn't create cairo context");
    // Render a single SVG layer, marked by a <g>
    renderer
        .render_document(
            &cr,
            &cairo::Rectangle::new(0f64, 0f64, card_theme.theme_width, card_theme.theme_height),
        )
        .expect(&format!("Failed to render layer {name}"));

    drop(cr);
    let stride = surface.stride() as usize;
    let data = surface
        .take_data()
        .expect("Failed to get data from surface");
    // Create a texture from the surface
    let bytes = glib::Bytes::from(&data[..]);
    let texture = MemoryTexture::new(
        width,
        height,
        MemoryFormat::B8g8r8a8Premultiplied, // Match ARGB32 surface
        &bytes,
        stride,
    );
    picture.set_paintable(Some(&texture));
}

pub struct CardTheme {
    pub handle: rsvg::SvgHandle,
    card_width: i32,
    card_height: i32,
    theme_width: f64,
    theme_height: f64,
}

pub fn draw_card(
    name: &str,
    renderer: &rsvg::CairoRenderer,
    card_theme: &CardTheme,
    card_x: i32,
    card_y: i32,
) -> MemoryTexture {
    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        card_theme.card_width,
        card_theme.card_height,
    )
    .expect("Couldn't create surface");

    let cr = cairo::Context::new(&surface).expect("Couldn't create cairo context");
    // Render a single SVG layer, marked by a <g>
    renderer
        .render_layer(
            &cr,
            Some(&format!("#{name}")),
            &cairo::Rectangle::new(
                (-card_theme.card_width * card_x) as f64,
                (-card_theme.card_height * card_y) as f64,
                card_theme.theme_width,
                card_theme.theme_height,
            ),
        )
        .expect(&format!("Failed to render layer {name}"));

    drop(cr);
    let stride = surface.stride() as usize;
    let data = surface
        .take_data()
        .expect("Failed to get data from surface");
    // Create a texture from the surface
    let bytes = glib::Bytes::from(&data[..]);
    MemoryTexture::new(
        card_theme.card_width,
        card_theme.card_height,
        MemoryFormat::B8g8r8a8Premultiplied, // Match ARGB32 surface
        &bytes,
        stride,
    )
}

pub fn set_back_texture(renderer: &rsvg::CairoRenderer, card_theme: &CardTheme) {
    let texture = draw_card("back", renderer, &card_theme, 2, 4); //TODO: More card sizes
    BACK_TEXTURE.with(|t| t.borrow_mut().replace(texture));
}

pub fn get_card_theme(theme_name: &str) -> CardTheme {
    let (card_width, card_height, theme_width, theme_height) = match theme_name {
        "anglo_poker" => (241, 337, 3133, 1685),
        "minimum" => (100, 140, 1300, 700),
        "minimum_dark" => (100, 140, 1300, 700),
        _ => panic!("Unknown card theme: {}", theme_name),
    };

    glib::g_message!("solitaire", "Loading SVG");
    let resource = gio::resources_lookup_data(
        &*format!("/org/gnome/gitlab/wwarner/Solitaire/card_themes/{theme_name}.svg"),
        gio::ResourceLookupFlags::NONE,
    )
    .expect("Failed to load resource data");
    glib::g_message!("solitaire", "loaded resource data");
    let handle = rsvg::Loader::new()
        .read_stream(
            &gio::MemoryInputStream::from_bytes(&resource),
            None::<&gio::File>,
            None::<&gio::Cancellable>,
        )
        .expect("Failed to load SVG");
    glib::g_message!("solitaire", "Done Loading SVG");
    CardTheme {
        handle,
        card_width,
        card_height,
        theme_width: theme_width as f64,
        theme_height: theme_height as f64,
    }
}

pub fn create_cards(card_theme: &CardTheme, cards: &mut Vec<Card>) {
    ASPECT.set(card_theme.card_height as f32 / card_theme.card_width as f32);
    let renderer = rsvg::CairoRenderer::new(&card_theme.handle);
    for i in 0..52 {
        let card_name = format!("{}_{}", games::SUITES[i / 13], games::RANKS[i % 13]);
        let card = Card::new(&*card_name, i as i32, &renderer, &card_theme);
        cards.push(card);
    }
    set_back_texture(&renderer, &card_theme);
    glib::g_message!("solitaire", "Done setting textures");
}
