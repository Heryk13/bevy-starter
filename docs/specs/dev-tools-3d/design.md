# Dev tools 3D: the physics debug view, fly, and photo mode's free camera

Status: **built** on 2026-11-19, approved the same day; waiting for the user's run.
Date: 2026-11-19
Bevy: 0.20.0, avian3d 0.8.0
Roadmap: row 5 of `docs/roadmap/roadmap.md`, from its "Dev tools 3D, as agreed" section, and
the 3D free camera of its "Photo mode, as agreed" section.

## Goal

Three tools for building a 3D level. The physics debug view (F6) draws every collider's
outline, the player's capsule included, to check them against the level; today it is always
on in dev builds, and `hide_body_debug_collider` hides the capsule's line from inside it.
Fly (F7) lifts the player off the floor and through the walls, to judge from high and far how
far the map can be seen. And photo mode, built on `main` as a frozen game and a photo of it,
gets its 3D free camera: the cursor stays held, and the view moves away from the player to
frame the photo, coming back when photo mode ends.

## Decided

| Question | Decision |
| --- | --- |
| Branch | `3d/player`, all three: avian, the only physics, lives there and not in `3d/base` (checked: `3d/base`'s `main.rs` and `Cargo.toml` name no avian), and so do the body and the first-person camera. One step goes to `main` first: the hook that lets a branch add its keys to photo mode's bar |
| Files | `src/dev_tools/physics_debug.rs`, `src/dev_tools/fly.rs` and `src/dev_tools/free_camera.rs`, each a dev tool's plugin, added in `dev_tools.rs` on `3d/player`. `Player`, `PlayroomCamera` and the walk actions become `pub(crate)`, so the tools reach them without living inside the player |
| F6, the physics debug view | Shows and hides the outline of every collider: `PhysicsDebugPlugin` moves from `main.rs` into `physics_debug.rs`, and the key flips `enabled` on avian's `PhysicsGizmos` group (`GizmoConfigStore::config_mut::<PhysicsGizmos>()`). Off when the game starts, and not saved: a debug view left on would greet the next launch |
| The capsule | Its outline shows with the rest: `hide_body_debug_collider` is deleted. From first person the line runs down the middle of the screen, which is enough to measure the hitbox (the roadmap's call, 2026-11-07) |
| F6 in photo mode | The outlines are gizmos, not a `DevUi` window, so they showed in photos. Entering photo mode hides them, keeping whether they showed; leaving brings them back, as `fps.rs` does with the FPS |
| F7, fly | Puts a `Flying` marker on the player's body, and with it `GravityScale(0.0)` and avian's `ColliderDisabled`: no gravity for the body alone, and no collisions, so it passes through walls, ceilings and floors. F7 again takes all three off: gravity comes back and the body falls. Turned off inside a wall, avian pushes it out as it can, with no code for it (the roadmap's call) |
| Flying | The walk actions move where the camera looks, up and down included (the camera's `GlobalTransform`, so it works with whatever camera the game keeps); Rise and Sink move straight up and down. `FLY_SPEED_M_PER_S`, 12 m/s, a fixed speed until row 7's multiplier, which multiplies it then. `fly_player` sets the whole velocity after `walk_player`, which set only the part across the ground |
| Where F7 works | Gameplay, while the game runs (`Pause::Off`). The marker is on the body, so leaving gameplay drops it with the body. A game saved while flying is saved in the air, and the loaded game falls: no code for it either |
| Rise and Sink | Two actions on the Dev page, on Space and Left Shift, registered by `fly.rs`, read by fly and by the free camera. They are inner keys (`register_inner_key`): the keys panel leaves them out, the Dev page lists them. Row 10's jump will want Space too; which action gives way is decided there |
| Photo mode's free camera | On entering photo mode, the camera's own `Transform` is kept in a `FreeCameraHome` on it. The mouse turns it, across and up and down, held short of straight up as first person is; the walk actions move it where it looks, Rise and Sink straight up and down, at `FREE_CAMERA_SPEED_M_PER_S`, 3 m/s, fixed: photos are taken close to the player. It counts `Time<Real>`, since the game is frozen. Leaving photo mode puts the `Transform` back, so the view is where it was |
| The camera's space | It moves in its parent's space, the body's, which is frozen: a move is turned into the body's space before it is added. So it needs no detaching and works for a camera with no parent too |
| Through walls | The camera has no collider, so it goes through walls. A photo from inside a wall is the user's to avoid |
| The cursor | Held in photo mode too, as the roadmap asks: `is_cursor_locked` in `first_person.rs` holds it in `Pause::Off` and `Pause::Photo`, and the free camera turns with the mouse there. Under the pause menu and any modal it stays free |
| The hint bar | Photo mode's bar on `main` lists Take Photo and Exit, written in `photo_mode.rs`. `main` gets `register_photo_hint(app, PhotoHint { icon, actions, words })`, as `register_inner_key` is: the bar lists the hints registered, in order, and Exit stays last. Take Photo becomes the first registered. On `3d/player`, `free_camera.rs` adds Move (the four walk actions' keycaps), Up (Rise) and Down (Sink) |

## Out of scope

- **Row 7's speed multiplier**: fly is built at its base speed, which row 7 multiplies.
- **The 2D free camera**: row 6's, which uses the same `register_photo_hint`.
- **Saving F6 or F7**: both start off at every launch.
- **Zoom, roll or a field of view in the free camera**: a camera that moves and turns is what
  the roadmap asks.
- **Getting out of a wall** when fly is turned off inside one: avian's own push.

## Structure

### `main`: the hint bar's hook

`photo_mode.rs` keeps its bar, but builds it from a resource, `PhotoHints`, a list that
`register_photo_hint` fills from each tool's plugin: an `Icon`, the actions whose keycaps it
shows, and its words. Take Photo registers itself from `photo/photo.rs`; Exit is not
registered, the bar always ends with it. On `main` the bar looks as it does today.

### `3d/player`: the three tools

`physics_debug.rs` adds `PhysicsDebugPlugin`, registers F6 on the Dev page, turns the gizmos
off at `Startup`, flips them on F6 (not in photo mode), and on `OnEnter(Pause::Photo)` /
`OnExit` hides and restores them, keeping whether they showed in a resource.

`fly.rs` registers F7, Rise and Sink, toggles `Flying` with its two avian components on F7
(in `Pause::Off`), and `fly_player` sets the body's velocity from the pure `fly_velocity`
(input, the camera's rotation, rise and sink, the speed).

`free_camera.rs` adds its three hints, keeps `FreeCameraHome` on entering photo mode, moves
and turns the camera while in it (`turn_free_camera`, `move_free_camera`, from the pure
`free_camera_step`), and puts it back on leaving.

## Text

English literals, as every dev tool's: "Physics Debug", "Fly", "Rise", "Sink", "Move", "Up",
"Down".

## Tests

- `photo_mode.rs`: the bar lists the hints in the order registered, Exit last.
- `fly.rs`: forward while looking up climbs; Rise with no other key goes straight up; no
  input stands still; the speed is `FLY_SPEED_M_PER_S`.
- `free_camera.rs`: a move forward goes where the camera looks; the move is turned into the
  body's space when the body has turned; the pitch stops short of straight up.
- `first_person.rs`: the cursor is held in photo mode.
- `physics_debug.rs`: leaving photo mode shows the outlines again only if they showed before.

## Implementation order

Each step compiles, passes `just check` and runs, written and tried uncommitted on `3d/player`;
step 1 is committed on `main` and reaches the rest by the merge down.

1. **The hint bar's hook, on `main`.** `PhotoHints` and `register_photo_hint`; the bar built
   from them. *Try:* the bar looks as before, on `main` and on `3d/player`.
2. **F6.** `physics_debug.rs`, `PhysicsDebugPlugin` out of `main.rs`, `hide_body_debug_collider`
   gone. *Try:* F6 shows and hides the outlines, the capsule's line included; a photo taken
   with them on has none.
3. **F7.** `fly.rs`, Rise and Sink. *Try:* fly up past the roof, through a wall, and F7 drops
   the player back down.
4. **The free camera.** `free_camera.rs`, the cursor held in photo mode, the three hints.
   *Try:* in photo mode the view flies around the player, a photo shows it framed, and
   leaving puts the view back where it was.
5. **Docs.** `README.md`, `docs/flow.md`, `docs/template.md`, the roadmap's row 5 and its
   keys, this spec's status.

## What changed while building it

- **All four steps were written at once**, the user's call ("não é muita coisa"), so there is
  no tasks file: the implementation order above was the list.
- **`3d/base` changes too**: `playroom` and `PlayroomCamera` become `pub(crate)` there, since
  `playroom.rs` is `3d/base`'s and a branch only adds to its parent.
- **`walk_input`** is new in `player.rs`: the four walk actions as right and forward, read by
  walking, fly and the free camera alike, so the three never disagree on a key.
- **The free camera moves with `fly_velocity`**, fly's own: the same move where it looks, up and
  down by Rise and Sink, at its own speed. It needs the body's turn only about the vertical,
  which `LockedAxes::ROTATION_LOCKED` guarantees, so no move is turned through the parent.
- **No test for F6's photo restore**: it copies one `bool` back, as `fps.rs` does untested.
