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
#include <glib/gi18n.h>

#include "game-rewrite.h"

/* Rust functions */
extern guint16 get_moves ();
extern void add_stack ();

enum ArSlotType
{
  AR_SLOT_CHOOSER,
  AR_SLOT_FOUNDATION,
  AR_SLOT_RESERVE,
  AR_SLOT_STOCK,
  AR_SLOT_TABLEAU,
  AR_SLOT_WASTE,
  AR_SLOT_UNKNOWN
};

#define DELAYED_CALLBACK_DELAY (50)

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

struct AisleriotGame
{
  GPtrArray *slots;

  char *game_module;

  GRand *rand;
  GRand *saved_rand;

  guint delayed_call_timeout_id;

  GTimer *timer;

  int timeout;
  char *score;

  /* Game callbacks */
  SCM lambdas[N_LAMBDAS];

  guint click_to_move : 1;
  guint can_undo : 1;
  guint can_redo : 1;
  guint can_deal : 1;
  guint show_score : 1;
  guint features : 3; /* enough bits for ALL_FEATURES */
  guint state : 3; /* enough bits for LAST_GAME_STATE */
  guint had_exception : 1;
  guint paused : 1;
};

typedef enum {
  FEATURE_DROPPABLE     = 1 << 0,
  FEATURE_SCORE_HIDDEN  = 1 << 1,
  FEATURE_DEALABLE      = 1 << 2,
  ALL_FEATURES          = 0x7
} AisleriotGameFeatures;

/* The one and only game */
struct AisleriotGame *app_game;

typedef struct {
  SCM lambda;
  SCM *args;
  gsize n_args;
} CallData;

static SCM
game_scm_call_lambda (void *user_data)
{
  CallData *data = (CallData *) user_data;
  return scm_call_n (data->lambda, data->args, data->n_args);
}

static gboolean
game_scm_call (SCM lambda,
               SCM *args,
               gsize n_args,
               SCM *retval)
{
  CallData data = { lambda, args, n_args };
  GError *error = NULL;
  SCM rv;

  rv = scm_c_catch (SCM_BOOL_T,
                    game_scm_call_lambda, &data,
                    NULL, NULL,
                    NULL, &error);
  if (error) {
    app_game->had_exception = TRUE;

    g_print ("aisleriot: exception in game-scm-call: %s\n", error->message);
    g_error_free (error);

    /* This game is over, but don't count it in the statistics */
    //set_game_state (app_game, GAME_LOADED);

    return FALSE;
  }

  if (retval)
    *retval = rv;

  return TRUE;
}

static void
cscmi_slot_set_cards (SCM cards)
{
  SCM list_el;
  guint8 *data = NULL;
  guint i, n_cards = 0;

  if (scm_is_true (scm_list_p (cards))) {
    for (list_el = cards; list_el != SCM_EOL; list_el = SCM_CDR (list_el)) {
      ++n_cards;
    }

    data = g_alloca (n_cards);
    i = n_cards;

    for (list_el = cards; list_el != SCM_EOL; list_el = SCM_CDR (list_el)) {
      int card = scm_to_int (SCM_CAR (list_el));
      data[--i] = (guint8)card;
    }
  }

  // /* Don't set the new cards if the same cards are already there.
  //  * This saves us lots of updates on undo/redo.
  //  */
  // if (slot->cards->len == n_cards &&
  //     memcmp (slot->cards->data, data, n_cards) == 0)
  //   return;
  //
  // g_byte_array_set_size (slot->cards, 0);
  //
  // aisleriot_game_slot_add_cards (game, slot, data, n_cards);
  g_print ("aisleriot: IDK what to do here");
}

static SCM
cscmi_add_slot (SCM slot_data)
{
  gboolean expanded_down = FALSE;
  gboolean expanded_right = FALSE;
  int expansion_depth = 0;
  enum ArSlotType type = AR_SLOT_UNKNOWN;
  SCM slot_placement, slot_type;

  if (get_moves() == 0) {
    return scm_throw (scm_from_locale_symbol ("aisleriot-invalid-call"),
                      scm_list_1 (scm_from_utf8_string ("Cannot add a new slot after the game has started.")));
  }

#define EQUALS_SYMBOL(string,object) (scm_is_true (scm_equal_p (scm_from_locale_symbol (string), object)))

  slot_placement = SCM_CADDR (slot_data);
  if (EQUALS_SYMBOL ("expanded", SCM_CAR (slot_placement))) {
    expanded_down = TRUE;
  } else if (EQUALS_SYMBOL ("expanded-right", SCM_CAR (slot_placement))) {
    expanded_right = TRUE;
  } else if (EQUALS_SYMBOL ("partially-expanded", SCM_CAR (slot_placement))) {
    expanded_down = TRUE;
    expansion_depth = scm_to_int (SCM_CADDR (slot_placement));
  } else if (EQUALS_SYMBOL ("partially-expanded-right", SCM_CAR (slot_placement))) {
    expanded_right = TRUE;
    expansion_depth = scm_to_int (SCM_CADDR (slot_placement));
  }

#undef CHECK_EXPANSION

  /* 3rd argument is the slot type (optionally) */
  slot_type = SCM_CDDDR (slot_data);
  if (slot_type != SCM_EOL) {
    if (EQUALS_SYMBOL ("chooser", SCM_CAR (slot_type))) {
      type = AR_SLOT_CHOOSER;
    } else if (EQUALS_SYMBOL ("foundation", SCM_CAR (slot_type))) {
      type = AR_SLOT_FOUNDATION;
    } else if (EQUALS_SYMBOL ("reserve", SCM_CAR (slot_type))) {
      type = AR_SLOT_RESERVE;
    } else if (EQUALS_SYMBOL ("stock", SCM_CAR (slot_type))) {
      type = AR_SLOT_STOCK;
    } else if (EQUALS_SYMBOL ("tableau", SCM_CAR (slot_type))) {
      type = AR_SLOT_TABLEAU;
    } else if (EQUALS_SYMBOL ("waste", SCM_CAR (slot_type))) {
      type = AR_SLOT_WASTE;
    }
  }

#undef EQUALS_SYMBOL

  add_stack();
  //
  // slot->id = scm_to_int (SCM_CAR (slot_data));
  // slot->type = type;
  //
  // slot->cards = g_byte_array_sized_new (SLOT_CARDS_N_PREALLOC);
  // slot->exposed = 0;
  // slot->x = scm_to_double (SCM_CAR (SCM_CADR (SCM_CADDR (slot_data))));
  // slot->y = scm_to_double (SCM_CADR (SCM_CADR (SCM_CADDR (slot_data))));
  //
  // slot->expansion_depth = expansion_depth;
  //
  // slot->expansion.dx = 0.0;
  // slot->expanded_down = expanded_down != FALSE;
  // slot->expanded_right = expanded_right != FALSE;
  //
  // slot->card_images = g_ptr_array_sized_new (SLOT_CARDS_N_PREALLOC);
  //
  // slot->needs_update = TRUE;
  //
  // /* this will update the slot length too */
  // cscmi_slot_set_cards (slot, SCM_CADR (slot_data));

  return SCM_EOL;
}

/* Scheme functions */

static SCM
scm_undo_set_sensitive (SCM in_state)
{
  struct AisleriotGame *game = app_game;
  gboolean state;

  state = scm_is_true (in_state) ? TRUE : FALSE;
  set_game_undoable (game, state);

  return SCM_EOL;
}

static SCM
scm_redo_set_sensitive (SCM in_state)
{
  struct AisleriotGame *game = app_game;
  gboolean state;

  state = scm_is_true (in_state) ? TRUE : FALSE;
  set_game_redoable (game, state);

  return SCM_EOL;
}

static SCM
scm_dealable_set_sensitive (SCM in_state)
{
  struct AisleriotGame *game = app_game;
  gboolean state;

  state = scm_is_true (in_state) ? TRUE : FALSE;
  set_game_dealable (game, state);

  return SCM_EOL;
}

static SCM
scm_get_feature_word (void)
{
  struct AisleriotGame *game = app_game;

  g_return_val_if_fail (game != NULL, SCM_EOL);

  return scm_from_uint (game->features);
}

static SCM
scm_set_feature_word (SCM features)
{
  struct AisleriotGame *game = app_game;

  g_return_val_if_fail (game != NULL, SCM_EOL);

  game->features = scm_to_uint (features);

  return SCM_EOL;
}

static SCM
scm_set_statusbar_message (SCM message)
{
  struct AisleriotGame *game = app_game;
  char *str;

  if (!scm_is_string (message))
    return SCM_EOL;

  scm_dynwind_begin (0);

  str = scm_to_utf8_string (message);
  scm_dynwind_free (str);
  if (!str)
    goto out;

  //g_signal_emit (game, signals[GAME_MESSAGE], 0, str);

out:
  scm_dynwind_end ();

  return SCM_EOL;
}

static SCM
scm_reset_surface (void)
{
  // struct AisleriotGame *game = app_game;
  //
  // clear_slots (game, TRUE);
  // return SCM_EOL;
  g_print ("aisleriot: IDK what to do here");
}

static SCM
scm_set_slot_x_expansion (SCM scm_slot_id,
                          SCM new_exp_val)
{
  // struct AisleriotGame *game = app_game;
  // ArSlot *slot;
  //
  // slot = get_slot (game, scm_to_int (scm_slot_id));
  //
  // /* We should only set the x expansion for right-expanded slots! */
  // g_return_val_if_fail (slot->expanded_right, SCM_EOL);
  // /* Cannot set x and y expansion at the same time */
  // g_return_val_if_fail (!slot->dy_set, SCM_EOL);
  //
  // slot->expansion.dx = scm_to_double (new_exp_val);
  // slot->dx_set = TRUE;
  g_print ("aisleriot: IDK what to do here");

  /* We don't need to emit the slot-changed signal here,
   * since we should be here only during game initialisation,
   * which means that there will be a slot-changed later anyway.
   */
  return SCM_EOL;
}

static SCM
scm_set_slot_y_expansion (SCM scm_slot_id,
                          SCM new_exp_val)
{
  // struct AisleriotGame *game = app_game;
  // ArSlot *slot;
  //
  // slot = get_slot (game, scm_to_int (scm_slot_id));
  //
  // /* We should only set the y expansion for down-expanded slots! */
  // g_return_val_if_fail (slot->expanded_down, SCM_EOL);
  // /* Cannot set x and y expansion at the same time */
  // g_return_val_if_fail (!slot->dx_set, SCM_EOL);
  //
  // slot->expansion.dy = scm_to_double (new_exp_val);
  // slot->dy_set = TRUE;
  g_print ("aisleriot: IDK what to do here");

  /* We don't need to emit the slot-changed signal here,
   * since we should be here only during game initialisation,
   * which means that there will be a slot-changed later anyway.
   */
  return SCM_EOL;
}

static SCM
scm_get_slot (SCM scm_slot_id)
{
  struct AisleriotGame *game = app_game;
  ArSlot *slot;

  slot = get_slot (game, scm_to_int (scm_slot_id));

  if (!slot)
    return SCM_EOL;

  return scm_cons (scm_slot_id,
                   scm_cons (c2scm_deck (slot->cards->data, slot->cards->len),
                             SCM_EOL));
}

static SCM
scm_set_cards (SCM scm_slot_id,
               SCM new_cards)
{
  struct AisleriotGame *game = app_game;
  ArSlot *slot;

  slot = get_slot (game, scm_to_int (scm_slot_id));

  cscmi_slot_set_cards (slot, new_cards);

  return SCM_BOOL_T;
}

static SCM
scm_set_lambda (SCM start_game_lambda,
                SCM pressed_lambda,
                SCM released_lambda,
                SCM clicked_lambda,
                SCM dbl_clicked_lambda,
                SCM game_over_lambda,
                SCM winning_game_lambda,
                SCM hint_lambda,
                SCM rest)
{
  struct AisleriotGame *game = app_game;

  game->lambdas[NEW_GAME_LAMBDA] = start_game_lambda;
  game->lambdas[BUTTON_PRESSED_LAMBDA] = pressed_lambda;
  game->lambdas[BUTTON_RELEASED_LAMBDA] = released_lambda;
  game->lambdas[BUTTON_CLICKED_LAMBDA] = clicked_lambda;
  game->lambdas[BUTTON_DOUBLE_CLICKED_LAMBDA] = dbl_clicked_lambda;
  game->lambdas[GAME_OVER_LAMBDA] = game_over_lambda;
  game->lambdas[WINNING_GAME_LAMBDA] = winning_game_lambda;
  game->lambdas[HINT_LAMBDA] = hint_lambda;

  game->lambdas[GET_OPTIONS_LAMBDA] = SCM_CAR (rest);
  rest = SCM_CDR (rest);

  game->lambdas[APPLY_OPTIONS_LAMBDA] = SCM_CAR (rest);
  rest = SCM_CDR (rest);

  game->lambdas[TIMEOUT_LAMBDA] = SCM_CAR (rest);
  rest = SCM_CDR (rest);

  if (game->features & FEATURE_DROPPABLE) {
    game->lambdas[DROPPABLE_LAMBDA] = SCM_CAR (rest);
    rest = SCM_CDR (rest);
  } else {
    game->lambdas[DROPPABLE_LAMBDA] = SCM_UNDEFINED;
  }

  if (game->features & FEATURE_DEALABLE) {
    game->lambdas[DEALABLE_LAMBDA] = SCM_CAR (rest);
    rest = SCM_CDR (rest);
  } else {
    game->lambdas[DEALABLE_LAMBDA] = SCM_UNDEFINED;
  }

  return SCM_EOL;
}

static SCM
scm_set_lambda_x (SCM symbol,
                  SCM lambda)
{
  struct AisleriotGame *game = app_game;
  const char *lambda_name;
  int i;

  lambda_name = lambda_names;
  for (i = 0; i < N_LAMBDAS; ++i) {
    if (scm_is_true (scm_equal_p (symbol, scm_from_locale_symbol (lambda_name)))) {
      game->lambdas[i] = lambda;
      return SCM_EOL;
    }

    lambda_name += strlen (lambda_name) + 1;
  }

  return scm_throw (scm_from_locale_symbol ("aisleriot-invalid-call"),
                    scm_list_1 (scm_from_utf8_string ("Unknown lambda name in set-lambda!")));
}

static SCM
scm_myrandom (SCM range)
{
  struct AisleriotGame *game = app_game;

  return scm_from_uint32 (g_rand_int_range (game->rand, 0, scm_to_int (range)));
}

static SCM
scm_click_to_move_p (void)
{
  /* This only affects elevator and escalator games. Their code claims
   * that in click-to-move makes no sense to move the cards away, but that's
   * bogus. Just always return FALSE here instead of
   * game->click_to_move ? SCM_BOOL_T : SCM_BOOL_F
   */
  return SCM_BOOL_F;
}

static SCM
scm_update_score (SCM new_score)
{
  struct AisleriotGame *game = app_game;
  char *score;

  score = scm_to_utf8_string (new_score);
  if (g_strcmp0 (score, game->score) != 0) {
    free (game->score);
    game->score = score;

    g_object_notify (G_OBJECT (game), "score");
  } else {
    free (score);
  }

  return new_score;
}

static SCM
scm_set_timeout (SCM new)
{
  struct AisleriotGame *game = app_game;

  g_warning ("(set-timeout) unimplemented\n");

  game->timeout = scm_to_int (new);

  return new;
}

static SCM
scm_get_timeout (void)
{
  struct AisleriotGame *game = app_game;

  g_warning ("(get-timeout) unimplemented\n");

  return scm_from_int (game->timeout);
}

static void
scm_delayed_call_destroy_data (SCM callback)
{
  struct AisleriotGame *game = app_game;

  scm_gc_unprotect_object (callback);

  game->delayed_call_timeout_id = 0;
}

/* @callback is GC protected during this call! */
static gboolean
scm_execute_delayed_function (SCM callback)
{
  struct AisleriotGame *game = app_game;

  /* We set game->delayed_call_timeout_id to 0 _before_ calling |callback|,
   * since it might install a new delayed call.
   */
  game->delayed_call_timeout_id = 0;

  if (!game_scm_call (callback, NULL, 0, NULL))
    return FALSE;

  aisleriot_game_test_end_of_game (game);

  return FALSE;
}

static SCM
scm_delayed_call (SCM callback)
{
  struct AisleriotGame *game = app_game;

  /* We can only have one pending delayed call! */
  if (game->delayed_call_timeout_id != 0) {
    return scm_throw (scm_from_locale_symbol ("aisleriot-invalid-call"),
                      scm_list_1 (scm_from_utf8_string ("Already have a delayed callback pending.")));
  }

  /* We need to protect the callback data from being GC'd until the
   * timeout has run.
   */
  scm_gc_protect_object (callback);

  g_timeout_add_full (G_PRIORITY_LOW,
                      DELAYED_CALLBACK_DELAY,
                      (GSourceFunc) scm_execute_delayed_function,
                      callback,
                      (GDestroyNotify) scm_delayed_call_destroy_data);

  return SCM_BOOL_T;
}

void cscm_init ()
{
  /* Let the scheme side of things know about our C functions. */
  scm_c_define_gsubr ("set-feature-word!", 1, 0, 0, scm_set_feature_word);
  scm_c_define_gsubr ("get-feature-word", 0, 0, 0, scm_get_feature_word);
  scm_c_define_gsubr ("set-statusbar-message-c", 1, 0, 0,
                      scm_set_statusbar_message);
  scm_c_define_gsubr ("reset-surface", 0, 0, 0, scm_reset_surface);
  scm_c_define_gsubr ("add-slot", 1, 0, 0, cscmi_add_slot);
  scm_c_define_gsubr ("get-slot", 1, 0, 0, scm_get_slot);
  scm_c_define_gsubr ("set-cards-c!", 2, 0, 0, scm_set_cards);
  scm_c_define_gsubr ("set-slot-y-expansion!", 2, 0, 0,
                      scm_set_slot_y_expansion);
  scm_c_define_gsubr ("set-slot-x-expansion!", 2, 0, 0,
                      scm_set_slot_x_expansion);
  scm_c_define_gsubr ("set-lambda", 8, 0, 1, scm_set_lambda);
  scm_c_define_gsubr ("set-lambda!", 2, 0, 0, scm_set_lambda_x);
  scm_c_define_gsubr ("aisleriot-random", 1, 0, 0, scm_myrandom);
  scm_c_define_gsubr ("click-to-move?", 0, 0, 0, scm_click_to_move_p);
  scm_c_define_gsubr ("update-score", 1, 0, 0, scm_update_score);
  scm_c_define_gsubr ("get-timeout", 0, 0, 0, scm_get_timeout);
  scm_c_define_gsubr ("set-timeout!", 1, 0, 0, scm_set_timeout);
  scm_c_define_gsubr ("delayed-call", 1, 0, 0, scm_delayed_call);
  scm_c_define_gsubr ("undo-set-sensitive", 1, 0, 0, scm_undo_set_sensitive);
  scm_c_define_gsubr ("redo-set-sensitive", 1, 0, 0, scm_redo_set_sensitive);
  scm_c_define_gsubr ("dealable-set-sensitive", 1, 0, 0, scm_dealable_set_sensitive);

  scm_c_export ("set-feature-word!",
                "get-feature-word",
                "set-statusbar-message-c",
                "reset-surface",
                "add-slot",
                "get-slot",
                "set-cards-c!",
                "set-slot-y-expansion!",
                "set-slot-x-expansion!",
                "set-lambda",
                "set-lambda!",
                "aisleriot-random",
                "click-to-move?",
                "update-score",
                "get-timeout",
                "set-timeout!",
                "delayed-call",
                "undo-set-sensitive",
                "redo-set-sensitive",
                "dealable-set-sensitive",
                NULL);
}

void
scm_start_game (void(* func), const char *filename)
{
    scm_boot_guile (0, NULL, func, NULL);
    scm_primitive_load_path (scm_from_utf8_string (filename));
}