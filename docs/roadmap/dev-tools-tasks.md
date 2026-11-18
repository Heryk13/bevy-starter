# Dev tools base: tasks

Roadmap row 2, built on `main`. The decisions and their reasons live in the spec,
`docs/specs/dev-tools/design.md`; this file only breaks its implementation order into tasks
small enough for one sitting each, with the facts each task needs. The windows as agreed are
in `docs/roadmap/ui/mockup.html`, blocks "2" and "2 + 7".

Rewritten on 2026-11-18, when the spec was revised for rows 0 and 1: the first version had a
flat `dev_window()`, a `DevKeys` resource and `*_KEY` constants read straight off the keyboard.

## How to read this

- **Step** = one commit. Its title is the commit message.
- **Who**: *Claude* writes each step in pieces and the user reads and runs it, the way the
  Look and the key bindings were built; *you* is a `just run`, which only the user's eyes can
  do; *both* is the review at the end of a step (the checklist in `CLAUDE.md`: every name, the
  order of sections and systems, `just check`). A step may go back to the user writing the
  Rust: that is settled in one line at its start.
- **Facts**: the names that cannot be guessed (types, methods, modules), checked in the Bevy
  0.20.0 source and in this repository on 2026-11-18.
- **Where**: step 1 is written on `main` itself, since it edits `main.rs`, which differs on
  `3d/player`. Steps 2 to 5 are written and tried uncommitted on `3d/player`, where the level
  moves under the windows, then `just tools/sync main "<message>"` commits each on `main` and
  merges it down the tree.
- **Nothing unused, in either build.** `just check` refuses code nobody calls, in the dev
  build and in the release one, which leaves the whole `dev_tools` module out. So whatever
  only the dev tools reach for outside their module is re-exported behind
  `#[cfg(feature = "dev")]`, and each piece lands in the task that first uses it.
- A task is done when `just check` passes, except where it says the next task makes it pass.
- Mark a task done by changing its `[ ]` to `[x]`.

## Departures from the spec, to record in its "What changed while building it"

- **The card's glass bundle is `card_glass()`, private until step 3** (decided while building
  2.1). Task 2.1 had it `pub(super)`; with `card()` its only caller, private says so, and step
  3 opens it along with its re-export.
- **`dev_root()` became `spawn_dev_window(commands, root_marker, window)`** (found while
  building 3.4). Bevy's picking stops at any node with no `Pickable`, and a keycap is built
  with children of its own, which the caller cannot reach. So the function spawns the root
  with the window under it and then gives every node `Pickable::IGNORE` with
  `insert_recursive::<Children>`; the root still carries it as it spawns, where `blur.rs`
  reads it. `dev_window()` no longer brings its own.
- **F1 waits in the panel's place while it is closed** (asked by the user while building 3.4):
  the panel's keycap beside "Dev Tools", so not even F1 has to be remembered. It is a plain
  root on the screen camera, as Bevy's FPS overlay is, not a dev window: an open layer blurs
  every frame, and the hint has no glass to blur for, so the pause menu blurs it like the FPS.
  It takes no click, as the spec has every dev window, and follows a rebinding through
  `paint_slots` like every keycap.
- **The panel sits 116 px from the top**, not just under a guessed FPS line: Bevy's overlay
  is its 32 px text over a graph twice that tall, about 104 px.

## The actions at a glance

| Action | Default | Label on the Dev page and the panel | Step |
|--------|---------|-------------------------------------|------|
| `fps_overlay` | F2 | FPS Overlay (exists since row 1) | 1 moves it |
| `keys_panel` | F1 | Keys Panel (not listed on the panel itself) | 3 |
| `slow_motion` | F4 | Slow Motion | 4 |
| `slow_motion_slower` | Numpad − | Slower | 4 |
| `slow_motion_faster` | Numpad + | Faster | 4 |
| —   | — | `just profile` | 5 |

The other dev keys belong to later rows: the roadmap's Keys table lists them.

## Step 1: `refactor: move the dev tools into their own file`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 1.1 On `main`: move `FPS_OVERLAY`, `FPS_OVERLAY_KEY`, the `register_action` call, `toggle_fps_overlay` and the `FpsOverlayPlugin` line out of `main.rs` into `src/dev_tools.rs` + `src/dev_tools/fps.rs` | Claude | a pure move; `main.rs` keeps `#[cfg(feature = "dev")] mod dev_tools;` and the `add_plugins` line; disabling `RenderDebugOverlayPlugin` stays, it configures `DefaultPlugins` |
| [x] | 1.2 `docs/flow.md` and `docs/template.md`: wherever they say the FPS overlay lives in `main.rs` | Claude | grep `toggle_fps_overlay`, `FPS_OVERLAY` |
| [x] | 1.3 `just run` on `main`: F2 still hides and shows the FPS, Settings' Dev page still lists it; review; sync | both | the merge into `3d/player` conflicts in `main.rs` on the dev tools block: keep `PhysicsDebugPlugin` there, as the spec says |

## Step 2: `refactor: hand out the card's glass on its own`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 2.1 `src/ui/panel.rs`: the rim (`BorderGradient`), the shadow, `entrance_transform(0.0)` and `Card` become one bundle, and `card()` is its `Node` plus that bundle | Claude | the bundle stays `pub(super)` and is not re-exported yet: step 3 is its first user outside, and an export nobody uses fails `just check`; the rim only shows on a `Node` with a border, which stays the caller's |
| [x] | 2.2 `just run`: the pause menu, an overlay and a dialog look as before; review; sync | both | |

## Step 3: `feat: list the dev keys on F1`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 3.1 `src/ui/blur.rs`: a floating root that lets the pointer through gets no backdrop (`target_layer_camera`), keeps its windows on a resize (`retarget_open_glass`) and never lets its layer freeze (`freeze_settled_layers`, through a pure function with its test) | Claude | `Pickable { should_block_lower, is_hoverable }` in `bevy::picking`, `Pickable::IGNORE` is both `false`; an `Add` observer sees every component of the bundle that brought `Floating`; none of it is dev-only, so nothing here is behind `cfg` |
| [x] | 3.2 `src/dev_tools.rs`: `DEV_TOOLS_Z_INDEX` (300) pushed onto `GlassLayers`, `dev_root()`, `dev_window()`; the card's glass bundle re-exported through `src/ui.rs` behind `#[cfg(feature = "dev")]` | Claude | `floating_root(z)` and `GlassLayers` are already `pub(crate)`; a window goes straight under its root, where `give_card_glass` reads the layer; nobody calls these until 3.4, which makes `just check` pass |
| [x] | 3.3 `keycap()` and `binding_legend()` in `src/overlays/settings/controls.rs` become `pub(crate)`, re-exported through `settings.rs` and `overlays.rs` behind `#[cfg(feature = "dev")]` | Claude | `paint_slots` and `press_held_keycaps` already reach every `KeycapLegend` and `KeycapFace` in the world while Settings is open: say so in a comment on each |
| [x] | 3.4 `src/dev_tools/keys_panel.rs`: the `KEYS_PANEL` action (F1, "Keys Panel"), a system that spawns the panel or despawns it, a title with `Icon::Keyboard`, one line per `Bindings::listed(Page::Dev)` except its own | Claude | `action_just_pressed(KEYS_PANEL)`; `Option<Single<Entity, With<KeysPanel>>>` tells open from closed; a node with no `Pickable` blocks the pointer (`bevy_ui-0.20.*/src/picking_backend.rs`), so every node of a window needs `Pickable::IGNORE`, the keycaps' and the texts' included |
| [x] | 3.5 `docs/flow.md`: the `DEV_TOOLS_Z_INDEX` row in the z-index table, the no-backdrop rule beside the layers; `docs/template.md`: the new files | Claude | |
| [x] | 3.6 `just run`: F1 waits under the FPS beside "Dev Tools"; it opens the panel in its place, listing the FPS on F2, and closes it; the game under it is sharp and takes clicks through it; it stays sharp over the pause menu; a key changed on the Dev page changes on the open panel; Reduce Transparency; a resize with it open. Tune its place and its tint; review; sync | both | whether a window wants a clearer glass than a card's is decided here, as the spec leaves it |

## Step 4: `feat: slow the game down on F4`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 4.1 `src/dev_tools/slow_motion.rs`: `SLOW_MOTION_STEPS = [1.0, 0.5, 0.25, 0.1]`, the pure functions (one step slower, one step faster, how many segments are lit) and the five tests of the spec | Claude | find the current speed among the steps with a tolerance, never `==`; nobody calls them until 4.2 |
| [ ] | 4.2 The three actions; Slow Motion opens the capsule, or closes it and sets the speed back to 1×; Slower and Faster step it only while the capsule is open | Claude | `ResMut<Time<Virtual>>`, `.set_relative_speed(f32)`, `.relative_speed()` (`bevy_time-0.20.0/src/virt.rs`); `run_if(any_with_component::<...>)`; `KeyCode::NumpadSubtract`, `KeyCode::NumpadAdd` |
| [ ] | 4.3 The capsule: the hourglass, the speed, the bar of steps, the two keycaps; one system rewrites the speed and the bar when the speed changes | Claude | `icon(Icon::Hourglass)` from `src/ui.rs` is sized by `ICON_SIZE_PX` (22): a HUD's icon is 32 px, so its `TextFont` is set here |
| [ ] | 4.4 `key_legend` in `controls.rs` spells the two Numpad keys `Num −` and `Num +`, with their test | Claude | today they read `Num Subtract` and `Num Add`, Bevy's names less the prefix |
| [ ] | 4.5 `docs/template.md`: the file; `README.md` waits for step 6 | Claude | |
| [ ] | 4.6 `just run`: in gameplay + and − step the speed and the capsule follows; the pause menu still comes in at normal speed; closing brings 1× back; closed, + and − do nothing; the keys panel lists the three; review; sync | both | |

## Step 5: `build: profile with just profile`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 5.1 `/profiles` in `.gitignore`, next to `/target` and `/dist` | Claude | |
| [ ] | 5.2 `profile` recipe in the `justfile`: create `profiles/`, run the game with Chrome tracing into a dated file there | Claude | `--features bevy/trace_chrome`; the env var `TRACE_CHROME` takes the file's path (`bevy_log-0.20.0/src/lib.rs`); the recipes run in `sh`, so `$(date +...)` works, as `release` already uses `rm` and `cp` |
| [ ] | 5.3 `just profile`, play a little, close, drag the file into `ui.perfetto.dev`; sync | you | the file is only written when the game closes |

## Step 6: `docs: mark the dev tools spec built`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 6.1 `README.md`: the dev keys (F1, F4 beside F2), `just profile` in the commands table, the `dev` feature row | Claude | |
| [ ] | 6.2 The spec's status and its "What changed while building it" from the Departures above; the roadmap's row 2 and its table of pieces (popover and HUD shown, the regions still waiting) | Claude | |
| [ ] | 6.3 Sync; `just run` on `main` and on `2d/base` | you | every step before was seen on `3d/player` only, step 1 aside |
