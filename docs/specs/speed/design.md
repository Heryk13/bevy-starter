# Speed: the player's speed on F8, and a multiplier for it

Status: **built** 2026-11-19: the capsule on `main`, the 3D player on `3d/player`, the 2D
player on `2d/player`.
Date: 2026-11-19
Bevy: 0.20.0
Roadmap: row 7 of `docs/roadmap/roadmap.md`, from its "Speed + UI, as agreed" section.

## Goal

A capsule on F8 that shows how fast the player is going (m/s in 3D, px/s in 2D), with a
multiplier under it that Numpad − and + step through 1×, 2×, 5× and 10×. The player's walking
and fly (row 5) go that many times faster. Photo mode's free cameras do not. `main` holds the
capsule and the multiplier, and each player shows its speed in it and reads the multiplier.
The capsule shares its place, its keys and its look with slow motion's (F4). The two are never
open together, and closing either puts its value back at 1×.

## Decided

| Question | Decision |
| --- | --- |
| Branch | The tool goes on `main`, in `src/dev_tools/speed.rs`, a dev tool's plugin added in `dev_tools.rs`. It knows no player: it makes sense in an Empty game, where it shows no speed. `3d/player` and `2d/player` each get a short step that connects their player to it |
| The multiplier | `SpeedMultiplier(f32)`, a `pub(crate)` resource of `speed.rs`, 1.0 by default. It is global, not a component, because it is a setting of the tool and no entity owns it. Fixed steps of `SPEED_STEPS`: 1×, 2×, 5× and 10× (the roadmap's call) |
| The speed shown | `Speedometer`, a `pub(crate)` component a player puts on its body with its unit (`Speedometer::new("m/s")`). The tool measures the body's own `GlobalTransform`: the distance it covered over `SPEEDOMETER_REFRESH_SECS` (0.25 s) of game time, divided by that time. Measuring from the transform works the same for avian's velocity in 3D and for the 2D square, which has no velocity, and needs no per-frame write from the player. In 3D, avian moves the body only on its fixed steps, and reading over a quarter second smooths that out, as the FPS counter does. Falling counts too: it is the speed of the body, not of the walk |
| No player | With no `Speedometer` in the world (main's Empty game, the menu) the capsule shows `—` where the speed would be. The multiplier still steps, for nothing |
| The capsule | Top right, where slow motion's is: the `SportShoe` icon, the speed over the multiplier (`13.2 m/s`, then `5×` smaller and muted), the bar of four segments lit from the left up to the step, and the Numpad − and + keycaps. One decimal for both units: `256.0 px/s` reads well enough, and a decimals field for one unit would be code for a matter of taste |
| Numpad − and + | Slow motion's two actions, shared: one key can only belong to one action (`swap` in `input.rs`), so a second pair on the same keys cannot exist. Their names go from `slow_motion_slower` / `slow_motion_faster` to `slower` / `faster`, since speed reads them too. A settings file holding the old names keeps them as unknown, as `Bindings::load` keeps any unknown name, and the keys start on their default once more. Slower steps toward 1× on the speed capsule and away from it on slow motion's, and Faster does the opposite |
| The step helpers | Slow motion's `slower`, `faster` and `lit_segments` become `step_out`, `step_in` and `lit_segments` over a list of steps written from 1× outward (`[1.0, 0.5, 0.25, 0.1]`, `[1.0, 2.0, 5.0, 10.0]`), `pub(super)` in `slow_motion.rs` and read by `speed.rs`. They stay in the first file that had them, as two users of a piece wait for a third before it moves (CONTRIBUTING) |
| One open at a time | Opening either capsule despawns the other. Each puts its own value back at 1× in an observer on `Remove` of its marker (`On<Remove, SlowMotionCapsule>`, `On<Remove, SpeedCapsule>`), so closing by its key and closing because the other opened go through one place. Slow motion's reset in `toggle_slow_motion` moves into that observer |
| Where F8 works | Everywhere but photo mode, like F4: it neither opens nor steps behind a photo. The capsule is a `DevUi` window, so photo mode hides it with the rest |
| What multiplies | The player's walk and fly. Photo mode's free cameras do not (the roadmap's call). The future run (row 10) will multiply too, since it is walking |
| Release builds | `speed.rs` is a dev tool, so the multiplier and `Speedometer` do not exist without the `dev` feature. A player's two connections to it each carry `#[cfg(feature = "dev")]`: the `Speedometer` on its body, and the `SpeedMultiplier` parameter of its walk with the line that multiplies by it. `first_person.rs` already holds a dev line this way |
| CCD | None: at 10× the capsule may pass through a thin wall between two physics steps. Whether a game needs CCD is its developer's call (the roadmap's call) |

## Out of scope

- **The free cameras**: fixed speed, as photo mode's row decided.
- **Steps between the fixed four**, or a typed value.
- **Saving the multiplier**: closing the capsule puts it back at 1×, and so does every launch.
- **Speeding time up**: that is slow motion's, which only slows down.

## Structure

### `main`

`slow_motion.rs`: `SLOWER` and `FASTER` become `pub(super)` with their new names, the three
step helpers take the list of steps, and the reset to 1× moves into
`reset_slow_motion` (`On<Remove, SlowMotionCapsule>`). `toggle_slow_motion` despawns an open
speed capsule as it opens its own.

`speed.rs`: registers F8, "Speed", on the Dev page. `toggle_speed` opens the capsule, or
closes it, and despawns an open slow motion capsule as it opens. `step_speed` (while the
capsule is open) steps `SpeedMultiplier` by Slower and Faster and repaints the bar and the
`5×`. `measure_speed` (while the capsule is open) adds up each frame's distance on the
`Speedometer` and writes the speed text every `SPEEDOMETER_REFRESH_SECS`. `reset_speed`
(`On<Remove, SpeedCapsule>`) puts the multiplier back at 1×. The pure helpers:
`measured_speed(distance, secs)` and `speed_text(speed, unit)`.

### `3d/player`

`player.rs`: the body spawns with `Speedometer::new("m/s")`, and `walk_player` multiplies the
walk by `SpeedMultiplier`. `fly.rs`: `fly_player` multiplies `FLY_SPEED_M_PER_S` by it, with no
`cfg`, since fly is a dev tool itself.

### `2d/player`

`player.rs`: the square spawns with `Speedometer::new("px/s")`, and `walk_player` multiplies
the walk by `SpeedMultiplier`.

## Text

English literals, as every dev tool's: "Speed", and the units "m/s" and "px/s". "Slower" and
"Faster" keep their labels.

## Tests

- `slow_motion.rs`: the step helpers' tests, rewritten over a list of steps: one step out from
  1× is the second step; out stops at the last; in stops at 1×; a value read back a little
  off still steps from its step; 1× lights one segment and the last step every one. Each runs
  over the speed steps too, where out goes up.
- `speed.rs`: 3 m covered over the refresh time reads `3 / SPEEDOMETER_REFRESH_SECS`; no time
  passed (paused) reads nothing rather than a division by zero; the speed text is one
  decimal and the unit (`13.2 m/s`).

## Implementation order

Each step compiles, passes `just check` and runs. Step 1 is committed on `main` and reaches the
players by the merge down; steps 2 and 3 are each committed on their branch.

1. **The capsule, on `main`.** The shared actions and helpers in `slow_motion.rs`,
   `speed.rs`, both resets moved into the observers. *Try:* F8 opens the capsule with `—`,
   Numpad + lights the bar up to 10×, F4 closes it and opens slow motion, F8 closes slow
   motion and the game runs at 1× again.
2. **The 3D player.** *Try:* walking at 5× shows about 22 m/s, flying about 60 m/s; closing
   F8 puts both back.
3. **The 2D player.** *Try:* walking at 2× shows about 512 px/s, and the camera keeps up.
4. **Docs.** `README.md`, `docs/flow.md`, `docs/template.md` (how a game's own player connects
   to it), the roadmap's row 7, this spec's status.

## What changed while building it

- **A step is the nearest one, not the next past a tolerance** (decided while building 1.1):
  the three helpers find the place of the step nearest the value (`nearest_step`), then move
  one place out or in, or count places for the bar. That works the same whichever way the
  list runs, down for slow motion and up for speed, and `SPEED_TOLERANCE` went away with it.
- **Each file tests its own list** (decided while building 1.1): `slow_motion.rs` tests the
  helpers over `SLOW_MOTION_STEPS`, and `speed.rs` tests them over `SPEED_STEPS`, where
  out goes up. The spec had every test run over both lists from `slow_motion.rs`, which would
  have needed the speed steps spelled a second time before `speed.rs` existed.
- **The observer is `On<Remove<SpeedCapsule>>`** (checked for 1.3): in Bevy 0.20 `Remove` is a
  pattern over the bundle (`bevy_ecs-0.20.0/src/lifecycle.rs`), not a second parameter of
  `On`, as the spec wrote it.
- **The whole capsule is shared, not only the steps** (decided while building 1.4):
  `slow_motion.rs` hands `speed.rs` a `Step Capsule` section, `step_capsule` (the window
  with an icon, a reading of the caller's, the bar and the two keys), `paint_step_bar`,
  `step_label`, `step_out`, `step_in` and one `StepSegment` marker for both bars, which never
  share the screen. Copying the bar's sizes and colors would have been two spellings of one
  look. `lit_segments` stays private, read only through the capsule and the bar.
- **`Speedometer` holds only its unit** (decided while building 1.6), a field a player fills
  in (`Speedometer { unit: "m/s" }`) with no `new`. What is measured lives in `SpeedReading`,
  on the capsule's text, born with the capsule: a measurement never starts from where the
  body was when the capsule last closed, and nothing measures while it is closed. A reading
  that finds another body, as after loading a save, starts over instead of counting the jump.
- **`speed` is a `pub(crate)` module** (found at the start of step 2): `player.rs` lives
  under `screens/`, outside `dev_tools/`, and could not reach a private module, so a commit
  of its own on `main` made it public to the crate before step 2.
- **The `Speedometer` joins the body's bundle, not an `insert`** (decided while building
  2.1): `let body = (...);`, then `#[cfg(feature = "dev")] let body = (body, Speedometer {..});`
  and one `spawn`. A spawn kept in a variable for a `cfg`'d `insert` would leave a release
  build with a variable and a `mut` nobody uses, which `just check` refuses. Both players do
  it the same way.
