# The playroom: a 3D level every 3D game starts from

Status: **approved**, not built yet.
Date: 2026-10-08
Bevy: 0.19.1
Branch: `3d/base`

## Goal

`Screen::Gameplay` is still a coloured background. On `3d/base` it becomes a place: entering
gameplay shows `assets/gameplay/playroom.glb`, a test level for character controllers
(stairs, ramps from 15° to 60°, jump gaps, a maze, a house), lit, in front of a camera that
stands still.

That is all `3d/base` does. Nothing moves and nothing can be controlled: no player, no input,
no physics. Whoever walks the level, and how, is the business of the branches cut from here
(`3d/first-person`, and the variants cut from it in turn), so they start from a level on
screen without inheriting a body or a camera they would have to take apart.

## Decided

| Question | Decision | Why |
| --- | --- | --- |
| What the base holds | The level, its light and a still camera. No physics, no player, no input | The user's call: the base is the map in front of you and nothing else. Physics arrives with the first branch that has something to move |
| Loading the glb | The loading screen loads it, for both destinations | `docs/flow.md` already says the loading screen "is where the gameplay's own assets will be loaded later". Entering gameplay with the level loaded avoids a blank frame and keeps the bar honest |
| Light | One `DirectionalLight` with shadows and an `AmbientLight`, both values in `Config` | The glb has no lights |
| Camera | A `Camera3d` at a fixed spot, both in `Config`, looking at the origin | Something has to show the level. A branch cut from here replaces it with its own |
| Where the code lives | `src/screens/gameplay/playroom.rs`, registered by `gameplay.rs` like `pause.rs` | Split by feature, following the `gameplay.rs` + `gameplay/` pattern |

## Risks to check while building

- **The UI camera over the 3D camera.** The screen camera is a `Camera2d` that clears the
  window and carries the blur (`ui/blur.rs`, a `Core2d` fullscreen pass). The `Camera3d` has to
  draw first (a lower `order`), and the screen camera must not clear over it during gameplay.
  Step 1 checks the playroom shows, and that the pause menu still blurs it.

## Not in this design

Physics, a player, input of any kind, a camera that moves, and anything the playroom's props
suggest (doors, levers, pickups, physics crates). Those belong to the branches cut from here.

## Implementation order

One commit per step. `just check` passes after each one, and `just run` shows it.

1. **Show the playroom.** `GameplayAssets` in `loading.rs`, loaded for both destinations and
   counted by the bar. `playroom.rs` spawns the scene, the light and the still `Camera3d`, all
   with `DespawnOnExit(Screen::Gameplay)`, in place of the coloured background. Settle the
   camera order and clearing, and check the pause blur.
   `feat: show the playroom in gameplay`
2. **Docs.** `docs/flow.md` (Loading, Gameplay), `README.md` (features), `docs/template.md`
   (structure, and the Bevy features a 3D game keeps), this spec's status.
   `docs: document the playroom`

## What changed while building it

Nothing yet.
