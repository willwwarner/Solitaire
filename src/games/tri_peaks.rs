/* tri_peaks.rs
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

use super::*;
use crate::{
    card::Card, card_stack::CardStack, game_board::GameBoard, runtime, runtime::MoveInstruction,
    window,
};
use gtk::glib;
use gtk::{prelude::*, subclass::prelude::*};

const LEVEL_SIZES: [i32; 4] = [3, 6, 9, 10];

pub struct TriPeaks {}

impl TriPeaks {}

impl Game for TriPeaks {
    fn new_game(mut cards: Vec<Card>, game_board: &GameBoard) -> Self {
        let mut n_cards = cards.len() as i32;
        let mut add_pyramid = |col: f64, row: f64, flip: bool| {
            let card_stack = CardStack::new("pyramid", 52 - n_cards, false);
            let random_card = glib::random_int_range(0, n_cards) as usize;
            let card = &cards[random_card];
            card_stack.add_card(&card);
            if flip {
                card.flip()
            }
            card_stack.add_drag_to_card(&card);
            runtime::connect_double_click(&card);
            cards.remove(random_card);
            n_cards -= 1;

            game_board.add_float(&card_stack, col, row, 1.0, 1.0);
        };
        for i in 1..4 {
            add_pyramid((3.0 * (i as f64)) - 1.5, 1.0, true);
        }
        for i in 0..3 {
            let offset = (i as f64) * 3.0;
            for j in 1..3 {
                add_pyramid(offset + (j as f64), 1.5, true);
            }
        }
        for i in 1..10 {
            add_pyramid((i as f64) - 0.5, 2.0, true);
        }
        for i in 0..10 {
            add_pyramid(i as f64, 2.5, false);
        }

        let waste = CardStack::new("waste", -1, false);
        waste.enable_drop();
        game_board.add(&waste, 1, 0, 1, 1);

        let stock = CardStack::new("stock", -1, false);
        stock.add_click_to_slot();
        while n_cards > 0 {
            let random_card = glib::random_int_range(0, n_cards) as usize;
            let card = &cards[random_card];
            stock.add_card(&card);
            card.flip();
            runtime::connect_double_click(&card);
            cards.remove(random_card);
            n_cards -= 1;
        }
        game_board.add(&stock, 0, 0, 1, 1);

        Self {}
    }
    fn verify_drag(&self, bottom_card: &Card, _from_stack: &CardStack) -> bool {
        if !bottom_card.imp().is_face_up.get() {
            false
        } else {
            true
        }
    }

    fn verify_drop(&self, bottom_card: &Card, to_stack: &CardStack) -> bool {
        let stack_type = to_stack.get_type();
        if stack_type == "waste" {
            if to_stack.is_empty() {
                return false;
            }
            let top_card = to_stack.last_card().unwrap();
            if top_card.is_one_rank_above(&bottom_card) || bottom_card.is_one_rank_above(&top_card)
            {
                return true;
            }
        }
        false
    }

    fn on_drag_completed(
        &self,
        origin_stack: &CardStack,
        _destination_stack: &CardStack,
        move_: &mut runtime::Move,
    ) {
        fn try_flip(stack: &CardStack, above: i32) {
            if above == -1 {
                return;
            }
            if stack.first_card().is_some() {
                return;
            }
            if stack
                .next_sibling()
                .unwrap()
                .downcast::<CardStack>()
                .unwrap()
                .first_card()
                .is_none()
            {
                runtime::get_stack(&format!("pyramid_{above}"))
                    .expect(&format!("tri_peaks: couldn't get pyramid_{above}"))
                    .face_up_top_card();
            }
        }

        if origin_stack.get_type() == "pyramid" {
            window::SolitaireWindow::get_window()
                .unwrap()
                .get_gameboard()
                .send_to_back(origin_stack);
            origin_stack.set_can_target(false); // Force GTK to consider other stacks for dragging
            let num: i32 = origin_stack
                .widget_name()
                .split_once('_')
                .unwrap()
                .1
                .parse()
                .unwrap();

            if let Some(prev) = origin_stack.prev_sibling() {
                let above = get_above(
                    prev.widget_name()
                        .split_once('_')
                        .unwrap()
                        .1
                        .parse()
                        .unwrap(),
                );
                try_flip(&prev.downcast().unwrap(), above);
            }

            if !(num == 27 || num == 17 || num == 8 || num == 6 || num == 4) {
                let above = get_above(num as usize);
                try_flip(origin_stack, above);
            }
        }
    }

    fn pre_undo_drag(
        &self,
        origin_stack: &CardStack,
        _dropped_stack: &CardStack,
        move_: &mut runtime::Move,
    ) {
        fn try_unflip(stack: &CardStack, above: i32) {
            if above == -1 {
                return;
            }
            if stack.first_card().is_some() {
                return;
            }
            if stack
                .next_sibling()
                .unwrap()
                .downcast::<CardStack>()
                .unwrap()
                .first_card()
                .is_none()
            {
                runtime::get_stack(&format!("pyramid_{above}"))
                    .expect(&format!("tri_peaks: couldn't get pyramid_{above}"))
                    .face_down_top_card();
            }
        }
        if origin_stack.get_type() == "pyramid" {
            window::SolitaireWindow::get_window()
                .unwrap()
                .get_gameboard()
                .reset_position(origin_stack);
            origin_stack.set_can_target(true);
            let num: i32 = origin_stack
                .widget_name()
                .split_once('_')
                .unwrap()
                .1
                .parse()
                .unwrap();

            if let Some(prev) = origin_stack.prev_sibling() {
                let above = get_above(
                    prev.widget_name()
                        .split_once('_')
                        .unwrap()
                        .1
                        .parse()
                        .unwrap(),
                );
                try_unflip(&prev.downcast().unwrap(), above);
            }

            if !(num == 27 || num == 17 || num == 8 || num == 6 || num == 4) {
                let above = get_above(num as usize);
                try_unflip(origin_stack, above);
            }
        }
    }

    fn on_double_click(&self, _card: &Card) {}

    fn on_slot_click(&self, slot: &CardStack) {
        if slot.get_type() == "stock" {
            let waste = runtime::get_stack("waste").unwrap();

            if slot.is_empty() {
                return;
            } else {
                let card = slot.last_card().unwrap();
                slot.remove_card(&card);
                card.flip();
                waste.add_card(&card);
                waste.add_drag_to_card(&card);
                card.remove_css_class("highlight");
                runtime::add_to_history(runtime::create_move(
                    &slot.widget_name(),
                    &card.widget_name(),
                    "waste",
                    MoveInstruction::Flip,
                ));
            }
        }
    }

    fn get_move_generator(&self) -> Box<dyn FnMut(&mut solver::State)> {
        Box::new(generate_solver_moves)
    }

    fn get_is_won_fn(&self) -> Box<dyn FnMut(&mut solver::State) -> bool> {
        Box::new(is_won)
    }
}

fn is_won(state: &mut solver::State) -> bool {
    for i in 0..4 {
        if !state.get_stack(i).is_empty() {
            return false;
        }
    }
    true
}

fn get_above(index: usize) -> i32 {
    match index {
        3 => 0,
        5 => 1,
        7 => 2,
        9 => 3,
        10 => 4,
        12 => 5,
        13 => 6,
        15 => 7,
        16 => 8,
        18 => 9,
        19 => 10,
        20 => 11,
        21 => 12,
        22 => 13,
        23 => 14,
        24 => 15,
        25 => 16,
        26 => 17,
        _ => -1,
    }
}

fn generate_solver_moves(state: &mut solver::State) {
    const WASTE: usize = 28;
    fn get_priority(state: &mut solver::State) -> usize {
        let mut outs: usize = 0; // outs = number of cards that are out (in waste)
        for i in 0..28 {
            let pyramid = state.get_stack(i);
            if pyramid.is_empty() {
                outs += 1;
            }
        }
        outs = outs.saturating_sub(state.get_stack(WASTE).len() / 2);
        outs
    }
    fn onmove(move_option: &mut solver::Move, state: &mut solver::State, undo: bool) {
        fn try_flip(state: &mut solver::State, num: usize, above: i32, move_: &mut solver::Move) {
            if above == -1 {
                return;
            }
            let stack = state.get_stack(num);
            if stack.first().is_some() {
                return;
            }
            if state.get_stack(num + 1).is_empty() {
                let above = above as usize;
                solver::flip(state.get_stack_mut(above).last_mut().unwrap());

                if let Some(flip) = move_.flip_index {
                    move_.flip_index = Some(flip + above);
                } else {
                    move_.flip_index = Some(above);
                }
            }
        }

        if move_option.origin_stack < 28 {
            let num = move_option.origin_stack;

            if !(num == 27 || num == 17 || num == 8 || num == 6 || num == 4) {
                let above = get_above(num);
                try_flip(state, num, above, move_option);
            }

            if num != 0 {
                let prev = num - 1;
                let above = get_above(prev);
                try_flip(state, prev, above, move_option);
            }
        }
    }

    let waste = state.get_stack(WASTE).to_owned();
    if !waste.is_empty() {
        for i in 0..28 {
            let pyramid = state.get_stack(i);
            let waste_top = waste.last().unwrap();
            if !pyramid.is_empty() {
                let pyramid_card = pyramid[0];
                if !solver::is_flipped(&pyramid_card) {
                    if solver::is_one_rank_above(&waste_top, &pyramid_card)
                        || solver::is_one_rank_above(&pyramid_card, &waste_top)
                    {
                        // FIXME: check if we flipped any cards, and add that to the move's rank
                        state.try_move(
                            solver::create_move(i, &pyramid_card, WASTE, MoveInstruction::None),
                            5,
                            get_priority,
                            onmove,
                        );
                    }
                }
            }
        }
    }

    let stock = state.get_stack(29);
    if let Some(last) = stock.last() {
        state.try_move(
            solver::create_move(29, &last, WASTE, MoveInstruction::Flip),
            1,
            get_priority,
            solver::no_onmove,
        );
    }
}
