# Dev tools: a keys panel, the FPS, slow motion and a profiler

Status: **designed**, not started. To revise before building, since two rows now come first:
row 0 (the Liquid Glass UI) redraws the windows below, written here with the flat `ui.rs` look;
row 1 (key bindings) makes the keys rebindable, so the `*_KEY` constants below become the
default bindings of registered actions, and the keys panel shows the bound key.
Date: 2026-11-07
Bevy: 0.20.0
Roadmap: row 2 of `docs/roadmap/roadmap.md`; its tasks are in
`docs/roadmap/dev-tools-tasks.md`.

## Goal

Dev builds are about to collect many tools on the function keys: the ones in this design,
then photo mode, the file explorer, the 3D ones (physics debug view, fly), speed and screen
recording. In a project that
had them all, nobody remembered which key did what. So the first tool is a list: press F1 and a
small window shows every dev key and what it does. That is the only key anyone has to remember.

Next to it come three tools every game needs while it is being made: the FPS readout that
already exists, slow motion to watch effects and physics frame by frame, and a profiler to
find what costs time.

## Decided

| Question | Decision |
| --- | --- |
| Keys | F1 keys panel, F2 FPS, F4 slow motion. The others belong to later rows (F3 photo mode, F5 camera switch, F6 physics debug view, F7 fly, F8 speed, Numpad 0 file explorer), listed in the roadmap's Keys table |
| Keys panel | Top-left, under the FPS. Lists every dev key except F1 itself. Starts closed |
| FPS | The overlay `main` already has; only its key moves from F3 to F2. Starts on, as today |
| Slow motion | Top-right window showing the speed (`0.25×`). Numpad − and + step through 1×, 0.5×, 0.25×, 0.1× |
| Slow motion's keys | − and + only work while its window is open, so they never steal a key from the game |
| Closing slow motion | Puts the speed back at 1×, so nobody forgets the game slowed down |
| What slows down | Gameplay and physics (`Time<Virtual>`). The UI and the pause menu count `Time<Real>` and do not. Sound does not |
| Faster than 1× | Out. It would only earn its place with rewind, which is out too |
| Layout | Keys panel on the left under the FPS, slow motion on the right: layout "B" of the mockups |
| Profiler | `just profile` with Bevy's `trace_chrome`, not Tracy (below) |
| Where traces go | `profiles/`, ignored by git |
| Text | English in the code, not locale keys, for every dev tool (confirmed by the user; below) |

## Out of scope

- **Photo mode (F3).** Its own spec, roadmap row 3. It hides these windows and the FPS while it
  is on; the marker it needs to find them arrives with it.
- **Clicking the dev windows.** They are read-outs: no buttons, and they never take a click
  from the game underneath (`Pickable::IGNORE`, as Bevy's FPS overlay does).
- **Remembering which windows were open** between runs, and rebinding the keys.
- **Tracy.** Its viewer is a separate download whose version must follow Bevy's, against the
  rule that the repository runs as cloned with nothing installed outside cargo.

## Structure

### Files

| File | Holds |
| --- | --- |
| `src/dev_tools.rs` | the plugin, `DevKeys`, `dev_window()`, `DEV_TOOLS_Z_INDEX` |
| `src/dev_tools/fps.rs` | `FPS_TOGGLE_KEY`, `toggle_fps_overlay`, the `FpsOverlayPlugin` (moved out of `main.rs`) |
| `src/dev_tools/keys_panel.rs` | F1 and the panel |
| `src/dev_tools/slow_motion.rs` | F4, the window, the steps and their pure functions |

`main.rs` keeps `mod dev_tools;` and `app.add_plugins(dev_tools::plugin)`, both behind
`#[cfg(feature = "dev")]`. Disabling `RenderDebugOverlayPlugin` stays in `main.rs`: it is
configuration of `DefaultPlugins`, which `main.rs` builds.

`dev_tools.rs` sits at the top of `src/` because it is not a screen and every screen uses it,
the same reason `ui.rs` and `save.rs` sit there. It is split the `screens.rs` way, `dev_tools.rs`
next to `dev_tools/`, never `mod.rs`.

### The keys list: each tool adds its own line

`DevKeys` is a resource holding the lines the panel shows, each a `KeyCode` and a short
description. Every tool's plugin calls `init_resource::<DevKeys>()` and pushes its own line,
so the plugins can run in any order, and a tool added later, here or in a branch below
(`3d/player`'s physics debug view), never edits a central list. `init_resource` does nothing
when the resource is already there.

The panel shows the lines in the order they were pushed, so `dev_tools.rs` adds its plugins
in key order (FPS, then slow motion). A key's name comes from its `Debug` form
(`format!("{key:?}")` writes `F2`).

### The windows

`dev_window()` in `dev_tools.rs` is the one look both windows share: an absolute `Node` with
padding, `ui.rs`'s `SURFACE` fill and `BORDER`, a rounded corner, `Pickable::IGNORE` and
`GlobalZIndex(DEV_TOOLS_Z_INDEX)`. Text uses `ui.rs`'s `SMALL_FONT_SIZE_PX`, `TEXT`, and
`ACCENT` for the title.

They are not built from `floating_root()` and `card()`, though the Bevy patterns say floating
UI is: those are centered panels with a blur behind and a backdrop that blocks the game, and
these are corner read-outs that must leave the game playable. The code carries that reason
in a comment.

They carry no `UiTargetCamera`, so they render on the screen camera (`IsDefaultUiCamera`), like
Bevy's FPS overlay. When the pause menu opens, the blur covers them along with the FPS: they
stay readable enough, and keeping them sharp would mean a third camera. Roadmap row 8 (screen
recording) adds that camera later, so its videos leave the dev UI out; until then they stay
here.

They belong to no state: a key spawns them on any screen, and the same key despawns them. So
they carry no `DespawnOnExit`.

### Z order

`DEV_TOOLS_Z_INDEX` is 300, above the dialog (200), so no screen root on the same camera covers
a dev window. It gets its row in `docs/flow.md`'s table. Bevy's FPS overlay uses its own
`FPS_OVERLAY_ZINDEX` (`i32::MAX - 32`, `bevy_dev_tools-0.20.0/src/fps_overlay.rs:37`).

## FPS

`toggle_fps_overlay` stays as it is, `FPS_TOGGLE_KEY` becomes `KeyCode::F2`, and `fps.rs`
pushes the F2 line. `FpsOverlayConfig::enabled` starts `true`, so it is on at start, as today.

The keys panel sits at a fixed distance from the top, under the FPS, and does not move up when
F2 hides it (Claude's call): following the FPS would mean a system reading `FpsOverlayConfig`
for a gap that only shows while both are in use.

## Slow motion

`SLOW_MOTION_STEPS` lists the speeds from fastest to slowest: `[1.0, 0.5, 0.25, 0.1]`. Two pure
functions take the current speed and return the next one: one step slower, stopping at the
last step, and one step faster, stopping at 1×. They find the current speed among the steps
with a small tolerance, since an `f32` read back may not equal the constant bit for bit.

- F4 spawns the window, or despawns it and calls `Time<Virtual>::set_relative_speed(1.0)`.
- One system reads `KeyCode::NumpadSubtract` and `KeyCode::NumpadAdd`, only while the window
  exists (`run_if(any_with_component::<...>)`), sets the relative speed, and rewrites the
  window's text.

Facts it rests on:

- `Time<Virtual>::set_relative_speed(f32)` and `relative_speed()`
  (`bevy_time-0.20.0/src/virt.rs:148`, `:192`).
- `FixedUpdate` advances by virtual time, so physics added by a branch below (`avian3d` in
  `3d/player`) slows down with no code of its own.
- The pause menu calls `Time<Virtual>::pause()` and `unpause()`, which leave the relative
  speed alone: a slowed game paused and resumed is still slowed.
- Bevy's audio plays in real time, so sound keeps its pitch and speed with nothing to do.

Known edges, left alone (Claude's call): anything that counts `Res<Time>` slows down too while
the window is open. Outside gameplay, that is the splash screen if its timers do. In gameplay,
it is `count_play_time` (`src/screens/gameplay.rs`): a slowed session saves less play time
than the clock on the wall, the same way the pause menu stops it. Play time is game time, and
slow motion is a dev tool used on purpose.

## `just profile`

Bevy's `trace_chrome` feature records how long every system takes, each frame, into a JSON
file written when the game closes. It is opened by dragging it into `ui.perfetto.dev` in any
browser.

- The recipe creates `profiles/` and runs the game with `--features bevy/trace_chrome`.
- The env var `TRACE_CHROME` sets the file's path (`bevy_log-0.20.0/src/lib.rs:335`); without
  it the file lands in the folder the game ran from, the repository's root. The recipe points it
  at a file inside `profiles/` named by date and time, so runs never overwrite each other.
- `/profiles` goes in `.gitignore`, next to `/target` and `/dist`. The `.gitignore` reaches
  games made with `create-bevy-starter`, so they ignore it too.
- `trace_chrome` also turns on Bevy's `trace` and `debug` features. Traced builds are slower,
  so its numbers compare systems with each other, not with a release build.
- Trace files grow fast, around hundreds of MB a minute: short runs.

## Text

Every word the player reads is a locale key, but no player reads these: release builds are
made with `--no-default-features`, which leaves the whole `dev_tools` module out. So the
descriptions and titles are English literals, and the code says why where they live. Putting
them in every `main.ftl` would add keys the game never ships.

## Tests

In `slow_motion.rs`, under `// ==== Tests ====`, comparing floats with a tolerance:

- one step slower from 1× is 0.5×
- slower stops at the slowest step
- faster stops at 1×
- one step faster from the slowest step is the one before it

The keys panel and the FPS have no logic of their own to test: they read the ECS and spawn
nodes.

## Implementation order

Every step compiles, passes `just check` and runs. One commit per step.

1. **Move.** The FPS code out of `main.rs` into `dev_tools.rs` and `dev_tools/fps.rs`. Only a
   move. *Try:* F3 still hides and shows the FPS.
2. **FPS on F2.** *Try:* F2 hides and shows it, F3 does nothing.
3. **Keys panel.** `DevKeys`, `dev_window()`, `DEV_TOOLS_Z_INDEX`, the panel on F1, the F2 line.
   *Try:* F1 opens the panel under the FPS, listing F2; F1 again closes it; clicks go through
   it to the game.
4. **Slow motion.** The steps, the two pure functions and their tests, the window on F4, the
   Numpad keys, the F4 line. *Try:* in gameplay, + and − step the speed and the window
   follows; the pause menu's blur still ramps at normal speed; closing the window brings 1×
   back; with the window closed, + and − do nothing.
5. **Profiler.** `/profiles` in `.gitignore`, the `profile` recipe. *Try:* `just profile`, play
   a little, close: a dated file is in `profiles/` and opens in `ui.perfetto.dev`.
6. **Docs.** `README.md` (the dev keys line that names F3 today, `just profile` in the commands
   table, the `dev` feature row), `docs/flow.md` (the z-index row), `docs/template.md` (the new
   files), and this spec's status. Then merge down: `main` into `3d/base` into `3d/player`,
   and `main` into `2d/base`.

Merging into `3d/player` conflicts in `main.rs`: that branch adds `PhysicsDebugPlugin` to the
same dev tools block step 1 moves out. The merge keeps it in `main.rs`, as it is (Claude's
call); moving it into `dev_tools` and giving it a key is roadmap row 5, and a merge commit
that quietly does a row's work hides it from that row's review.

## What changed while building it

Nothing yet.
