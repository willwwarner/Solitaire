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
use crate::{games, runtime};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/org/gnome/gitlab/wwarner/Solitaire/window.ui")]
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
        #[template_child]
        pub undo: TemplateChild<gtk::Button>,
        #[template_child]
        pub redo: TemplateChild<gtk::Button>,
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
            obj.populate_game_list(&obj.imp().list.get());
            obj.imp().search_bar.connect_entry(&obj.imp().search_entry.get());
            runtime::set_grid(self.card_grid.get());
        }
    }
    impl WidgetImpl for SolitaireWindow {}
    impl WindowImpl for SolitaireWindow {}
    impl ApplicationWindowImpl for SolitaireWindow {}
    impl AdwApplicationWindowImpl for SolitaireWindow {}
}

glib::wrapper! {
    pub struct SolitaireWindow(ObjectSubclass<imp::SolitaireWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup,
                    gtk::Root, gtk::Native, gtk::ShortcutManager,
                    gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl SolitaireWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    fn hint(&self) {
        if let Some(move_) = runtime::get_hint() {
            glib::g_message!("solitaire", "Hint: {:?}", move_);

            let grid = self.imp().card_grid.get();

            // Focus the source stack
            if let Ok(source_stack) = runtime::get_child(&grid, &*move_.origin_stack) {
                let source_stack = source_stack.downcast::<crate::card_stack::CardStack>().unwrap();
                source_stack.focus_card(move_.card_name);
            }
        } else {
            println!("No hints available!");
        }
    }
    
    fn undo(&self) {
        runtime::undo_last_move();
        runtime::update_redo_actions(self);
    }

    fn redo(&self) {
        runtime::redo_first_move();
        runtime::update_redo_actions(self);
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
        for game in &games::get_games() {
            let action_row = adw::ActionRow::new();
            let icon = gtk::Image::new();
            icon.set_icon_name(Some("go-next-symbolic"));
            icon.set_valign(gtk::Align::Center);
            action_row.set_activatable(true);
            action_row.set_property("title", game);
            action_row.set_property("subtitle", games::get_game_description(game));
            action_row.add_suffix(&icon);
            let nav_view = self.imp().nav_view.get();
            let card_grid = self.imp().card_grid.get();
            let game_name = game.clone();
            action_row.connect_activated(move |action_row| {
                glib::g_message!("solitaire", "Starting {game_name}!");
                nav_view.push_by_tag("game");
                games::test_solver_state();
                for i in 0..3 {
                    games::load_game(&*game_name, &card_grid);
                    if let Some(solution) = games::solve_game() {
                        runtime::set_solution(solution);
                        glib::g_message!("solitaire", "pushed to game");
                        return;
                    } else {
                        games::unload(&card_grid);
                        println!("Failed to solve game {i}!");
                    }
                }
                let dialog = adw::AlertDialog::builder()
                    .heading(gettext("Failed to make a winnable game"))
                    .body(gettext("Would you like to try to create a new game?"))
                    .default_response("accept")
                    .build();
                dialog.add_responses(&[
                    ("accept",          gettext("Try Again").as_str()),
                    ("delete_event",    gettext("Go Back").as_str())
                ]);
                dialog.set_response_appearance("accept", adw::ResponseAppearance::Suggested);
                let owned_row = action_row.to_owned();
                dialog.connect_response(Some("accept"), move |_dialog, _response| {
                    owned_row.emit_activate();
                });

                dialog.connect_response(Some("delete_event"), move |dialog, _response| {
                    dialog.root().unwrap().downcast::<SolitaireWindow>().unwrap().imp().nav_view.pop_to_tag("chooser");
                });
                dialog.present(Some(&card_grid.root().unwrap().downcast::<Self>().unwrap()));
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
            games::unload(&grid);
            nav_view.pop_to_tag("chooser");
        });
        dialog.set_response_appearance("accept", adw::ResponseAppearance::Destructive);
        dialog.present(Some(self));
    }
}
