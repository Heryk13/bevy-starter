# Dev tools base: tasks

Roadmap row 2, built on `main`. The decisions and their reasons live in the spec,
`docs/specs/dev-tools/design.md`; this file only breaks its implementation order into tasks
small enough for one sitting each, with the facts each task needs.

**To redo before starting.** These tasks were written before rows 0 and 1 joined the roadmap.
Once those are built, the spec is revised and these tasks with it: step 3's `dev_window()`
becomes row 0's glass pieces (no `SURFACE` fill), and the `*_KEY` constants become default
bindings of actions registered with row 1. Steps 1, 2 and 5 (the move, F2, `just profile`)
stand as they are.

## How to read this

- **Step** = one commit. Its title is the commit message.
- **Who**: *you* writes the Rust; *Claude* does pure moves, docs, doc comments and separators;
  *both* is a review together (the checklist in `CLAUDE.md`: every name, the order of sections
  and systems, `just check`).
- **Facts**: the names you cannot guess (types, methods, modules), checked in the Bevy 0.20
  source. Assembling them is the task.
- A task is done when `just check` passes, except where it says the next task makes it pass.
- Mark a task done by changing its `[ ]` to `[x]`.

## The keys at a glance

| Key | Tool | Step |
|-----|------|------|
| F1  | keys panel | 3 |
| F2  | FPS (moved from F3) | 2 |
| F4  | slow motion; Numpad − / + while its window is open | 4 |
| —   | `just profile` | 5 |

The other dev keys belong to later rows: the roadmap's Keys table lists them.

## Step 1: `refactor: move the dev tools into their own file`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 1.1 Move the FPS constant, system and plugin lines out of `main.rs` into `src/dev_tools.rs` + `src/dev_tools/fps.rs` | Claude | a pure move; `main.rs` keeps `#[cfg(feature = "dev")] mod dev_tools;` and the `add_plugins` line |
| [ ] | 1.2 `just run`: F3 still hides and shows the FPS; review; commit | both | |

## Step 2: `feat: show the FPS on F2`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 2.1 Move the FPS to F2 | you | `KeyCode::F2` |
| [ ] | 2.2 `just run`: F2 toggles it, F3 does nothing; review; commit | both | |

## Step 3: `feat: list the dev keys on F1`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 3.1 `DevKeys` resource in `dev_tools.rs`: a list of (key, description) | you | `#[derive(Resource, Default)]`; the description is `&'static str` |
| [ ] | 3.2 `fps.rs` registers its F2 line from its plugin | you | `app.init_resource::<DevKeys>()` (does nothing if it already exists), then `app.world_mut().resource_mut::<DevKeys>()` |
| [ ] | 3.3 `DEV_TOOLS_Z_INDEX` (300) and `dev_window()` in `dev_tools.rs` | you | `Node { position_type: PositionType::Absolute, .. }`, `BackgroundColor(SURFACE)`, `BorderColor::all(BORDER)`, `Pickable::IGNORE`, `GlobalZIndex`; colors and sizes from `ui.rs` |
| [ ] | 3.4 Config in `keys_panel.rs`: the F1 key and the panel's distance from the top (under the FPS) | you | `KeyCode::F1`; the distance is tuned by eye in 3.7 |
| [ ] | 3.5 System on F1: spawn the panel with one row per line, or despawn it if it is open | you | `input_just_pressed(KEY)` in `bevy::input::common_conditions`; `Option<Single<Entity, With<KeysPanel>>>` tells open from closed; a key's name is `format!("{key:?}")` |
| [ ] | 3.6 `DEV_TOOLS_Z_INDEX` row in `docs/flow.md`'s z-index table | Claude | |
| [ ] | 3.7 `just run`: F1 opens it under the FPS, listing F2; clicks go through it; review; commit | both | |

## Step 4: `feat: slow the game down on F4`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 4.1 Config in `slow_motion.rs`: the F4 key, `SLOW_MOTION_STEPS = [1.0, 0.5, 0.25, 0.1]`, the two Numpad keys | you | `KeyCode::NumpadSubtract`, `KeyCode::NumpadAdd` |
| [ ] | 4.2 Two pure functions: one step slower, one step faster, both stopping at the ends | you | find the current speed in the steps with a tolerance (`(a - b).abs() < EPSILON`), not `==` |
| [ ] | 4.3 Four tests (listed in the spec's Tests) | you | floats compared with a tolerance |
| [ ] | 4.4 F4 opens the window, or closes it and sets the speed back to 1× | you | `ResMut<Time<Virtual>>`, `.set_relative_speed(1.0)` |
| [ ] | 4.5 − / + system, only while the window is open: new speed, then the window's text | you | `run_if(any_with_component::<SlowMotionWindow>)`; `.relative_speed()` reads the current one; `Text` holds the string |
| [ ] | 4.6 `slow_motion.rs` registers its F4 line | you | same as 3.2 |
| [ ] | 4.7 `just run`: physics slows, the pause blur does not; closing brings 1× back; review; commit | both | |

## Step 5: `build: profile with just profile`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 5.1 `/profiles` in `.gitignore`, next to `/target` and `/dist` | you | |
| [ ] | 5.2 `profile` recipe in the `justfile`: create `profiles/`, run the game with Chrome tracing into a dated file there | you | `--features bevy/trace_chrome`; the env var `TRACE_CHROME` takes the file's path; the recipes run in `sh`, so `$(date +...)` works, as `release` already uses `rm` and `cp` |
| [ ] | 5.3 `just profile`, play a little, close, drag the file into `ui.perfetto.dev`; commit | both | the file is only written when the game closes |

## Step 6: `docs: document the dev tools`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 6.1 `README.md`: the line naming F3 for the FPS, `just profile` in the commands table, the `dev` feature row | Claude | |
| [ ] | 6.2 `docs/template.md`: the new files; spec status to built | Claude | |
| [ ] | 6.3 Commit; merge `main` → `3d/base` → `3d/player` and `main` → `2d/base`; push | both | |
