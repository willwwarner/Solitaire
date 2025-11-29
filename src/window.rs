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
use crate::card_stack::CardStack;

thread_local! {
    static SELF: std::cell::RefCell<Option<SolitaireWindow>> = std::cell::RefCell::new(None);
}

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
        pub game_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub game_stack: TemplateChild<gtk::Stack>,
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
        pub new_game_is_safe: std::cell::Cell<bool>,
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
            SELF.set(Some(obj.clone()));
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
                let source_stack = source_stack.downcast::<CardStack>().unwrap();
                source_stack.hint_card(move_.card_name);
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
        for game in games::get_games() {
            let action_row = adw::ActionRow::new();
            let icon = gtk::Image::new();
            icon.set_icon_name(Some("go-next-symbolic"));
            icon.set_valign(gtk::Align::Center);
            action_row.set_activatable(true);
            action_row.set_property("title", &game);
            action_row.set_property("subtitle", games::get_game_description(&game));
            action_row.add_suffix(&icon);
            action_row.connect_activated(move |action_row| {
                let game_name = game.to_owned();
                glib::g_message!("solitaire", "Starting {game_name}");
                let window = Self::get_window().unwrap();
                window.imp().new_game_is_safe.set(false);
                window.imp().nav_view.get().push_by_tag("game");
                window.imp().game_stack.set_visible_child_name("spinner");
                let card_grid = window.imp().card_grid.get();
                #[cfg(debug_assertions)]
                games::test_solver_state();

                window.imp().game_page.set_title(&*game_name);

                // Run the solver async blocking the UI
                glib::spawn_future_local(glib::clone!(
                    #[weak]
                    window,
                    #[weak]
                    action_row,
                    async move {
                        if let Some(solution) = games::try_game(&*game_name, &card_grid).await {
                            window.imp().game_stack.set_visible_child_name("grid");
                            if !solution.is_empty() {
                                window.lookup_action("hint").unwrap().downcast::<gio::SimpleAction>().unwrap().set_enabled(true);
                            }
                            runtime::set_solution(solution);
                            let won_fn = games::get_is_won_fn();
                            runtime::set_won_fn(won_fn);
                        } else {
                            if games::solver::get_should_stop() { return; }
                            let dialog = adw::AlertDialog::builder()
                                .heading(gettext("Failed to make a winnable game"))
                                .body(gettext("Would you like to try again?"))
                                .default_response("accept")
                                .close_response("delete_event")
                                .build();
                            dialog.add_responses(&[
                                ("accept",          gettext("Try Again").as_str()),
                                ("delete_event",    gettext("Go Back").as_str())
                            ]);
                            dialog.set_response_appearance("accept", adw::ResponseAppearance::Suggested);
                            let owned_row = action_row.clone();
                            dialog.connect_response(Some("accept"), move |_dialog, _response| {
                                owned_row.emit_activate();
                            });

                            dialog.connect_response(Some("delete_event"), move |dialog, _response| {
                                dialog.root().unwrap().downcast::<SolitaireWindow>().unwrap().imp().nav_view.pop_to_tag("chooser");
                            });

                            dialog.present(Some(&window));
                        }
                    }
                ));
            });
            list.append(&action_row);
        }
    }
    
    #[template_callback]
    fn new_game_clicked(&self, _button: &gtk::Button) {
        let nav_view = self.imp().nav_view.get();
        let grid = self.imp().card_grid.get();
        if self.imp().new_game_is_safe.get() {
            games::unload(&grid);
            games::solver::set_should_stop(true);
            nav_view.pop_to_tag("chooser");
            return;
        }
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Do you want to start a new game?"))
            .body(gettext("If you start a new game, your current progress will be lost."))
            .default_response("delete_event")
            .build();
        dialog.add_responses(&[
            ("accept",          gettext("Start New Game").as_str()),
            ("delete_event",    gettext("Keep Current Game").as_str())
        ]);

        dialog.connect_response(Some("accept"), move |_dialog, _response| {
            games::unload(&grid);
            games::solver::set_should_stop(true);
            nav_view.pop_to_tag("chooser");
        });
        dialog.set_response_appearance("accept", adw::ResponseAppearance::Destructive);
        dialog.present(Some(self));
    }

    pub fn get_window() -> Option<SolitaireWindow> {
        SELF.with(|window| window.borrow().to_owned())
    }
    
    pub fn incompatible_move_dialog<U: Fn(&adw::AlertDialog, &str) + 'static, K: Fn(&adw::AlertDialog, &str) + 'static>
        (undo_move: U, keep_playing: K) {
        
        let window = Self::get_window().unwrap();
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Game is no longer winnable"))
            .body(gettext("A recent move has made the game impossible to win"))
            .default_response("undo")
            .close_response("delete_event")
            .build();
        dialog.add_responses(&[
            ("delete_event",    &*gettext("Keep Playing")),
            ("undo",            &*gettext("Undo Move"))
        ]);
        dialog.set_response_appearance("delete_event", adw::ResponseAppearance::Destructive);
        dialog.connect_response(Some("undo"), undo_move);
        dialog.connect_response(Some("delete_event"), keep_playing);
        dialog.present(Some(&window));
    }

    pub fn won_dialog(&self) {
        self.imp().new_game_is_safe.set(true);
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("You have won"))
            .body(gettext("Congratulations, you have solved the game"))
            .default_response("new_game")
            .build();
        dialog.add_responses(&[
            ("new_game",        &*gettext("New Game")),
            ("delete_event",    &*gettext("Keep Playing"))
        ]);
        dialog.set_response_appearance("new_game", adw::ResponseAppearance::Suggested);
        let nav_view = self.imp().nav_view.get();
        let grid = self.imp().card_grid.get();
        dialog.connect_response(Some("new_game"), move |_dialog, _response| {
            games::unload(&grid);
            games::solver::set_should_stop(true);
            nav_view.pop_to_tag("chooser");
        });
        dialog.present(Some(self));
    }
}
