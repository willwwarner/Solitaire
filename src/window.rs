/* window.rs
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
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use rsvg::{Loader, SvgHandle};
use crate::renderer;
use crate::games;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/shbozz/Solitaire/window.ui")]
    pub struct SolitaireWindow {
        // Template widgets
        #[template_child]
        pub nav_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub recent_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub klondike_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub spider_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub freecell_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub nav_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub card_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SolitaireWindow {
        const NAME: &'static str = "SolitaireWindow";
        type Type = super::SolitaireWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            <crate::SolitaireWindow as gtk::subclass::widget::CompositeTemplateCallbacks>::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SolitaireWindow {}
    impl WidgetImpl for SolitaireWindow {}
    impl WindowImpl for SolitaireWindow {}
    impl ApplicationWindowImpl for SolitaireWindow {}
    impl AdwApplicationWindowImpl for SolitaireWindow {}
}

glib::wrapper! {
    pub struct SolitaireWindow(ObjectSubclass<imp::SolitaireWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,        @implements gio::ActionGroup, gio::ActionMap;
}

#[gtk::template_callbacks]
impl SolitaireWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    #[template_callback]
    fn recent_clicked(&self, _row: &adw::ActionRow) {
        println!("Starting Klondike!");
        games::load_game("klondike");
        self.imp().nav_view.get().push_by_tag("game");
    }
    #[template_callback]
    fn draw_init_internal(&self, card_box: &gtk::Box) {
        println!("Drawing cards!");
        let resource = gio::resources_lookup_data("/io/github/shbozz/Solitaire/assets/minimum_dark.svg", gio::ResourceLookupFlags::NONE)
            .expect("Failed to load resource data");
        let handle = Loader::new().read_stream(&gio::MemoryInputStream::from_bytes(&resource), None::<&gio::File>, None::<&gio::Cancellable>).expect("Failed to load SVG"); 
        let renderer = rsvg::CairoRenderer::new(&handle); // We need to hand this out to the rendering functions
        let mut cards_to_add:u8 = 52; // This is the amount of gtk::images (cards) to add to the box, of course a standard deck has 52 cards

        while cards_to_add > 0 {
            let image = gtk::Image::new();

            let suite_index = ((cards_to_add - 1) / 13) as usize;
            let rank_index = ((cards_to_add - 1) % 13) as usize;
            let card_name = format!("{}_{}", games::SUITES[suite_index], games::RANKS[rank_index]);

            println!("Adding {}", &card_name);
            image.set_widget_name(card_name.as_str());
            card_box.append(&image);
            renderer::draw_image(&image, &card_name, &renderer);

            cards_to_add -= 1;
        }
    }
    pub fn draw_init(&self) {
        let card_box = self.imp().card_box.get();
        println!("Drawing cards!");
        let resource = gio::resources_lookup_data("/io/github/shbozz/Solitaire/assets/minimum_dark.svg", gio::ResourceLookupFlags::NONE)
            .expect("Failed to load resource data");
        let handle = Loader::new().read_stream(&gio::MemoryInputStream::from_bytes(&resource), None::<&gio::File>, None::<&gio::Cancellable>).expect("Failed to load SVG");
        let renderer = rsvg::CairoRenderer::new(&handle); // We need to hand this out to the rendering functions
        let mut cards_to_add:u8 = 52; // This is the amount of gtk::images (cards) to add to the box, of course a standard deck has 52 cards

        while cards_to_add > 0 {
            let image = gtk::Image::new();

            let suite_index = ((cards_to_add - 1) / 13) as usize;
            let rank_index = ((cards_to_add - 1) % 13) as usize;
            let card_name = format!("{}_{}", games::SUITES[suite_index], games::RANKS[rank_index]);

            println!("Adding {}", &card_name);
            image.set_widget_name(card_name.as_str());
            card_box.append(&image);
            renderer::draw_image(&image, &card_name, &renderer);

            cards_to_add -= 1;
        }
    }
}
