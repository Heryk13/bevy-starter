# Split screen: a second player on this PC, on a gamepad

Status: **built** 2026-11-24, tried with two gamepads faked in code and screenshots of the
window: two views, top and bottom. A real gamepad still to try.
Date: 2026-11-24
Bevy: 0.20.0
Roadmap: row 18, part 8 of 8 (`docs/specs/multiplayer-core/design.md` on `main` lists them):
"split screen", its players 2 and up on gamepads (part 7, `docs/specs/gamepads/design.md`). The
CLI is to offer it apart from multiplayer (row 21).

## Goal

A friend on the same sofa picks up a gamepad, presses Select, and plays: the window cuts in two,
the first player on top, the friend below, each walking and looking around on their own.

## Decided

| Question | Decision |
| --- | --- |
| Joining and leaving | Select on a gamepad, while the game runs (`Pause::Off`): one more player for that gamepad, or out again if it already plays. A gamepad unplugged takes its player out. Select is the one button no action takes |
| How many | `MAX_LOCAL_PLAYERS`, two. Three and four were tried first, with the window in a top half and two quarters, or four quarters (`views_of` keeps both layouts); the level went dark in every view, lit by the sky alone, as if the sun's shadows covered it. Two until that is found |
| The first player | Keeps the keyboard, the mouse and every gamepad no one else took (`GuestPad`, on `main`, keeps `Controls` from hearing a guest's) |
| The other player | A `Guest`: the same capsule body (`capsule_body`, shared with `spawn_player`), beside the first player, walking (`walk_velocity`) and looking (`turn_view`, `look_delta_px`, shared with the mouse look) by its gamepad's sticks. First person only |
| Its view | A `PlayroomView` (on `3d/base`, the camera kit without being `PlayroomCamera`): everything that follows or reads "the" camera, the switch, the third person, the editor, the save, still means the first player's. Its order is below the first player's, one each |
| Clearing | The views share one image, and a camera that clears it clears all of it, the views drawn before included: only the view drawn first (the last to join, with the lowest order) clears, the rest draw over. `playroom.rs` had found the same of the UI cameras |
| Not kept | A guest is not saved and not sent to a shared game; it is gone as gameplay ends, and every gamepad goes back to the first player |
| Markers | `split_screen.rs` and every line ending `// split screen`: deleting them leaves a game that builds and passes clippy (tried on a copy) |

## Out of scope

A guest pausing the game (Start on its gamepad is not heard: only the first player's are), a
guest in third person, a guest seen by the others in a shared game, a split screen in 2D.

## What changed while building it

- **The other views stopped clearing** (found at the first screenshot): the first player's view,
  drawn last, cleared the guest's half to grey.
- **Two players, not four** (found at the screenshot of three): see How many.
