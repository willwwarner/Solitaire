/* game_board.rs
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

use gtk::glib;
use gtk::{prelude::*, subclass::prelude::*};
use std::cell::Cell;

glib::wrapper! {
    pub struct AspectGridLayoutChild(ObjectSubclass<imp::AspectGridLayoutChild>)
        @extends gtk::LayoutChild;
}

glib::wrapper! {
    pub struct AspectGridLayout(ObjectSubclass<imp::AspectGridLayout>)
        @extends gtk::LayoutManager;
}

glib::wrapper! {
    pub struct GameBoard(ObjectSubclass<imp::GameBoard>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

mod imp {
    use super::*;

    #[derive(glib::Properties, Default)]
    #[properties(wrapper_type = super::AspectGridLayoutChild)]
    pub struct AspectGridLayoutChild {
        #[property(get, set)]
        column: Cell<f64>,
        #[property(get, set)]
        row: Cell<f64>,
        #[property(get, set)]
        column_span: Cell<f64>,
        #[property(get, set)]
        row_span: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AspectGridLayoutChild {
        const NAME: &'static str = "AspectGridLayoutChild";
        type Type = super::AspectGridLayoutChild;
        type ParentType = gtk::LayoutChild;
    }

    #[glib::derived_properties]
    impl ObjectImpl for AspectGridLayoutChild {}

    impl LayoutChildImpl for AspectGridLayoutChild {}

    #[derive(Default)]
    pub struct AspectGridLayout {
        pub ratio: Cell<f64>,
        pub cols: Cell<f64>,
        pub rows: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AspectGridLayout {
        const NAME: &'static str = "AspectGridLayout";
        type Type = super::AspectGridLayout;
        type ParentType = gtk::LayoutManager;
    }

    impl ObjectImpl for AspectGridLayout {}

    impl LayoutManagerImpl for AspectGridLayout {
        fn allocate(&self, widget: &gtk::Widget, width: i32, height: i32, _baseline: i32) {
            let mut child_widget = match widget.first_child() {
                Some(child) => child,
                None => return,
            };
            let col_width1;
            let row_height1;
            let row_height2;
            let col_width2;
            let row_constrained;
            let split;
            let h_offset;
            {
                let width_f = width as f64;
                let height_f = height as f64;
                let ratio_real = height_f / width_f;
                let card_aspect = crate::renderer::ASPECT.get() as f64;
                row_constrained = ratio_real < self.ratio.get() * card_aspect;
                if row_constrained {
                    let rows = self.rows.get();
                    row_height1 = (height_f / rows).floor();
                    split = height_f as i32 % rows as i32;
                    row_height2 = row_height1 + 1.0;
                    col_width1 = (row_height1 / card_aspect).floor();
                    col_width2 = (row_height2 / card_aspect).floor();
                    h_offset = (width_f - (col_width1 * self.cols.get())) as i32 / 2;
                } else {
                    let cols = self.cols.get();
                    col_width1 = (width_f / cols).floor();
                    col_width2 = col_width1 + 1.0;
                    split = width_f as i32 % cols as i32;
                    row_height1 = (col_width1 * card_aspect).floor();
                    row_height2 = (col_width2 * card_aspect).floor();
                    h_offset = 0;
                }
            }
            loop {
                let layout_child = self.obj().layout_child(&child_widget).downcast::<super::AspectGridLayoutChild>().unwrap();

                let n_constrained = if row_constrained { layout_child.row() } else { layout_child.column() } as i32;
                let allocation = if n_constrained < split {
                    gtk::Allocation::new((layout_child.column() * col_width2) as i32 + h_offset,
                                         (layout_child.row() * row_height2) as i32,
                                         (layout_child.column_span() * col_width2) as i32,
                                         (layout_child.row_span() * row_height2) as i32)
                } else {
                    let h_mod = {
                        if row_constrained { h_offset }
                        else { h_offset + split }
                    };
                    gtk::Allocation::new((layout_child.column() * col_width1) as i32 + h_mod,
                                         (layout_child.row() * row_height1) as i32,
                                         (layout_child.column_span() * col_width1) as i32,
                                         (layout_child.row_span() * row_height1) as i32)
                };

                child_widget.size_allocate(&allocation, -1);
                if let Some(next_child) = child_widget.next_sibling() {
                    child_widget = next_child;
                } else {
                    break;
                }
            }
        }

        fn layout_child_type() -> Option<glib::Type> {
            Some(super::AspectGridLayoutChild::static_type())
        }

        fn measure(&self, _widget: &gtk::Widget, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let card_aspect = crate::renderer::ASPECT.get() as f64;
            let nat_size;
            if for_size == -1 {
                nat_size = -1;
            } else if orientation == gtk::Orientation::Horizontal {
                nat_size = (for_size as f64 / (self.ratio.get() * card_aspect)) as i32;
            } else {
                nat_size = (for_size as f64 * (self.ratio.get() * card_aspect)) as i32;
            }

            (-1, nat_size, -1, -1)
        }
    }

    #[derive(Default)]
    pub struct GameBoard {}

    #[glib::object_subclass]
    impl ObjectSubclass for GameBoard {
        const NAME: &'static str = "GameBoard";
        type Type = super::GameBoard;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<super::AspectGridLayout>();
        }
    }

    impl ObjectImpl for GameBoard {}
    impl WidgetImpl for GameBoard {
        fn unrealize(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
                child.unrealize();
            }
            self.parent_unrealize();
        }
    }
}

impl GameBoard {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn add(&self, widget: &impl IsA<gtk::Widget>, column: i32, row: i32, width: i32, height: i32) {
        if widget.parent().is_some() { return; }
        widget.set_parent(&self.clone().upcast::<gtk::Widget>());
        let layout_child = self.layout_manager().unwrap().layout_child(widget).downcast::<AspectGridLayoutChild>().unwrap();
        layout_child.set_column(column as f64);
        layout_child.set_row(row as f64);
        layout_child.set_column_span(width as f64);
        layout_child.set_row_span(height as f64);
        self.recalculate_layout(widget);
    }

    pub fn add_float(&self, widget: &impl IsA<gtk::Widget>, column: f64, row: f64, width: f64, height: f64) {
        widget.set_parent(&self.clone().upcast::<gtk::Widget>());
        let layout_child = self.layout_manager().unwrap().layout_child(widget).downcast::<AspectGridLayoutChild>().unwrap();
        layout_child.set_column(column);
        layout_child.set_row(row);
        layout_child.set_column_span(width);
        layout_child.set_row_span(height);
        self.recalculate_layout(widget);
    }

    pub fn recalculate_layout(&self, new_widget: &impl IsA<gtk::Widget>) {
        fn set_if_greater(lesser: &Cell<f64>, greater: f64) -> bool {
            let set = lesser.get() < greater;
            if set {
                lesser.set(greater);
            }
            set
        }

        let layout = self
            .layout_manager()
            .unwrap()
            .downcast::<AspectGridLayout>()
            .unwrap();
        let imp = layout.imp();
        let layout_child = layout.layout_child(new_widget).downcast::<AspectGridLayoutChild>().unwrap();
        if set_if_greater(&imp.rows, layout_child.row() + layout_child.row_span()) ||
           set_if_greater(&imp.cols, layout_child.column() + layout_child.column_span()) {
            imp.ratio.set(imp.rows.get() / imp.cols.get());
        }
    }

    pub fn reset_layout(&self) {
        let layout = self
            .layout_manager()
            .unwrap()
            .downcast::<AspectGridLayout>()
            .unwrap();
        let imp = layout.imp();
        imp.cols.set(0.0);
        imp.rows.set(0.0);
        imp.ratio.set(0.0);
    }
}
