# Walking the playroom in first person

Status: **approved**, not built yet.
Date: 2026-10-08
Bevy: 0.19.1, avian3d 0.7.0
Branch: `3d/first-person`, cut from `3d/base`

## Goal

`3d/base` gives gameplay a lit level with colliders, seen from a fixed camera (see
`docs/specs/playroom/design.md`). This design puts a player in it: a plain capsule seen in
first person. WASD walks, the mouse looks around, walls stop the player and gravity keeps
them on the floor.

Saving and loading already carry `GameData` the whole way (see `docs/flow.md`, "Where the
game's own state goes"). This design gives `GameData` its first field: where the player
stands and where they look. Save, quit, load, and the player is back on the same spot,
facing the same way.

## Decided

| Question | Decision | Why |
| --- | --- | --- |
| The player | A dynamic `RigidBody` with a capsule `Collider` and `LockedAxes::ROTATION_LOCKED`, its look a `Capsule3d` mesh with a plain `StandardMaterial` | The smallest body that walks: gravity and walls come from avian, and locked rotation keeps it from tipping over |
| When the player spawns | On the playroom's `ColliderConstructorHierarchyReady` | Before the colliders exist, a dynamic body falls through the floor on the first fixed step |
| Where a new game starts | The origin, as a constant in `player.rs`'s `Config` | The centre of the map, among the ramps, jumps and stairs. A constant does not break when a node is renamed in Blender |
| Walking | WASD sets `LinearVelocity` on the ground plane, relative to where the player faces, and leaves its `y` to gravity | Setting the velocity, not adding force, stops the player as soon as the key is released |
| Camera | The playroom's fixed `Camera3d` is replaced by one that is a child of the player at eye height. Yaw turns the body, pitch turns only the camera | Walking follows yaw alone, so looking down never slows the player |
| Mouse | `AccumulatedMouseMotion`, cursor locked and hidden through the window's `CursorOptions` on `OnEnter(Pause::Off)`, freed on `OnExit(Pause::Off)` | `Pause::Off` exists only while playing unpaused, so the pause menu, its overlays and leaving gameplay all free the cursor with no extra code |
| What a save keeps | `GameData::player: PlayerState { position: [f32; 3], yaw: f32, pitch: f32 }`, with a `Default` at the new game's spawn | Plain floats keep the save format independent of Bevy's: `Vec3` only serializes with Bevy's `serialize` feature, which is off. Velocity is not kept, so a loaded game starts standing still |
| Keeping it current | A system copies the player's `Transform` into `Session::game` every frame | `write_save` already writes `Session::game`, so the pause menu's Save and the autosave need no change |
| A save from before this design | Loads at the spawn | `GameData` is `#[serde(default)]`, so a missing `player` is `PlayerState::default()` with no `SAVE_VERSION` bump |
| Where the code lives | `src/screens/gameplay/player.rs`, registered by `gameplay.rs` | Split by feature, next to `playroom.rs` |

## Risks to check while building

- **Stairs.** A plain dynamic capsule climbs ramps up to the friction's angle, but it may stop
  at the 0.2 to 0.3 m steps of `Stair_*`. If it does, write it down under "What changed": a
  real character controller (stepping, slopes, jumping) is its own design.
- **Falling out of the map.** Nothing catches a player who falls off the edge. That is left for
  later and written down here so it is not forgotten.

## Not in this design

Jumping, running, crouching, a character controller crate (`bevy-tnua`), a mouse sensitivity
setting, gamepad look, and anything the playroom's props suggest (doors, levers, pickups,
physics crates).

## Implementation order

One commit per step, after `3d/base` has built its playroom design and been merged in.
`just check` passes after each one, and `just run` shows it.

1. **Spawn the player.** `player.rs`: the capsule body, its mesh, spawned at the origin on
   `ColliderConstructorHierarchyReady`. The playroom's fixed camera gives way to one that is the
   player's child at eye height. The capsule should drop onto the floor and stay there.
   `feat: drop a capsule player into the playroom`
2. **Look around.** Lock and free the cursor with `Pause::Off`, turn yaw and pitch from mouse
   motion. Pitch clamping is a pure function with tests.
   `feat: look around with the mouse`
3. **Walk.** WASD to a velocity on the ground plane relative to yaw: a pure function with tests
   (forward, diagonal not faster than straight, nothing pressed is still).
   `feat: walk with WASD`
4. **Remember where the player is.** `PlayerState` in `GameData`, written every frame, read by
   the spawn. Tests: a `GameData` survives a RON round trip, a save without `player` loads at
   the spawn.
   `feat: save where the player stands and looks`
5. **Docs.** `docs/flow.md` (Gameplay, the `GameData` field), `README.md` (features),
   `docs/template.md` (structure), this spec's status.
   `docs: document walking the playroom`

## What changed while building it

Nothing yet.
