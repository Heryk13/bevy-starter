# Key bindings: tasks

Roadmap row 1, built on `main`. The decisions and their reasons live in the spec,
`docs/specs/key-bindings/design.md`; this file only breaks its implementation order into
tasks small enough for one sitting each, with the facts each task needs. The page as
approved is in `docs/roadmap/ui/controls-screens.html`.

## How to read this

- **Step** = one commit. Its title is the commit message.
- **Who**: *Claude* writes each step in pieces and the user reads and runs it, the way the
  Look was built; *you* is a `just run`, which only the user's eyes can do; *both* is the
  review at the end of a step (the checklist in `CLAUDE.md`: every name, the order of
  sections and systems, `just check`). A step may go back to the user writing the Rust: that
  is settled in one line at its start.
- **Facts**: the names that cannot be guessed (types, methods, modules), checked in the Bevy
  0.20.0 source and in this repository on 2026-11-18.
- **Where**: every step is written and tried uncommitted on `3d/player`, then
  `just tools/sync main "<message>"` commits it on `main` and merges it down the tree.
  Step 6 alone is `3d/player`'s: its sync is `just tools/sync 3d/player "<message>"`.
- **Nothing unused on `main`.** `just check` refuses code nobody calls, a field nobody reads
  and a variant nobody builds, in the dev build and in the release one, and `#[allow]` with
  them. So each piece lands in the task that first uses it, and a task says so when it adds
  a parameter to something an earlier task wrote.
- A task is done when `just check` passes, except where it says the next task makes it pass.
- Mark a task done by changing its `[ ]` to `[x]`.

## Departures from the spec, to record in its "What changed while building it"

- **A slot paints itself, at once** (decided while building 4.1). Task 4.1 gave a slot the
  white fill of a `Choice` button, which `paint_buttons` eases; but a slot has three faces
  (at rest, under the pointer, listening) and shows words in place of its keycap, so
  `paint_slots` in `controls.rs` paints it, as `paint_menu_words` paints the menu, and with
  no ease.
- **What listens is a field of `Bindings`, not a resource of its own** (decided while
  building 4.2). `Controls` already asks `Bindings` what an action is bound to, so asking it
  whether one is being chosen needs no second resource in every reader.
- **The press that starts the listening is never taken as the answer** (found while building
  4.4). `take_pressed_binding` waits a frame after a slot starts to listen: a tap
  that presses and lets go within one frame would otherwise bind Mouse 1 on the spot.
- **Reset to Defaults is put in the card's footer by its page** (decided while building 5.1).
  The footer belongs to the panel and holds Close; a page of controls inserts its button
  before Close, with a margin that takes the room between the two, and it goes when the page
  does. `SettingsFooter` marks the row.
- **The language's control and the row of pages share `segment` and `set_segment`** (decided
  while building 3.1), where the language had `language_segment` and its own painting.
- **Two tests open Settings in a real app** (added in 3.1). A page is spawned by another
  state's `OnEnter` than the panel's and asks for the panel's body, an order no pure function
  can hold: `settings_opens_on_the_general_page_inside_the_panel` does.
- **"Press a key..." is translated on the Dev page too** (decided while building 4.1). The
  slot is one widget for both pages, and its words are the game's: only a dev tool's own
  label is English.
- **Right-hand modifier keys say so** (decided while building 3.5): `R Shift`, `R Ctrl`,
  `R Alt`. A keyboard prints the same word on both, but they are two keys to bind.
- **No slot keeps listening after the panel closes** (added in 4.4). Nothing can close the
  panel while a slot listens, since Esc is silenced and the shield takes every click; an
  `OnExit` system stops the listening all the same, so no action could ever stay deaf.
- **A key the OS cannot name is not heard** (found in the review after step 7). A slot keeps
  listening through a `KeyCode::Unidentified`, which has nothing to print on a keycap. The
  same review added a test that opens the Controls page in a real app, and one that no
  action is heard while a slot listens.
- **Two commits, not seven** (decided at the sync). The seven steps were written in one
  sitting and run in the game together, so they landed as one commit on `main`, steps 1 to 5
  and 7, and one on `3d/player`, step 6.

## The actions at a glance

| Action | Name in `settings.ron` | Registered by | Key | Page | Label |
|--------|------------------------|---------------|-----|------|-------|
| Pause | `pause` | `overlays.rs`, `main` | Esc | game | none: it is not listed and never changes |
| FPS Overlay | `fps_overlay` | `main.rs`, `main`, dev builds | F2 | dev | `FPS Overlay`, English |
| Move Forward | `move_forward` | `player.rs`, `3d/player` | W | game | `action-move-forward` |
| Move Back | `move_back` | `player.rs`, `3d/player` | S | game | `action-move-back` |
| Move Left | `move_left` | `player.rs`, `3d/player` | A | game | `action-move-left` |
| Move Right | `move_right` | `player.rs`, `3d/player` | D | game | `action-move-right` |

## Step 1: `feat: read pause and the fps overlay as actions`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 1.1 `src/input.rs`, a new shared file: a binding (a key or a mouse button), an action (its name and its default binding), and the resource that lists them with the binding each has now | Claude | `KeyCode` and `MouseButton` are in `bevy::prelude`; the name is a `&'static str` behind a small type, so a feature keeps it as a constant. Only what step 1 reads: no page, no label yet |
| [x] | 1.2 Registering an action from a plugin | Claude | the shape `GlassLayers` has: `app.init_resource::<..>()`, then `app.world_mut().resource_mut::<..>()` pushes. Two actions with one default key are logged with `error!` at `Startup` |
| [x] | 1.3 Reading: a system parameter that says whether an action was just pressed, and the run condition built on it | Claude | `#[derive(SystemParam)]` (`bevy::ecs::system::SystemParam`) over the resource, `Res<ButtonInput<KeyCode>>` and `Res<ButtonInput<MouseButton>>`. "Is it held" is not written here: nothing reads it on `main` until 3.6 |
| [x] | 1.4 `overlays.rs` registers Pause and `apply_esc_action` runs on its press; `main.rs` adds the input plugin, and in dev builds registers the FPS overlay, on F2, and `toggle_fps_overlay` runs on its press | Claude | today: `input_just_pressed(KeyCode::Escape)` and `input_just_pressed(FPS_TOGGLE_KEY)`; `FPS_TOGGLE_KEY` goes. The plugin is added in `main.rs`'s game block, where the others are |
| [x] | 1.5 `README.md` line 28 (F3 → F2), `docs/flow.md` (a row for `input.rs` in the shared files, Esc and the FPS key read as actions), `docs/template.md` (the source tree), the roadmap's Keys table if it still says F3 anywhere | Claude | |
| [x] | 1.6 `just run`: Esc pauses and closes panels as before; F2 shows and hides the FPS overlay, F3 does nothing; review; sync | you, then both | |

## Step 2: `feat: remember changed bindings in the settings file`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 2.1 `Cargo.toml`: a line of its own for `bevy_input` with its `serialize` feature, and the comment that it is not used directly | Claude | as `icu_provider` is written there; at Bevy's version, `default-features = false`; under it `KeyCode` and `MouseButton` derive `Serialize` and `Deserialize`. The line is new, so it is the same on every branch and merges down clean, where the `bevy = ..` line differs on each |
| [x] | 2.2 The swap, a pure function: the bindings, an action and a new binding in, the bindings out. Whoever held it takes what this action gave up | Claude | no label exists yet, so "an action that is not listed is refused" arrives in 3.2 with the label |
| [x] | 2.3 Its tests | Claude | `binding_a_free_key_changes_one_action`, `binding_a_taken_key_swaps_the_two_actions`, `binding_an_actions_own_key_changes_nothing` |
| [x] | 2.4 The bindings that differ from the defaults, a pure function, and its tests | Claude | `default_bindings_have_nothing_to_save`, `only_changed_bindings_are_saved` |
| [x] | 2.5 `SettingsFile` gains the changed bindings by action name; `load_settings` applies each saved one through the swap, in the order the actions were registered; `save_settings` writes what differs, and writes back untouched the names this build does not have | Claude | `#[serde(default)]` on the field, as `language` has; a `BTreeMap<String, ..>` keeps the file's order stable. The names this build does not know are kept in the bindings resource between the load and the save. `load_settings` runs at `Startup`, after every plugin has registered |
| [x] | 2.6 Tests of the file and of loading | Claude | `settings_without_bindings_keep_the_defaults`, `a_saved_binding_reads_back`, `a_binding_for_an_unknown_action_is_kept`, `two_saved_bindings_on_one_key_load_as_one_key_each` |
| [x] | 2.7 `docs/flow.md` (what `settings.ron` holds), `docs/template.md` (`Cargo.toml`'s new line) | Claude | |
| [x] | 2.8 `just run`: with the game closed, edit `settings.ron` so `fps_overlay` is another key; that key now toggles the overlay and F2 does not; open and close Settings, and the file still holds it; add a made-up action name by hand, open and close Settings, and it is still there; review; sync | you, then both | the file is in `%APPDATA%\bevy-starter\` |

## Step 3: `feat: list the actions on the settings pages`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 3.1 The pages: a sub-state of `Overlay::Settings` with General, Controls and, in dev builds, Dev; the card keeps its title, the row of pages and Close; General's four rows move into a page of their own | Claude | `#[derive(SubStates)]` with `#[source(Overlay = Overlay::Settings)]` and `app.add_sub_state::<..>()`, as `Pause` in `pause.rs`; each page spawns on `OnEnter` with `DespawnOnExit`, as a child of the card. The row of pages is the language's control again: a `RadioGroup` of `RadioButton`s, its `ValueChange<Entity>` setting the next page |
| [x] | 3.2 An action gains its page (game or dev) and its label, which is optional: no label, not listed, never changed. The swap refuses a key held by an action with no label, and loading ignores a saved binding for one | Claude | the two registrations of step 1 gain the two values: Pause is a game action with no label, the FPS overlay a dev action labelled `FPS Overlay`. The dev page's variant sits behind `#[cfg(feature = "dev")]`. Tests: `a_key_held_by_an_unlisted_action_is_refused`, `a_saved_binding_for_an_unlisted_action_is_ignored` |
| [x] | 3.3 A page is offered only when it has an action to list, and the row of pages only when there is more than General | Claude | on `main`: release shows Settings as today, dev shows General and Dev |
| [x] | 3.4 `src/overlays/settings/controls.rs`, a new part of `settings.rs`: the list, a scrolling column of one `setting_row` per listed action of the page | Claude | `settings.rs` next to `settings/`, never `mod.rs`; `setting_row` becomes `pub(super)`; the scrolling column is `Overflow::scroll_y()` with `ScrollArea`, as `save_list()` in `saves.rs` |
| [x] | 3.5 A key's legend, a pure function, and its tests | Claude | `a_letter_key_reads_as_its_letter`, `a_digit_key_reads_as_its_digit`, `a_function_key_reads_as_its_name`, `a_named_key_reads_as_printed`, `a_mouse_button_reads_as_mouse_and_its_number`. Legends are not translated. The `ponytail:` comment on legends being a US keyboard's goes here |
| [x] | 3.6 The keycap: a grey skirt holding a white face holding the legend in `INK`; and "is it held", read by a system that sets each keycap's face down on its skirt while its action is held | Claude | the roadmap's measures ("The look"): face `#FFFFFF`, skirt about 5 px of `#C3C3CB`, a 6 px corner, a soft shadow under the key. "Is it held" is added to the system parameter here, its first reader |
| [x] | 3.7 The page's words in the three `main.ftl` | Claude | `settings-page-general`, `settings-page-controls`: General / Controls; Geral / Controles; 一般 / 操作. The Dev page's name and its labels are English in the code |
| [x] | 3.8 `docs/flow.md` (the Settings section, the shared files), `docs/template.md` (the source tree) | Claude | |
| [x] | 3.9 `just run`: General as before; the Dev page lists FPS Overlay beside an F2 keycap; hold F2 and the keycap goes down; the keycap reads over the sky and under Reduce Transparency; in the three languages; review; sync | you, then both | on `3d/player` too there is no Controls page yet: its actions come in step 6 |

## Step 4: `feat: rebind an action from its page`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 4.1 A slot is a button around its keycap; activating it makes it the one listening, which turns it white with "Press a key…" in place of the keycap, and shows the cancel hint under the list | Claude | which slot listens is a resource; `Activate` (`bevy::ui_widgets`) read off `activate.entity`, as every button in the project. The slot takes the white fill and the dark ink of a `Choice` button under the pointer |
| [x] | 4.2 While a slot listens every action reads as not pressed | Claude | the system parameter of 1.3 asks the listening resource first |
| [x] | 4.3 What a pressed input does to a listening slot, a pure function, and its tests | Claude | `esc_cancels_the_listening`, `any_other_key_becomes_the_binding`, `a_mouse_button_becomes_the_binding` |
| [x] | 4.4 The page takes the key in `PostUpdate`: it binds through the swap, repaints the keycaps of the two actions, and stops listening | Claude | `ButtonInput::get_just_pressed()` for both devices. `PostUpdate`, since every reader of an action runs in `Update` or before: the Esc that cancels must not close the panel, nor a key just bound to a tool run it |
| [x] | 4.5 A click that binds reaches nothing under it: while a slot listens, a node over the whole window takes the press | Claude | a child of the overlay's root spawned after the card, `PositionType::Absolute` at 100% by 100%; a UI node blocks what is under it by default (`bevy::picking::Pickable`). Gone when listening ends |
| [x] | 4.6 The words: `controls-listening` and `controls-cancel-hint` in the three `main.ftl` | Claude | Press a key… / Esc cancels; Pressione uma tecla… / Esc cancela; キーを押してください… / Escでキャンセル |
| [x] | 4.7 `docs/flow.md` | Claude | |
| [x] | 4.8 `just run`, on the Dev page: click the F2 keycap, press F4: it says F4 and F4 toggles the overlay; click again and press Esc: it stays F4 and the panel stays open; click again and click Close: it says Mouse 1 and the panel stays open; restart: the binding is still there; review; sync | you, then both | the swap between two actions is seen in the game in step 6: `main` has one key that can change |

## Step 5: `feat: reset the controls to their defaults`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 5.1 Reset to Defaults at the left of each page's footer, Close at its right; it asks, and Yes puts every listed action back, those of the other page included | Claude | `dialog(question, ButtonKind::Danger, on_yes)` from `ui.rs`, with `DespawnOnExit(Overlay::Settings)`; the footer is `close_footer()` in `overlays.rs` today, with Close alone at the right |
| [x] | 5.2 The words: `controls-reset` and `dialog-reset-controls` in the three `main.ftl` | Claude | Reset to Defaults / Reset all controls to their defaults?; Restaurar Padrões / Restaurar todos os controles para o padrão?; デフォルトに戻す / すべての操作をデフォルトに戻しますか？ |
| [x] | 5.3 `docs/flow.md` | Claude | |
| [x] | 5.4 `just run`: rebind the FPS overlay, Reset, No: nothing changes; Reset, Yes: F2 again, on the page and in the game; restart: still F2, and `settings.ron` holds no binding for it; review; sync | you, then both | |

## Step 6 (on `3d/player`): `feat: walk through actions`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 6.1 `player.rs` registers the four actions, in the order the page lists them, and `walk_player` asks whether each is held | Claude | `walk_player` reads `Res<ButtonInput<KeyCode>>` today, with `KeyCode::KeyW`, `KeyS`, `KeyA`, `KeyD`; `walk_velocity` and its tests do not change |
| [x] | 6.2 Their labels in the three `main.ftl` | Claude | `action-move-forward`, `-back`, `-left`, `-right`: Move Forward, Move Back, Move Left, Move Right; Mover para Frente, Mover para Trás, Mover para a Esquerda, Mover para a Direita; 前進, 後退, 左へ移動, 右へ移動 |
| [x] | 6.3 `docs/flow.md`'s gameplay section, on `3d/player` | Claude | |
| [x] | 6.4 `just run`: Settings now has Controls, with four rows; bind Move Forward to the up arrow and it walks, W does not; bind Move Forward to S and Move Back takes the key it gave up; bind Move Left to F2 and the FPS overlay, on the Dev page, takes A; hold a key and its keycap goes down; Reset puts all of it back; no label wraps in the three languages; review; sync | you, then both | `just tools/sync 3d/player "feat: walk through actions"`: this step is not `main`'s |

## Step 7: `docs: mark the key bindings spec built`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 7.1 The spec's status line and its "What changed while building it", from the departures above; the roadmap's row 1; `README.md`'s features | Claude | |
| [ ] | 7.2 Sync; `just run` on `main` in a dev build (General and Dev) and on `2d/base` | you | a release build of `main` is not run by `just run`: `just check` builds it, and it shows Settings as today |
