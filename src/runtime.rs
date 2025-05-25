/* runtime.rs
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

use std::sync::atomic;

static MOVES: atomic::AtomicU16 = atomic::AtomicU16::new(0);

unsafe extern "C" {
    fn scm_start_game (
        main_func: Option<
        unsafe extern "C" fn(
            closure: *mut std::os::raw::c_void,
            argc: std::os::raw::c_int,
            argv: *mut *mut std::os::raw::c_char,
        )>,
        filename: *const std::os::raw::c_char,
    );
}

#[no_mangle]
pub extern "C" fn get_moves() -> u16 {
    MOVES.load(atomic::Ordering::Acquire)
}