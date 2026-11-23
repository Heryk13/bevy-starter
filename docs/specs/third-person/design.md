# Third person: a camera over the shoulder, and F5 between it and first person

Status: **built** 2026-11-23, tried in the game.
Date: 2026-11-23
Bevy: 0.20.0
Roadmap: rows 15 and 17 of `docs/roadmap/roadmap.md`, from their "as agreed" sections; row 16,
the isometric camera, is skipped, and the CLI's part of row 15 (choosing several cameras,
removing the others by their markers, `just check-combos`) moves to row 21 (the user's calls,
2026-11-23).

## Goal

A third-person camera behind and above the player's shoulder, shooter style, as *Helldivers*
aiming: the mouse turns the body as in first person, A and D walk sideways, and the character
always faces where the camera looks. V swaps the shoulder, and the camera pulls in when a wall
comes between it and the player. F5 switches between first and third person at once, with no
effect, as in Minecraft, and the save keeps the camera chosen.

## Decided

| Question | Decision |
| --- | --- |
| How the body turns | With the mouse, as in first person: the same look, yaw on the body and pitch on the camera (the user's call, option A). The two cameras only differ in where the camera sits |
| The look and the cursor | Out of `first_person.rs` into `player/look.rs`, unmarked, since both cameras need them: `look_around`, `hold_cursor`, `is_cursor_locked`, and spawning the one `PlayroomCamera` as the body's child at eye height, tilted to the saved pitch. An exception to "everything about the camera is in `first_person.rs`" (`docs/flow.md`) and to "a piece two features share stays with the first" (`CONTRIBUTING.md`): the first may be deleted by `create-bevy-starter`, and a copy in each camera would drift apart at the first change (the user's call, 2026-11-23) |
| Which camera is on | A marker component on the camera, each defined in its own file: `FirstPersonView`, `ThirdPersonView`. Each camera file puts its marker on the camera as it spawns and places the camera only while its marker is there. With one camera file left, its marker is the only one, so a game with one camera needs no switch |
| First person | `first_person.rs`: the camera at eye height, set when `FirstPersonView` is added |
| Third person | `third_person.rs`: every frame, the camera behind (`THIRD_PERSON_DISTANCE_M`) and above (`THIRD_PERSON_RISE_M`) the eyes, `THIRD_PERSON_SHOULDER_M` to one side, turned with the pitch around the eyes. Only while the camera hangs from the body, so photo mode's and the editor's free camera are left alone |
| The wall | A ray from the eyes to where the camera would be (avian's `SpatialQuery::cast_ray`, the body left out); a hit pulls the camera to just short of it, `THIRD_PERSON_WALL_GAP_M` in front |
| V | `swap_shoulder`, a game action on the Game page, read only in third person: the camera goes to the other shoulder, at once. The shoulder is not saved: a game starts over the right one |
| F5 | `switch_camera`, a game action in release builds too, read in `Pause::Off`: the next camera in `CAMERA_VIEWS`, at once, with no effect (the user's call, Minecraft's switch) |
| The save | `GameData::camera: CameraView`, written as F5 changes it, and put on the camera as it spawns, so Continue resumes with it. A new game starts with the first in `CAMERA_VIEWS`, first person |
| Markers | `// camera: first-person`, `// camera: third-person` and `// camera: switch` on every line outside each file that names it, as `docs/specs/create-bevy-starter/design.md` set them. `camera_switch.rs` names each camera on a marked line, so a camera removed leaves the switch whole |
| Text | Two locale keys, `action-swap-shoulder` and `action-switch-camera`, in English, Brazilian Portuguese and Japanese |

## Out of scope

- The isometric camera (row 16), and the CLI's part (row 21).
- An effect on the switch: it is instant.
- The character's own model: the capsule shows, as it already does.

## Structure

- `player/look.rs`: the camera's spawn, the look, the cursor.
- `player/first_person.rs`: `FirstPersonView`, the camera at the eyes.
- `player/third_person.rs`: `ThirdPersonView` (with its shoulder), the camera behind it, the
  wall, V.
- `player/camera_switch.rs`: `CameraView`, `CAMERA_VIEWS`, F5, the save.

## Tests

- `look.rs`: the pitch and cursor tests, moved as they are.
- `third_person.rs`: the camera sits behind and above the eyes, on the shoulder's side; the
  other shoulder mirrors it; looking down raises the camera; a wall pulls it to short of the
  hit.
- `camera_switch.rs`: F5 steps through the views and wraps; the default is the first.

## Implementation order

1. **The look moves to `look.rs`** (`refactor:`). Nothing changes in the game.
2. **Third person, with V and the wall.** On its own for a commit, both markers on the camera,
   the last to place it wins; step 3 settles which.
3. **F5, and the camera in the save.**
4. **Docs.** `README.md`, `docs/flow.md` (the exception), `docs/template.md`, the roadmap's rows
   15, 16 and 17, this spec's status.

## What changed while building it

- **Steps 2 and 3 are one commit** (decided while building 2): with both views on the camera and
  no switch to keep one, the two would fight over where it sits.
- **The camera's place in third person is set every frame in `Pause::Off` only**, and only while
  the camera hangs from the body: photo mode's free camera moves it on the body, out of the
  running game, and the prop editor lifts it off the body.
- **`CameraView`'s default is written by hand**, `CAMERA_VIEWS[0]`, so leaving a camera out never
  strands a `#[default]` on a line of its own.
- **The markers were tried**: a copy with `third_person.rs`, `camera_switch.rs` and their marked
  lines deleted passes clippy, and so does one with `first_person.rs` and `camera_switch.rs`
  deleted, ahead of row 21's `just check-combos`.
- **README and the roadmap are `main`'s**: their lines for these rows went into `main` and reached
  `3d/player` by the merge down.
