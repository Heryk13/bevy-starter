# Speed: tasks

Roadmap row 7, its base on `main`, then a step on `3d/player` and one on `2d/player`. The
decisions and their reasons live in the spec, `docs/specs/speed/design.md`; this file only
breaks its implementation order into tasks small enough for one sitting each, with the facts
each task needs. The capsule's look is slow motion's (F4), with the `SportShoe` icon.

## How to read this

- **Step** = one commit. Its title is the commit message.
- **Who**: *Claude* writes each step in pieces and the user reads and runs it; *you* is a
  `just run`, which only the user's eyes can do; *both* is the review at the end of a step
  (the checklist in `CLAUDE.md`: every name, the order of sections and systems, `just check`).
- **Facts**: the names that cannot be guessed, checked in the Bevy 0.20.0 source and in this
  repository on 2026-11-19.
- **Where**: step 1 is written and tried uncommitted on `main`, and reaches the players by
  the merge down; steps 2 and 3 are each written on their branch. When to commit is the
  user's call each time.
- **Nothing unused, in either build**, as in every dev tool: what a player reaches for in
  `speed.rs` sits behind `#[cfg(feature = "dev")]`.
- Mark a task done by changing its `[ ]` to `[x]`.

## Departures from the spec, to record in its "What changed while building it"

- **A step is the nearest one, not the next past a tolerance** (decided while building 1.1):
  the three helpers find the place of the step nearest the value (`nearest_step`), then move
  one place out or in, or count places for the bar. That works the same whichever way the
  list runs, down for slow motion and up for speed, and `SPEED_TOLERANCE` went away with it.
- **Each file tests its own list** (decided while building 1.1): `slow_motion.rs` tests the
  helpers over `SLOW_MOTION_STEPS`, and `speed.rs` will test them over `SPEED_STEPS`, where
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

## The actions at a glance

| Action | Default | Label on the Dev page and the panel | Heard | Step |
|--------|---------|-------------------------------------|-------|------|
| `speed` | F8 | Speed | everywhere but photo mode | 1 |
| `slower` (was `slow_motion_slower`) | Numpad − | Slower | while slow motion's or speed's capsule is open | 1 |
| `faster` (was `slow_motion_faster`) | Numpad + | Faster | while slow motion's or speed's capsule is open | 1 |

## Step 1: `feat: show the speed on F8 and multiply it`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 1.1 `slow_motion.rs`: `slower`, `faster` and `lit_segments` become `step_out`, `step_in` and `lit_segments` over a list written from 1× outward, `pub(super)`; their tests | Claude | `f32::total_cmp` orders the distances to each step; `steps.get(..)`, never `[..]` |
| [x] | 1.2 `SLOWER` and `FASTER` become `pub(super)`, named `slower` and `faster` | Claude | `Bindings::load` keeps an unknown name, so a settings file with the old ones starts on the defaults |
| [x] | 1.3 The reset to 1× leaves `toggle_slow_motion` for `reset_slow_motion`, an observer on the capsule's removal | Claude | `On<Remove<SlowMotionCapsule>>`; `app.add_observer(..)` |
| [x] | 1.4 `src/dev_tools/speed.rs`: the `SPEED` action (F8, "Speed"), `SpeedMultiplier`, `SPEED_STEPS`, the capsule with `—`, `toggle_speed`, `reset_speed`; each toggle despawns the other's capsule as it opens its own; added to `dev_tools.rs` after slow motion | Claude | `Icon::SportShoe` (lucide-icons 1.74.0); the bar's and keys' pieces of `slow_motion.rs` (`segment_color`, the `SEGMENT_*` constants, the keycaps' row) are shared too or copied: settled while writing |
| [x] | 1.5 `step_speed`: Slower and Faster step `SpeedMultiplier`, the bar and the `5×` repainted; the helpers' tests over `SPEED_STEPS` | Claude | `step_in` for Slower and `step_out` for Faster, the opposite of slow motion's |
| [x] | 1.6 `Speedometer` with its unit; `measure_speed` adds up the distance and writes the text every `SPEEDOMETER_REFRESH_SECS`; `measured_speed` and `speed_text` and their tests | Claude | `GlobalTransform::translation()`; `Res<Time>`, so a paused game measures nothing; `Option<Single<..>>` for no player |
| [x] | 1.7 `just run`: F8 opens the capsule with `—`, Numpad + lights the bar up to 10×, F4 closes it and opens slow motion, F8 closes slow motion and the game runs at 1× again; neither opens in photo mode. Review | both | |

## Step 2 (on `3d/player`): `feat: show the 3D player's speed on F8`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 2.1 `player.rs`: the body spawns with `Speedometer { unit: "m/s" }`, and `walk_player` multiplies the walk by `SpeedMultiplier`, both behind `cfg(feature = "dev")` | Claude | `walk_player` in `src/screens/gameplay/player.rs` |
| [ ] | 2.2 `fly.rs`: `fly_player` multiplies `FLY_SPEED_M_PER_S` by it, with no `cfg` | Claude | `fly.rs` is a dev tool itself |
| [ ] | 2.3 `just run`: walking at 5× shows about 22 m/s, flying about 60 m/s; closing F8 puts both back. Review | both | |

## Step 3 (on `2d/player`): `feat: show the 2D player's speed on F8`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 3.1 `player.rs`: the square spawns with `Speedometer { unit: "px/s" }`, and `walk_player` multiplies `PLAYER_WALK_SPEED_PX_PER_S` by `SpeedMultiplier`, both behind `cfg(feature = "dev")` | Claude | `walk_player` in `src/screens/gameplay/player.rs` |
| [ ] | 3.2 `just run`: walking at 2× shows about 512 px/s, and the camera keeps up. Review | both | |

## Step 4: `docs: mark the speed spec built`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 4.1 `README.md` (F8 in the dev keys), `docs/flow.md`, `docs/template.md` (how a game's own player connects to it), the roadmap's row 7, the spec's status and "What changed" | Claude | |
| [ ] | 4.2 Sync; `just run` on `main`, `3d/player` and `2d/player` | you | |
