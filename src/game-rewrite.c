/* game-rewrite.c
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

#include <string.h>
#include <unistd.h>
#include <time.h>

#include <libguile.h>

#include <glib.h>

#include "game-rewrite.h"

enum {
    NEW_GAME_LAMBDA,
    BUTTON_PRESSED_LAMBDA,
    BUTTON_RELEASED_LAMBDA,
    BUTTON_CLICKED_LAMBDA,
    BUTTON_DOUBLE_CLICKED_LAMBDA,
    GAME_OVER_LAMBDA,
    WINNING_GAME_LAMBDA,
    HINT_LAMBDA,
    GET_OPTIONS_LAMBDA,
    APPLY_OPTIONS_LAMBDA,
    TIMEOUT_LAMBDA,
    DROPPABLE_LAMBDA,
    DEALABLE_LAMBDA,
    N_LAMBDAS,
    LAST_MANDATORY_LAMBDA = TIMEOUT_LAMBDA
};

static const char lambda_names[] = {
    "new-game\0"
    "button-pressed\0"
    "button-released\0"
    "button-clicked\0"
    "button-double-clicked\0"
    "game-over\0"
    "winning-game\0"
    "hint\0"
    "get-options\0"
    "apply-options\0"
    "timeout\0"
    "droppable\0"
    "dealable\0"
};

static void
scm_start_game (void(* func))
{
    scm_boot_guile (0, nullptr, func, NULL);
}
