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

use crate::{card_stack::CardStack, game_board::GameBoard, games, runtime};
use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::prelude::*;
use gtk::{gio, glib};
use lggs::prelude::*;

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
        pub nav_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub game_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub game_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub game_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub search_page: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub empty_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub undo: TemplateChild<gtk::Button>,
        #[template_child]
        pub redo: TemplateChild<gtk::Button>,
        #[template_child]
        pub hint_or_drop: TemplateChild<gtk::Button>,
        #[template_child]
        pub welcome: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub welcome_revealer: TemplateChild<gtk::Revealer>,

        pub can_drop: std::cell::Cell<bool>,
        pub new_game_is_safe: std::cell::Cell<bool>,
        pub good_search: std::cell::Cell<bool>,
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

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SolitaireWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.populate_game_list(&obj.imp().list.get());
            let welcome_revealer = self.welcome_revealer.get();
            self.search_bar
                .get()
                .connect_search_mode_enabled_notify(move |search_bar| {
                    welcome_revealer.set_reveal_child(!search_bar.is_search_mode())
                });
            self.welcome
                .get()
                .set_icon_name(Some(crate::config::APP_ID));
            SELF.set(Some(obj.clone()));
            self.game_bin.get().set_child(Some(&GameBoard::new()));
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

    #[inline]
    pub fn get_gameboard(&self) -> GameBoard {
        self.imp()
            .game_bin
            .get()
            .child()
            .unwrap()
            .downcast()
            .unwrap()
    }

    fn appearance(&self) {
        use crate::renderer;
        let theme_name = renderer::get_requested_theme();
        let settings = gio::Settings::new(crate::APP_ID);
        let change_theme = move |theme_name: &str, widget: &gtk::Widget| {
            let picture = widget.to_owned().downcast::<gtk::Picture>().unwrap();
            let card_theme = renderer::get_card_theme(theme_name);
            let renderer = rsvg::CairoRenderer::new(&card_theme.handle);
            renderer::draw_theme_preview(theme_name, &card_theme, &renderer, &picture);
            settings.set("theme", theme_name.to_string());
            picture.upcast()
        };
        let picture = gtk::Picture::new();
        picture.set_content_fit(gtk::ContentFit::ScaleDown);
        picture.set_margin_start(6);
        picture.set_margin_end(6);

        let theme_dialog =
            lggs::ThemeSelectorDialog::new(&renderer::THEME_NAMES, &theme_name, &picture);
        change_theme(&theme_name, &picture.upcast::<gtk::Widget>());

        theme_dialog.set_content_height(350);
        theme_dialog.set_content_width(600);
        theme_dialog
            .connect_change_theme(move |_, theme_name, widget| change_theme(theme_name, widget));
        theme_dialog.present(Some(self));
    }

    fn drop(&self) {
        runtime::drop();
    }

    fn hint(&self) {
        if let Some(move_) = runtime::get_hint() {
            glib::g_message!("solitaire", "Hint: {:?}", move_);

            let game_board = self.get_gameboard();

            // Focus the source stack
            if let Ok(source_stack) = runtime::get_child(&game_board, &*move_.origin_stack) {
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
        let appearance_action = gio::ActionEntry::builder("appearance")
            .activate(move |win: &Self, _, _| win.appearance())
            .build();
        let drop_action = gio::ActionEntry::builder("drop")
            .activate(move |win: &Self, _, _| win.drop())
            .build();
        let hint_action = gio::ActionEntry::builder("hint")
            .activate(move |win: &Self, _, _| win.hint())
            .build();
        let undo_action = gio::ActionEntry::builder("undo")
            .activate(move |win: &Self, _, _| win.undo())
            .build();
        let redo_action = gio::ActionEntry::builder("redo")
            .activate(move |win: &Self, _, _| win.redo())
            .build();
        self.add_action_entries([
            appearance_action,
            drop_action,
            hint_action,
            undo_action,
            redo_action,
        ]);
    }

    #[template_callback]
    fn recent_clicked(&self, _row: &adw::ActionRow) {
        let settings = gio::Settings::new(crate::APP_ID);
        games::load_game(
            &settings.get::<String>("recent-game"),
            &self.get_gameboard(),
        );
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
                window.set_can_drop(false);
                window.imp().new_game_is_safe.set(false);
                window.imp().nav_view.get().push_by_tag("game");
                window.imp().game_stack.set_visible_child_name("spinner");
                let game_board = window.get_gameboard();
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
                        if let Some(solution) = games::try_game(&*game_name, &game_board).await {
                            window.imp().game_stack.set_visible_child_name("grid");
                            if !solution.is_empty() {
                                window.set_hint_drop_enabled(true);
                            }
                            runtime::set_solution(solution);
                            let won_fn = games::get_is_won_fn();
                            runtime::set_won_fn(won_fn);
                        } else {
                            if games::solver::get_should_stop() {
                                return;
                            }
                            let dialog = adw::AlertDialog::builder()
                                .heading(gettext("Failed to make a winnable game"))
                                .body(gettext("Would you like to try again?"))
                                .default_response("accept")
                                .close_response("delete_event")
                                .build();
                            dialog.add_responses(&[
                                ("accept", gettext("Try Again").as_str()),
                                ("delete_event", gettext("Go Back").as_str()),
                            ]);
                            dialog.set_response_appearance(
                                "accept",
                                adw::ResponseAppearance::Suggested,
                            );
                            let owned_row = action_row.clone();
                            dialog.connect_response(Some("accept"), move |_dialog, _response| {
                                owned_row.emit_activate();
                            });

                            dialog.connect_response(
                                Some("delete_event"),
                                move |dialog, _response| {
                                    dialog
                                        .root()
                                        .unwrap()
                                        .downcast::<SolitaireWindow>()
                                        .unwrap()
                                        .imp()
                                        .nav_view
                                        .pop_to_tag("chooser");
                                },
                            );

                            dialog.present(Some(&window));
                        }
                    }
                ));
            });
            list.append(&action_row);
        }

        let search_entry = self.imp().search_entry.get();
        list.set_filter_func(glib::clone!(
            #[weak(rename_to=this)]
            self,
            #[weak]
            search_entry,
            #[upgrade_or]
            true,
            move |row| {
                let row = row.clone().downcast::<adw::ActionRow>().unwrap();
                let row_text = row.title().to_uppercase();
                let matches = row_text.contains(&search_entry.text().to_uppercase());
                if matches {
                    this.imp().good_search.set(true)
                }
                matches
            }
        ));
        let search_stack = self.imp().search_stack.get();
        let search_page = self.imp().search_page.get();
        let empty_page = self.imp().empty_page.get();
        search_entry.connect_search_changed(glib::clone!(
            #[weak(rename_to=this)]
            self,
            #[weak]
            list,
            move |_| {
                this.imp().good_search.set(false);
                list.invalidate_filter();
                if this.imp().good_search.get() {
                    search_stack.set_visible_child(&search_page);
                } else {
                    search_stack.set_visible_child(&empty_page);
                }
            }
        ));
    }

    #[template_callback]
    fn new_game_clicked(&self, _button: &gtk::Button) {
        let nav_view = self.imp().nav_view.get();
        let game_board = self.get_gameboard();
        if self.imp().new_game_is_safe.get() {
            games::unload(&game_board);
            games::solver::set_should_stop(true);
            nav_view.pop_to_tag("chooser");
            return;
        }
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Do you want to start a new game?"))
            .body(gettext(
                "If you start a new game, your current progress will be lost.",
            ))
            .default_response("delete_event")
            .build();
        dialog.add_responses(&[
            ("accept", gettext("Start New Game").as_str()),
            ("delete_event", gettext("Keep Current Game").as_str()),
        ]);

        dialog.connect_response(Some("accept"), move |_dialog, _response| {
            games::unload(&game_board);
            games::solver::set_should_stop(true);
            nav_view.pop_to_tag("chooser");
        });
        dialog.set_response_appearance("accept", adw::ResponseAppearance::Destructive);
        dialog.present(Some(self));
    }

    pub fn get_window() -> Option<SolitaireWindow> {
        SELF.with(|window| window.borrow().to_owned())
    }

    pub fn incompatible_move_dialog<
        U: Fn(&adw::AlertDialog, &str) + 'static,
        K: Fn(&adw::AlertDialog, &str) + 'static,
    >(
        undo_move: U,
        keep_playing: K,
    ) {
        let window = Self::get_window().unwrap();
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Game is no longer winnable"))
            .body(gettext("A recent move has made the game impossible to win"))
            .default_response("undo")
            .close_response("delete_event")
            .build();
        dialog.add_responses(&[
            ("delete_event", &*gettext("Keep Playing")),
            ("undo", &*gettext("Undo Move")),
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
            ("new_game", &*gettext("New Game")),
            ("delete_event", &*gettext("Keep Playing")),
        ]);
        dialog.set_response_appearance("new_game", adw::ResponseAppearance::Suggested);
        let nav_view = self.imp().nav_view.get();
        let game_board = self.get_gameboard();
        dialog.connect_response(Some("new_game"), move |_dialog, _response| {
            games::unload(&game_board);
            games::solver::set_should_stop(true);
            nav_view.pop_to_tag("chooser");
        });
        dialog.present(Some(self));
    }

    pub fn set_can_drop(&self, can_drop: bool) {
        self.lookup_action("hint")
            .unwrap()
            .downcast::<gio::SimpleAction>()
            .unwrap()
            .set_enabled(!can_drop);
        self.lookup_action("drop")
            .unwrap()
            .downcast::<gio::SimpleAction>()
            .unwrap()
            .set_enabled(can_drop);
        self.imp().can_drop.set(can_drop);
        let hint_or_drop = self.imp().hint_or_drop.get();
        if can_drop {
            hint_or_drop.set_tooltip_text(Some(&*gettext("Drop")));
            hint_or_drop.set_action_name(Some("win.drop"));
            hint_or_drop.set_icon_name("object-select-symbolic");
        } else {
            hint_or_drop.set_tooltip_text(Some(&*gettext("Hint")));
            hint_or_drop.set_action_name(Some("win.hint"));
            hint_or_drop.set_icon_name("lightbulb-symbolic");
        }
    }

    pub fn set_hint_drop_enabled(&self, enabled: bool) {
        self.lookup_action("hint")
            .unwrap()
            .downcast::<gio::SimpleAction>()
            .unwrap()
            .set_enabled((!self.imp().can_drop.get()) && enabled);
        self.lookup_action("drop")
            .unwrap()
            .downcast::<gio::SimpleAction>()
            .unwrap()
            .set_enabled(self.imp().can_drop.get() && enabled);
    }
}
