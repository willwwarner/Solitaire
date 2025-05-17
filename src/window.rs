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

use adw::gdk::Paintable;
use gtk::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use glib::subclass::InitializingObject;
use rsvg::Loader;
use crate::card_stack::CardStack;
use crate::renderer;
use crate::games;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/org/gnome/Solitaire/window.ui")]
    pub struct SolitaireWindow {
        // Template widgets
        #[template_child]
        pub nav_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub recent_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub nav_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub card_grid: TemplateChild<gtk::Grid>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SolitaireWindow {
        const NAME: &'static str = "SolitaireWindow";
        type Type = super::SolitaireWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            <crate::SolitaireWindow as CompositeTemplateCallbacks>::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SolitaireWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
        }
    }
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

    pub fn draw_init(&self) {
        let game_board = &self.imp().card_grid.get();
        println!("Drawing cards!");
        let resource = gio::resources_lookup_data("/org/gnome/Solitaire/assets/minimum_dark.svg", gio::ResourceLookupFlags::NONE)
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
            image.set_property("sensitive", true);
            game_board.attach(&image, rank_index as i32, suite_index as i32, 1, 1);
            let texture = renderer::draw_card(&card_name, &renderer);
            image.set_paintable(Some(texture.upcast_ref::<Paintable>()));

            cards_to_add -= 1;
        }
    }
    
    fn hint(&self) {
        println!("Hint!");
    }
    
    fn undo(&self) {
        println!("Undo!");
    }

    fn redo(&self) {
        println!("Redo!");
    }

    fn setup_gactions(&self) {
        let hint_action = gio::ActionEntry::builder("hint")
            .activate(move |win: &Self, _, _| win.hint())
            .build();
        let undo_action = gio::ActionEntry::builder("undo")
            .activate(move |win: &Self, _, _| win.undo())
            .build();
        let redo_action = gio::ActionEntry::builder("redo")
            .activate(move |win: &Self, _, _| win.redo())
            .build();
        self.add_action_entries([hint_action, undo_action, redo_action]);
    }

    #[template_callback]
    fn recent_clicked(&self, _row: &adw::ActionRow) {
        println!("Starting Recent!");
        games::load_recent();
        self.imp().nav_view.get().push_by_tag("game");
    }

    #[template_callback]
    fn populate_game_list(&self, list: &gtk::ListBox) {
        println!("Populating game list!");
        for game in games::GAMES {
            let action_row = adw::ActionRow::new();
            let icon = gtk::Image::new();
            icon.set_icon_name(Some("go-next-symbolic"));
            icon.set_valign(gtk::Align::Center);
            action_row.set_activatable(true);
            action_row.set_property("title", game);
            action_row.set_property("subtitle", "You haven't played this yet");
            action_row.add_suffix(&icon);
            let nav_view = self.imp().nav_view.get();
            let card_grid = self.imp().card_grid.get();
            action_row.connect_activated(move |_| {
                println!("Starting {}!", game);
                let game_id = game.to_lowercase(); 
                games::load_game(game_id.as_str(), &card_grid);
                nav_view.push_by_tag("game");
            });
            list.append(&action_row);
        }
    }
    
    #[template_callback]
    fn new_game_clicked(&self, _button: &gtk::Button) {
        let dialog = adw::AlertDialog::builder()
            .heading("Do you want to start a new game?")
            .body("If you start a new game, your current progress will be lost.")
            .default_response("delete_event")
            .build();
        dialog.add_responses(&[
            ("accept",   "Start New Game"),
            ("delete_event", "Keep Current Game")
        ]);
        let nav_view = self.imp().nav_view.get();
        let grid = self.imp().card_grid.get();
        dialog.connect_response(Some("accept"), move |_dialog, _response| {
            println!("Going to game chooser!");
            games::unload(&grid);
            let items = grid.observe_children().n_items();
            for i in 0..items {
                let child = grid.first_child().expect("Couldn't get child");
                let stack = child.downcast::<CardStack>().expect("Couldn't downcast child");
                stack.dissolve_to_row(&grid, i as i32);
            }
            nav_view.pop_to_tag("chooser");
        });
        dialog.connect_response(Some("delete_event"), |_dialog, _response| {
            println!("Keeping current game!");
        });
        dialog.set_response_appearance("accept", adw::ResponseAppearance::Destructive);
        dialog.present(Some(self));
    }
}
