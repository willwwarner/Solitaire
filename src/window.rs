/* window.rs
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

use gettextrs::gettext;
use gtk::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use glib::subclass::InitializingObject;
use crate::games;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
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
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SolitaireWindow {
        const NAME: &'static str = "SolitaireWindow";
        type Type = super::SolitaireWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
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
            obj.add_cards();
            obj.populate_game_list(&obj.imp().list.get());
            obj.imp().search_bar.connect_entry(&obj.imp().search_entry.get());
            crate::runtime::set_grid(self.card_grid.get());
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

    pub fn add_cards(&self) {
        let game_board = &self.imp().card_grid.get();
        
        let mut cards_to_add:u8 = 52; // The number of gtk Pictures (cards) to add to the grid, a standard deck has 52 cards

        while cards_to_add > 0 {
            let picture = gtk::Picture::new();

            let suite_index = ((cards_to_add - 1) / 13) as usize;
            let rank_index = ((cards_to_add - 1) % 13) as usize;
            let card_name = format!("{}_{}", games::SUITES[suite_index], games::RANKS[rank_index]);

            picture.set_widget_name(card_name.as_str());
            picture.set_property("sensitive", true);
            game_board.attach(&picture, rank_index as i32, suite_index as i32, 1, 1);

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
        let settings = gio::Settings::new(crate::APP_ID);
        games::load_game(&settings.get::<String>("recent-game"), &self.imp().card_grid.get());
        self.imp().nav_view.get().push_by_tag("game");
    }

    #[template_callback]
    fn populate_game_list(&self, list: &gtk::ListBox) {
        println!("Populating game list!");
        let not_played_text = gettext("You haven't played this yet");
        for game in &games::get_games() {
            let action_row = adw::ActionRow::new();
            let icon = gtk::Image::new();
            icon.set_icon_name(Some("go-next-symbolic"));
            icon.set_valign(gtk::Align::Center);
            action_row.set_activatable(true);
            action_row.set_property("title", gettext(game));
            action_row.set_property("subtitle", &not_played_text);
            action_row.add_suffix(&icon);
            let nav_view = self.imp().nav_view.get();
            let card_grid = self.imp().card_grid.get();
            let game_name = game.clone();
            action_row.connect_activated(move |_| {
                glib::g_message!("solitaire", "Starting {game_name}!");
                games::load_game(&*game_name, &card_grid);
                nav_view.push_by_tag("game");
                glib::g_message!("solitaire", "pushed to game");
            });
            list.append(&action_row);
        }
    }
    
    #[template_callback]
    fn new_game_clicked(&self, _button: &gtk::Button) {
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Do you want to start a new game?"))
            .body(gettext("If you start a new game, your current progress will be lost."))
            .default_response("delete_event")
            .build();
        dialog.add_responses(&[
            ("accept",          gettext("Start New Game").as_str()),
            ("delete_event",    gettext("Keep Current Game").as_str())
        ]);
        let nav_view = self.imp().nav_view.get();
        let grid = self.imp().card_grid.get();
        dialog.connect_response(Some("accept"), move |_dialog, _response| {
            println!("Going to game chooser!");
            games::unload(&grid);
            nav_view.pop_to_tag("chooser");
        });
        dialog.connect_response(Some("delete_event"), |_dialog, _response| {
            println!("Keeping current game!");
        });
        dialog.set_response_appearance("accept", adw::ResponseAppearance::Destructive);
        dialog.present(Some(self));
    }
}