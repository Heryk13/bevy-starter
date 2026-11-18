# Dev tools: a keys panel, the FPS, slow motion and a profiler

Status: **built** on `main` on 2026-11-18, merged down to every branch. First written before rows
0 and 1 of the roadmap existed; the revision redraws the windows as glass (row 0) and reads
every key as a registered action (row 1), which already moved the FPS to F2.
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
| Keys | F1 keys panel, F2 FPS, F4 slow motion, as default bindings of actions on Settings' Dev page, where each can be changed. The others belong to later rows, listed in the roadmap's Keys table |
| Keys panel | Top-left, under the FPS. Lists every dev action except its own, each beside the key it is bound to now. Starts closed |
| The list | The actions themselves: `Bindings::listed(Page::Dev)`. No list of its own (below) |
| FPS | Bevy's overlay, as `main` has it, already on F2 as an action since row 1. Only moves to its own file. Starts on |
| Slow motion | Top-right capsule: an hourglass, the speed (`0.25×`), a bar of steps, and the two keys that step it. Numpad − and + step through 1×, 0.5×, 0.25×, 0.1× |
| Slow motion's keys | Two actions of their own, Slower and Faster, read only while the capsule is open, so they never steal a key from the game |
| Closing slow motion | Puts the speed back at 1×, so nobody forgets the game slowed down |
| What slows down | Gameplay and physics (`Time<Virtual>`). The UI and the pause menu count `Time<Real>` and do not. Sound does not |
| Faster than 1× | Out. It would only earn its place with rewind, which is out too |
| Layout | Keys panel on the left under the FPS, slow motion on the right: layout "B" of the mockups |
| Look | Glass, as every card: the windows float on a layer of their own, with no backdrop behind them (below). `docs/roadmap/ui/mockup.html`, blocks "2" and "2 + 7" |
| Profiler | `just profile` with Bevy's `trace_chrome`, not Tracy (below) |
| Where traces go | `profiles/`, ignored by git |
| Text | English in the code, not locale keys, for every dev tool (confirmed by the user; below) |

## Out of scope

- **Photo mode (F3).** Its own spec, roadmap row 3. It hides these windows and the FPS while it
  is on; the marker it needs to find them arrives with it.
- **Clicking the dev windows.** They are read-outs: no buttons, and they never take a click
  from the game underneath (`Pickable::IGNORE`, as Bevy's FPS overlay does).
- **Remembering which windows were open** between runs.
- **An icon beside each line of the keys panel**, which the mockup shows. An action carries no
  icon, and one more field on every action, read only by this panel, would be dead code in a
  release build. The panel's title keeps its icon.
- **The regions** (blur only where glass is), which the roadmap had put off to the first glass
  smaller than the screen over a running game. That is these windows, but they exist only in
  dev builds, and the whole chain measured about 0.03 ms a frame on the full screen
  (`docs/specs/glass/design.md`). They wait for a glass a released game shows while playing.
- **A glass FPS.** Bevy's overlay stays as it is, text and graph, as in the mockup.
- **Tracy.** Its viewer is a separate download whose version must follow Bevy's, against the
  rule that the repository runs as cloned with nothing installed outside cargo.

## Structure

### Files

| File | Holds |
| --- | --- |
| `src/dev_tools.rs` | the plugin, `DEV_TOOLS_Z_INDEX` and its glass layer, `dev_root()`, `dev_window()` |
| `src/dev_tools/fps.rs` | the `FPS_OVERLAY` action and its default key, `toggle_fps_overlay`, the `FpsOverlayPlugin` (moved out of `main.rs`) |
| `src/dev_tools/keys_panel.rs` | the `KEYS_PANEL` action and the panel |
| `src/dev_tools/slow_motion.rs` | the three actions, the capsule, the steps and their pure functions |

`main.rs` keeps `mod dev_tools;` and `app.add_plugins(dev_tools::plugin)`, both behind
`#[cfg(feature = "dev")]`. Disabling `RenderDebugOverlayPlugin` stays in `main.rs`: it is
configuration of `DefaultPlugins`, which `main.rs` builds.

`dev_tools.rs` sits at the top of `src/` because it is not a screen and every screen uses it,
the same reason `ui.rs` and `save.rs` sit there. It is split the `screens.rs` way, `dev_tools.rs`
next to `dev_tools/`, never `mod.rs`.

Three files outside it change, each as little as the windows need:

| File | Change |
| --- | --- |
| `src/ui/blur.rs` | a floating root that lets the pointer through gets no backdrop, and its layer is never frozen |
| `src/ui/panel.rs` | the card's rim, shadow and glass marker become a bundle of their own, which `card()` is built from and `dev_window()` reuses |
| `src/overlays/settings/controls.rs` | `keycap()` and `binding_legend()` reach the keys panel; two legends added (`Num −`, `Num +`) |

### The keys list: the actions are the list

The first version of this design had a `DevKeys` resource each tool pushed a line into. Row 1
built that list already: every tool registers an `Action` with `page: Page::Dev` and a
`label`, and `Bindings::listed(Page::Dev)` returns them in the order they were registered,
each with the binding it has now. The keys panel reads that and skips its own action. So a
tool added later, here or in a branch below (`3d/player`'s physics debug view), appears on
the panel and on Settings' Dev page by registering once, and no second list can disagree
with the first.

`dev_tools.rs` adds its plugins in key order (FPS, then slow motion), which is the order the
panel shows.

### The keycap

Each line of the panel shows its key as the keycap of Settings' Controls page: `keycap(action,
binding_legend(binding))` from `src/overlays/settings/controls.rs`. The panel is its second
user, and with two the code stays with the first (`CONTRIBUTING.md`, Splitting files): the two
functions become `pub(crate)` and are re-exported through `settings.rs` and `overlays.rs`
behind `#[cfg(feature = "dev")]`, since nothing in a release build reaches for them from
outside. The third user (photo mode's hint, row 3) moves the keycap to `src/ui/`.

A keycap's legend follows a binding changed while the panel is open with no code here:
`paint_slots` rewrites every `KeycapLegend` in the world when `Bindings` changes, and it runs
while Settings is open, the only place a binding changes. The code says so where it relies
on it.

`key_legend` spells `NumpadSubtract` and `NumpadAdd` as Bevy names them (`Num Subtract`), so
they get named legends: `Num −` and `Num +`.

### The windows: glass with no backdrop

Everything that floats today is a modal: `floating_root()` covers the window, takes every
click, and `blur.rs` draws the blurred, dimmed screen on it. A dev window is the first thing
that floats over a game that keeps running. Its root is the same `floating_root()`, on a
layer of its own, plus `Pickable::IGNORE`, and `blur.rs` learns one rule from that:

**A floating root that lets the pointer through has no backdrop.** What is under it stays
sharp, live and clickable. So:

- `target_layer_camera` sends such a root to its layer's camera and gives it no glass of its
  own; its windows still get theirs, from the layer's card copy.
- `freeze_settled_layers` never freezes a layer with such a root open: nothing promises that
  what shows under it is still (Settings open under the keys panel, say). A layer never
  frozen never counts as covering the screens, so `ScreensCovered` stays right as it is.
- `retarget_open_glass` reaches the windows of a root that has no glass itself, so they
  follow a resize.

The rule reads `Pickable` instead of a marker of its own on purpose: a marker only the dev
tools insert would be dead code in a release build, and the two always go together anyway,
since a backdrop that dims the game while clicks pass through it would be a bug.

`dev_root()` in `dev_tools.rs` is that root: `floating_root(DEV_TOOLS_Z_INDEX)` and
`Pickable::IGNORE`. `dev_window()` is the one look both windows share: an absolute `Node`
with its own padding and corner, the glass bundle of `panel.rs` (rim, shadow, the `Card`
marker that brings the glass), and `Pickable::IGNORE`. Each tool spawns a root with its
window straight under it, where `give_card_glass` looks for the layer. They come in with the
entrance every floating root has.

A window starts with the card's glass. Whether it needs a clearer tint of its own over a
running game is seen in the game, in the step that first shows one, and if so it is a named
constant beside `DEV_TOOLS_Z_INDEX`.

While a dev window is open its layer is blurred every frame, the pause menu included: about
0.03 ms on the full screen, in dev builds only. Under Reduce Transparency the windows are the
solid surface, as every card.

They belong to no state: a key spawns them on any screen, and the same key despawns them. So
they carry no `DespawnOnExit`.

### Z order

`DEV_TOOLS_Z_INDEX` is 300, above the dialog (200), and is pushed onto `GlassLayers` from the
plugin, so the windows are drawn by a camera of their own over every other layer: sharp over
the pause menu, and with whatever is open under them blurred in their glass. It gets its row
in `docs/flow.md`'s table. Roadmap row 8 (screen recording) wanted a camera that leaves the
dev UI out of its videos: this is that camera.

Bevy's FPS overlay is not ours to move: it stays on the screen camera with its own
`FPS_OVERLAY_ZINDEX` (`i32::MAX - 32`, `bevy_dev_tools-0.20.0/src/fps_overlay.rs:37`), and the
pause menu blurs it as today.

## FPS

Nothing changes but where it lives: the `FPS_OVERLAY` action, `FPS_OVERLAY_KEY`,
`toggle_fps_overlay` and the plugin line move from `main.rs` to `dev_tools/fps.rs`.
`FpsOverlayConfig::enabled` starts `true`, so it is on at start, as today.

The keys panel sits at a fixed distance from the top, under the FPS, and does not move up when
F2 hides it (Claude's call): following the FPS would mean a system reading `FpsOverlayConfig`
for a gap that only shows while both are in use.

## Slow motion

`SLOW_MOTION_STEPS` lists the speeds from fastest to slowest: `[1.0, 0.5, 0.25, 0.1]`. Two pure
functions take the current speed and return the next one: one step slower, stopping at the
last step, and one step faster, stopping at 1×. They find the current speed among the steps
with a small tolerance, since an `f32` read back may not equal the constant bit for bit. A
third tells how many segments of the bar are lit: one at 1×, one more per step slower.

Three actions on the Dev page: Slow Motion (F4), Slower (Numpad −), Faster (Numpad +).

- Slow Motion spawns the capsule, or despawns it and calls
  `Time<Virtual>::set_relative_speed(1.0)`.
- One system reads Slower and Faster, only while the capsule exists
  (`run_if(any_with_component::<...>)`), sets the relative speed, and rewrites the capsule's
  speed and bar.

The capsule holds the hourglass (`Icon::Hourglass`, at the HUD's 32 px), the speed, the bar,
and the two keys as keycaps, which follow their bindings like the panel's.

Roadmap row 7 (speed) steps with the same two keys, "whichever window is open". Two actions
cannot share a default key, so that row moves Slower and Faster up to `dev_tools.rs` and
reads them too. Until then they are slow motion's.

Facts it rests on:

- `Time<Virtual>::set_relative_speed(f32)` and `relative_speed()`
  (`bevy_time-0.20.0/src/virt.rs:148`, `:192`).
- `FixedUpdate` advances by virtual time, so physics added by a branch below (`avian3d` in
  `3d/player`) slows down with no code of its own.
- The pause menu calls `Time<Virtual>::pause()` and `unpause()`, which leave the relative
  speed alone: a slowed game paused and resumed is still slowed.
- Bevy's audio plays in real time, so sound keeps its pitch and speed with nothing to do.
- `Controls` hears nothing while Settings waits for a key, so the key pressed to bind an
  action never also opens a window.

Known edges, left alone (Claude's call): anything that counts `Res<Time>` slows down too while
the capsule is open. Outside gameplay, that is the splash screen if its timers do. In gameplay,
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
labels and titles are English literals, and the code says why where they live. Putting
them in every `main.ftl` would add keys the game never ships. An action's label is the same
words on the keys panel and on Settings' Dev page.

## Tests

In `slow_motion.rs`, under `// ==== Tests ====`, comparing floats with a tolerance:

- one step slower from 1× is 0.5×
- slower stops at the slowest step
- faster stops at 1×
- one step faster from the slowest step is the one before it
- the bar lights one segment at 1× and every segment at the slowest step

In `blur.rs`: a layer with a root that lets the pointer through open is not frozen over a
still background. In `controls.rs`: the two Numpad legends.

The keys panel and the FPS have no logic of their own to test: they read the ECS and spawn
nodes.

## Implementation order

Every step compiles, passes `just check` and runs. One commit per step.

1. **Move.** The FPS action, key, system and plugin line out of `main.rs` into `dev_tools.rs`
   and `dev_tools/fps.rs`. Only a move. *Try:* F2 still hides and shows the FPS, and Settings'
   Dev page still lists it.
2. **The card's glass on its own.** `panel.rs` hands out the rim, the shadow and the `Card`
   marker as one bundle, and `card()` is built from it. *Try:* the pause menu, an overlay and
   a dialog look as before.
3. **Keys panel.** The no-backdrop rule in `blur.rs` and its test, `DEV_TOOLS_Z_INDEX` and its
   layer, `dev_root()`, `dev_window()`, the keycap's re-exports, the `KEYS_PANEL` action and
   the panel, the z-index row in `docs/flow.md`. *Try:* F1 opens the panel under the FPS,
   listing the FPS on F2; F1 again closes it; the game under it is sharp and takes clicks
   through it; it stays sharp over the pause menu; a key changed on the Dev page changes on
   the open panel; Reduce Transparency and a resize with it open.
4. **Slow motion.** The steps, the pure functions and their tests, the three actions, the
   capsule, the two Numpad legends. *Try:* in gameplay, + and − step the speed and the
   capsule follows; the pause menu still comes in at normal speed; closing the capsule brings
   1× back; with it closed, + and − do nothing; the panel lists the three.
5. **Profiler.** `/profiles` in `.gitignore`, the `profile` recipe. *Try:* `just profile`, play
   a little, close: a dated file is in `profiles/` and opens in `ui.perfetto.dev`.
6. **Docs.** `README.md` (the dev keys, `just profile` in the commands table, the `dev` feature
   row), `docs/flow.md` (the no-backdrop rule beside the layers), `docs/template.md` (the new
   files), the roadmap's row and its table of pieces, and this spec's status.

Step 1 is written on `main` itself, not tried on `3d/player` first as the glass was: it edits
`main.rs`, which differs between the two, and a pure move has nothing to look at behind a
panel. Steps 2 to 4 are tried on `3d/player`, where the level moves under the windows.

Merging step 1 into `3d/player` conflicts in `main.rs`: that branch adds `PhysicsDebugPlugin`
to the same dev tools block step 1 moves out. The merge keeps it in `main.rs`, as it is
(Claude's call); moving it into `dev_tools` and giving it a key is roadmap row 5, and a merge
commit that quietly does a row's work hides it from that row's review.

## What changed while building it

- **The card's glass bundle is `card_glass()`, private until step 3.** Step 2 had it
  `pub(super)`; with `card()` its only caller, private said so, and step 3 opened it along
  with its re-export.
- **`dev_root()` became `spawn_dev_window(commands, root_marker, window)`.** Bevy's picking
  stops at any node with no `Pickable`, and a keycap is built with children of its own, which
  the caller cannot reach. So the function spawns the root with the window under it and then
  gives every node `Pickable::IGNORE` with `insert_recursive::<Children>`; the root still
  carries it as it spawns, where `blur.rs` reads it. `dev_window()` no longer brings its own.
- **F1 waits in the panel's place while it is closed**, asked by the user once the panel ran:
  the panel's keycap beside "Dev Tools", so not even F1 has to be remembered. It is a plain
  root on the screen camera, as Bevy's FPS overlay is, not a dev window: an open layer blurs
  every frame, and the hint has no glass to blur for, so the pause menu blurs it like the FPS.
  It takes no click, and follows a rebinding through `paint_slots` like every keycap.
- **The panel sits 116 px from the top**: Bevy's FPS overlay is its 32 px text over a graph
  twice that tall, about 104 px, whose sizes it keeps private.
- **A HUD's icon comes from a new `sized_icon(icon, size_px)` in `ui.rs`**, which `icon()` is
  now built on. The plan had the capsule set its own `TextFont`, but a bundle cannot hold the
  same component twice, so the size has to come in where the icon is made.
- **The capsule is a row inside the window**: `dev_window()` lays its children in a column,
  and the fields it sets win over the caller's `place`, so the hourglass, the speed, the bar
  and the keys sit in one row node under it.
- **Slower and faster need no index**: each takes the first step past the speed in its
  direction, or stays where it is, so a speed between two steps moves to the next step on its
  side. A sixth test covers a speed read back a little off its step.
- **`just profile` says where to read its file**, asked by the user: the recipe prints the
  file's path and https://ui.perfetto.dev when the game closes, and the README explains it
  beside the commands.
- **Steps 5 and 6 landed as one commit**, the user's call, to sync less.
