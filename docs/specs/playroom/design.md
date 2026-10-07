# The playroom: a 3D level with physics, shared by every 3D game

Status: **approved**, not built yet.
Date: 2026-10-08
Bevy: 0.19.1, avian3d 0.7.0
Branch: `3d/base`

## Goal

`Screen::Gameplay` is still a coloured background. On `3d/base` it becomes a place: entering
gameplay shows `assets/gameplay/playroom.glb`, a test level for character controllers
(stairs, ramps from 15° to 60°, jump gaps, a maze, a house), lit, seen from a fixed camera,
with colliders ready for whatever walks it.

Only what every 3D game needs belongs here. Who walks the level and how the camera follows
them is each variant's own design (`3d/first-person`, `3d/third-person`, `3d/combined`), so
this one stops at a level that is ready to be walked.

## Decided

| Question | Decision | Why |
| --- | --- | --- |
| Collisions | `avian3d` 0.7, which targets Bevy 0.19 | The playroom exists to test stairs, ramps and walls, which mean nothing without physics. Hand-written raycasts would grow into a worse physics engine |
| The playroom's colliders | `ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)` on the scene root, `RigidBody::Static` | One line covers every mesh in the glb, and it follows the `.blend` when the level changes. A trimesh is exact, and a static body never needs a convex one |
| `Pool_Water` | No collider, via `without_constructor_for_name` | Water you cannot get into is a floor |
| Telling the variants the level is solid | They observe avian's `ColliderConstructorHierarchyReady` on the playroom | A body spawned before the colliders exist falls through the floor on the first fixed step |
| Pausing physics | Nothing to add | avian steps in the fixed schedule, and `Time<Fixed>` follows `Time<Virtual>`, which the pause menu already stops |
| Loading the glb | The loading screen loads it, for both destinations | `docs/flow.md` already says the loading screen "is where the gameplay's own assets will be loaded later". Entering gameplay with the level loaded avoids a blank frame and keeps the bar honest |
| Light | One `DirectionalLight` with shadows and an `AmbientLight`, both values in `Config` | The glb has no lights |
| Camera | A `Camera3d` at a fixed spot looking at the origin | Something has to show the level. A variant replaces it with its own camera |
| Where the code lives | `src/screens/gameplay/playroom.rs`, registered by `gameplay.rs` like `pause.rs` | Split by feature, following the `gameplay.rs` + `gameplay/` pattern |

## Risks to check while building

- **The UI camera over the 3D camera.** The screen camera is a `Camera2d` that clears the
  window and carries the blur (`ui/blur.rs`, a `Core2d` fullscreen pass). The `Camera3d` has to
  draw first (a lower `order`), and the screen camera must not clear over it during gameplay.
  Step 1 checks the playroom shows, and that the pause menu still blurs it.
- **Collider cost.** About 300 meshes become trimeshes. If building them stalls the first
  frame of gameplay, write it down under "What changed".

## Not in this design

A player, a camera that moves, input, and anything the playroom's props suggest (doors,
levers, pickups, physics crates). Those belong to the variants.

## Implementation order

One commit per step. `just check` passes after each one, and `just run` shows it.

1. **Show the playroom.** `GameplayAssets` in `loading.rs`, loaded for both destinations and
   counted by the bar. `playroom.rs` spawns the scene, the light and the fixed `Camera3d`, all
   with `DespawnOnExit(Screen::Gameplay)`, in place of the coloured background. Settle the
   camera order and clearing, and check the pause blur.
   `feat: show the playroom in gameplay`
2. **Give it colliders.** Add `avian3d`, `PhysicsPlugins`, the static trimesh hierarchy without
   `Pool_Water`. In dev builds, `PhysicsDebugPlugin` draws the colliders to check them.
   `build: add avian3d and give the playroom colliders`
3. **Docs.** `docs/flow.md` (Loading, Gameplay), `README.md` (features), `docs/template.md`
   (structure, and the Bevy features a 3D game keeps), this spec's status.
   `docs: document the playroom`

## What changed while building it

Nothing yet.
