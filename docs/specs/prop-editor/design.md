# Prop editor: place, move, turn, scale and delete props on F10, kept in a map file

Status: **designed** 2026-11-23, not started.
Date: 2026-11-23
Bevy: 0.20.0
Roadmap: row 13 of `docs/roadmap/roadmap.md`, from its "Prop editor, as agreed so far"
section, and the answers of 2026-11-23 below.

## Goal

F10 opens a props window and takes the camera off the player, so the map can be built from
anywhere while the game keeps running. A prop is a file dropped into `assets/props/` (a `.glb`
in 3D, an image in 2D). A click on its thumbnail sticks a new one to the mouse; a click in the
world puts it down; R turns it, the mouse wheel scales it, Delete removes it. Every edit is
written at once to `assets/map.ron`, which the game reads on entering gameplay, release builds
included, so each prop starts where it was put. In 3D each placed prop has a mode: fixed,
physics (the player pushes it) or no collision.

It is for building the map and the trailer scenes (row 14), and, once multiplayer lands
(row 18), for building them with friends.

## Why now, and why skipping rows 10 to 12

Rows 10 (character controller), 11 (ladders) and 12 (doors) wait: how a player moves and climbs
depends on the game made from the template (the user's call, 2026-11-23). The roadmap put the
editor after doors "since a door is a kind of prop". Nothing is lost by the swap: no door exists
yet, so when row 12 comes its doors can be born as props.

## Decided

| Question | Decision |
| --- | --- |
| What can be edited | Only props placed from `assets/props/`. The playroom's own level (the Blender glb) stays as it is and never enters the map file (the user's call) |
| The map file | `assets/map.ron`, our own RON through `serde` and `ron`, as `save.rs`, `settings.rs` and the explorer's trash already are, written with `write_atomic`. Not a Bevy scene (`bevy_scene`'s `.scn.ron`): that file spells Bevy's own type paths, which move between versions (0.20 itself is mid-move to BSN), so a committed map would break on every Bevy upgrade of the template (the user's call, approach A) |
| Where the map file is | In `assets/`, committed: the map is part of the game. Dev builds write it, release builds only read it. Found the way the file explorer finds the assets folder: `FileAssetReader::get_base_path().join("assets")` (`file_explorer/listing.rs:149`) |
| What the map holds | `MapFile { next_id: u32, props: Vec<PlacedProp> }`; `PlacedProp { id: PropId, file: String, position: Vec3, rotation: Quat, scale: f32, mode: PropMode }`. `file` is the asset path from `assets/` (`props/chair.glb`), the way every other `.ron` under `assets/` names an asset, so the file explorer's usage guard (`file_explorer/usage.rs`, which searches `.ron` files under `assets/` for the path) asks before deleting or renaming a prop the map places, with no code of ours. One struct for both genres: 2D keeps its depth in `position.z` and turns about Z |
| A prop whose file is gone | Skipped with a warning in the log, and kept in the map: a file renamed back, or put back, brings the prop back where it was. The editor's window does not list it, so it cannot be placed again until its file returns |
| Prop ids | `PropId(u32)`, from `next_id`, never reused. An `Entity` differs on every machine, so "move prop 7" is what an edit names, ready for multiplayer to send (the user's call) |
| One path for every edit | `MapEdit`: `Place`, `Move`, `Delete`, `SetMode`. The editor's clicks only ever produce one; one observer applies it to the world and to `MapFile` and writes the file. Multiplayer later sends the same `MapEdit` to the host first, without touching the editor's logic (the user's call). No networking is built now |
| Physics during play is never saved | The map is where props were put. A box pushed across the room starts back where it was placed next time. Picking up a pushed box in the editor lifts it from where it is now, and putting it down saves that new place (the user's call) |
| Modes, 3D | Per placed prop, not per file, so the same chair can be fixed in one spot and loose in another (the user's call). `PropMode::Fixed`: `RigidBody::Static` and `ColliderConstructorHierarchy` of `TrimeshFromMesh`, as the level. `PropMode::Physics`: `RigidBody::Dynamic` and `ConvexHullFromMesh`, since a trimesh is for what does not move. `PropMode::NoCollision`: no body. Every new prop starts `Fixed`. The player is already a dynamic body (`player.rs:133`), so pushing needs no code |
| Modes, 2D | None for now: `2d/player` has no physics crate and its square goes through everything. 2D props are scenery; modes come when a 2D game has collision, which depends on the game (the user's call, option A). The 2D map still writes `mode`, always `NoCollision` |
| Depth, 2D | Per placed prop, chosen like a 3D mode: a quick right click opens a menu with In Front and Behind, the player being what a prop is in front of or behind (the user's call: an option, as in 3D). It is `position.z`, set to `PROP_FRONT_Z` or `PROP_BEHIND_Z` around the player's z, so it needs no field of its own and goes through `MapEdit::Move`. A new prop starts Behind, as floor and scenery |
| Choosing a mode | A quick right click on a prop in the world opens a menu (Fixed, Physics, No Collision, the current one checked), in the file explorer's menu look. Holding the right button turns the camera, as in Unreal (the user's call) |
| Scale | The mouse wheel, while a prop is stuck to the mouse, in fixed steps (`PROP_SCALE_STEP`, 1.1 a notch, between `PROP_SCALE_MIN` and `PROP_SCALE_MAX`), one uniform `f32` saved with the prop (the user's call). In 2D the wheel zooms the free camera when nothing is held, as photo mode's does, and scales when a prop is held |
| The props window | Bottom of the screen, as Unreal's content drawer, in the look of a Finder icon view: a segmented control (All, Favourites, Recent), a search field (the save screens' `EditableText` field), sorting by name or by file size, and a grid of thumbnails with their names and a star. The pointer goes through around it, as the explorer's drawer lets it (`drawer.rs`) |
| Favourites and recents | The developer's, not the game's: `prop_editor.ron` in the data folder, written with `write_atomic`. The star on a thumbnail marks and unmarks. Recent is the last `RECENT_PROPS` (20) files placed, newest first |
| Where props come from | `load_folder("props")`. Bevy 0.20 reloads a `LoadedFolder` when a file is added, removed or moved in it (`bevy_asset-0.20.0/src/server/mod.rs:942`, `AddedAsset` reloads the parent folders at :1991), so a new file shows live with no code of ours. File sizes for sorting come from `std::fs::metadata` on the file |
| Formats | Each genre registers what it accepts, as tools register photo mode's hints: `register_prop_extensions(app, &["glb"])` in 3D, `&["png", "jpg"]` in 2D (`2d/player` builds with the `jpeg` feature). On `main`, the Empty game, nothing is registered: F10 opens a window with nothing in it, as speed's capsule shows `—` with no player |
| Dropping files | `FileDragAndDrop::DroppedFile { path_buf, .. }`, read with `MessageReader<FileDragAndDrop>` (`bevy_window-0.20.0/src/event.rs:384`), only while the editor is open. A file whose extension is registered is copied into `assets/props/` with `write_atomic`, so a crash never leaves half a model there; a name already taken gets a number, as the explorer names a new folder. Anything else is refused with a line in the log |
| Thumbnails, 3D | Each model is rendered once into a small `Image` (`THUMBNAIL_SIZE_PX`, 128) by a camera of its own: `RenderTarget::Image(handle.into())`, a separate component in 0.20 (`bevy_camera-0.20.0/src/camera.rs:891`), with an image from `Image::new_target_texture`, on a render layer of its own with its own light, so neither the playroom nor its light shows in it. It is framed from the model's `Aabb`s, which the glTF loader puts on each mesh (`bevy_gltf-0.20.0/src/loader/mod.rs:1716`), joined through their `GlobalTransform`s. One at a time; the camera is turned off (`is_active: false`) after its frame. Kept in memory for the run, not on disk: a cache earns its place only once a folder is big enough that opening the window waits |
| Thumbnails, 2D | The image itself |
| Holding a prop | A click on a thumbnail spawns a new prop stuck to the mouse; a click on a placed prop picks it up. In 3D the held prop sits where the ray from the cursor meets the world (`Camera::viewport_to_world`, then `MeshRayCast::cast_ray`, filtered off the held prop itself), its lowest point resting there, not its origin: the sample tree's origin is 1.77 m above its base, so a prop put down by its origin would sink into the floor. The lowest point comes from the same joined `Aabb`s the thumbnails frame. When the ray meets nothing, the prop sits at `HELD_PROP_DISTANCE_M` in front of the camera. Mesh ray casts, not avian's, so a no-collision prop can be picked too. In 2D the held prop sits at the cursor's point in the world. Another left click puts it down |
| A held physics prop | Held as a kinematic body (`RigidBody::Kinematic`), so it neither falls from the mouse nor is pushed while held, and goes back to its mode when put down. `RigidBody` is immutable in avian 0.8 and is swapped by inserting it again, which moves its colliders over (`collider_tree/update.rs:123`) |
| R, Delete, Esc | R turns the held prop by `PROP_TURN_STEP_DEG` (15°) about the vertical (in 2D, in the plane). Delete removes the held prop or the one under the mouse. Esc while holding puts the prop back where it was picked up, or drops a new one; the next Esc closes the editor |
| The camera, 3D | `PlayroomCamera` leaves the body for the world as the editor opens (its `ChildOf` removed, its place kept), so the body moving under physics never drags it, and goes back on the body, where it was, as the editor closes. WASD, Space and Shift fly it whether the right button is held or not; the right button held turns it and hides the cursor; released, the cursor shows to click props. Photo mode's free camera (`dev_tools/free_camera.rs`) holds the moves and the turn: its systems run in `PropEditor::On` too |
| The camera, 2D | Photo mode's 2D free camera: the walk keys pan, the wheel zooms unless a prop is held |
| The player while editing | It neither walks nor looks: its walk and its look are dev-only gated on `PropEditor::Off`, as speed's line in the walk is. Physics still moves it, so a falling body still lands |
| The editor's state | `PropEditor` (`Off`, `On`), a state of its own beside `Pause`, as `RecordingState` is (`docs/flow.md`): the game keeps running, so it is not a pause. F10 enters it from `Pause::Off` only. Leaving gameplay, or the pause menu opening, sets it `Off`. Photo mode and the editor never run together, since both drive `PlayroomCamera`; F3 does nothing while the editor is open and F10 nothing in photo mode. Recording is allowed: it films the game alone |
| Branches | `main`: the map file, the props it spawns as plain entities (`Prop` and its `PropMode`, with a `Transform` and nothing to see), the editor's state, the window, the library, dropping files, favourites and recents, and `MapEdit`. It makes sense in an Empty game. `3d/player`: what a 3D prop looks like and how it collides, thumbnails, the camera, picking and placing, the mode menu. `2d/player`: the sprite, picking and placing, the camera. Not `3d/base` nor `2d/base`: avian and the free cameras live on the player branches, and a prop editor with no camera to fly is no use |
| The sample prop | The template ships one: the user's Douglas pine (`douglaspinetree.glb`, 105 KB, made in Blender), so a game cloned from a player branch shows the editor working. `3d/player` commits it as `assets/props/douglas_pine.glb` and an `assets/map.ron` with one pine, fixed, beside the spawn point. `2d/player` commits a PNG of the same pine, rendered from above by a Blender script (`D:/blender`), and a map with one pine In Front, so the player walks behind it. `main` commits neither: its Empty game has no props, and a missing map is an empty one, so nothing from `main` ever conflicts with a player branch's map. A game that does not want the pine deletes it in the editor and its file; `create-bevy-starter` changes nothing |
| Release builds | `map.rs` and the props it spawns are in every build: a released game shows its props. The editor is a dev tool behind `#[cfg(feature = "dev")]` |
| Multiplayer | Not now. `PropId` and the single `MapEdit` path are the two doors kept open. A pushed box's live position is replication's, row 18's |

## Out of scope

- **Editing the level itself**: the Blender glb stays fixed.
- **Modes in 2D**, until a 2D game has collision.
- **Moving with gizmos** (arrows, rings), snapping to a grid, scaling along one axis, undo.
- **A thumbnail cache on disk.**
- **Networking**: only the two doors above.

## Structure

### `main`

- `src/screens/gameplay/map.rs` (every build): `MapFile`, `PlacedProp`, `PropId`, `PropMode`,
  `read_map` and `write_map` (dev only), the `Prop` component (`id`, `file`), and
  `spawn_map_props` on entering gameplay: one `(Prop, PropMode, Transform,
  DespawnOnExit(Screen::Gameplay))` per placed prop. A missing file is an empty map; a file
  that fails to read is logged and treated as empty, never a panic. Its `Map` resource holds
  the `MapFile` the game is showing.
- `src/dev_tools/prop_editor.rs` and its folder: the `PropEditor` state and F10;
  `register_prop_extensions`; `window.rs` (the drawer, tabs, search, sort, grid, star);
  `library.rs` (the `load_folder`, sizes, dropping files, `prop_editor.ron`); `edits.rs`
  (`MapEdit`, `apply_map_edit`, `Held`, R, the wheel, Delete, Esc). What a genre fills in: a
  thumbnail per file (`PropThumbnail` on the library's entry), and which prop is under the
  mouse (`HoveredProp`), and where the held one goes.

### `3d/player`

- `src/screens/gameplay/props.rs` (every build, this branch): an observer on `Add<Prop>` puts
  the model (`WorldAssetRoot`) and the body of its `PropMode` on it; `SetMode` re-inserts the
  body.
- `src/dev_tools/prop_editor_3d.rs`: thumbnails, the camera off the body, the cursor held while
  the right button is (`first_person.rs`'s `is_cursor_locked` learns the editor), picking and
  placing by mesh ray casts, the mode menu. Registers `glb`.

### `2d/player`

- `src/screens/gameplay/props.rs`: an observer on `Add<Prop>` puts the `Sprite` on it.
- `src/dev_tools/prop_editor_2d.rs`: the free camera in the editor, the wheel shared with the
  zoom, picking by the sprite's rectangle, placing at the cursor, the depth menu (In Front,
  Behind). Registers `png` and `jpg`.

## Text

English literals, as every dev tool's: "Props", "All", "Favourites", "Recent", "Search",
"Name", "Size", "Fixed", "Physics", "No Collision", and the F10 label "Prop Editor". The
props themselves carry no words the player reads.

## Tests

- `map.rs`: a map written and read back is the same map; a missing file is an empty map; a
  broken file is an empty map; a new prop takes `next_id` and the next one the id after it;
  deleting a prop never frees its id.
- `edits.rs`: each `MapEdit` applied to a `MapFile` gives the file expected (place adds, move
  changes only that prop, delete removes only it, set mode changes only the mode); turning
  steps by `PROP_TURN_STEP_DEG`; scaling stops at `PROP_SCALE_MIN` and `PROP_SCALE_MAX`.
- `library.rs`: search keeps the names that hold the text, whatever their case; sort by size
  and by name; recents put the newest first, never twice, and keep `RECENT_PROPS`; a dropped
  name already taken gets a number; an extension not registered is refused.
- `prop_editor_3d.rs`: the thumbnail framing puts a model of any size whole in view.
- `prop_editor_2d.rs`: the wheel scales with a prop held and zooms without one; In Front draws
  over the player and Behind under it.
- `prop_editor_3d.rs`: a prop whose origin is above its base rests its base on the surface.

## Implementation order

Each step compiles, passes `just check` and runs. Steps 1 to 5 are committed on `main` and reach
the players by the merge down; 6 to 9 on `3d/player`; 10 on `2d/player`.

1. **The map file, on `main`.** `map.rs`: the types, reading, spawning `Prop`s on entering
   gameplay. *Try:* nothing to see: the Empty game draws no prop, so the tests carry this
   step.
2. **F10 and the window.** `PropEditor`, F10, the empty drawer, Esc and leaving gameplay close
   it, F3 and F10 exclude each other. *Try:* F10 opens and closes it, and the pause menu closes
   it.
3. **The library.** `load_folder("props")`, registered extensions, the grid with names and
   sizes, tabs, search, sort, star, `prop_editor.ron`. *Try:* on `main` nothing shows; a fake
   extension registered by hand lists files live as they are added.
4. **Dropping files.** *Try:* a file dragged from the explorer lands in `assets/props/`, a
   second copy gets a number.
5. **Edits.** `MapEdit`, `apply_map_edit`, `Held`, R, the wheel, Delete, Esc, recents. *Try:*
   the tests; on `main` there is no world to put a prop in.
6. **3D props.** `props.rs`: the model and the three bodies, the pine and its map. *Try:* the
   pine stands by the spawn point and stops the player; a map edited by hand to three pines,
   one per mode: the player stops at one, pushes one, walks through one (then back to one).
7. **The 3D editor's camera.** Off the body and back, fly, turn on the right button, the player
   frozen. *Try:* F10, fly away, F10 again: back in the body, looking where it looked.
8. **3D thumbnails, picking and placing.** *Try:* drop a glb, see its thumbnail, place it on
   a table, turn it, scale it, move it, delete it, quit and come back: it is where it was put.
9. **The mode menu.** *Try:* a quick right click on a box makes it physics, push it, quit:
   it starts back where it was placed.
10. **2D.** `props.rs` and `prop_editor_2d.rs` on `2d/player`, the pine's PNG and its map.
    *Try:* the player walks behind the pine; drop a png, place it, turn it, scale it, set it
    In Front, zoom with nothing held.
11. **Docs.** `README.md`, `docs/flow.md` (`PropEditor` beside `Pause`, the drawer's Z
    index), `docs/template.md` (how a game reads its map, where props live), the roadmap's
    row 13, this spec's status.

## Left to the builder

- How the 3D cursor is held while the right button turns the camera, and how long a quick
  right click may be (`RIGHT_CLICK_MAX_SECS`) and how far the mouse may move in it.
- The thumbnail light and camera angle, tuned by eye.
- The density of a physics prop, so a chair is pushed and a crate is heavy (avian's default is
  1 kg/m³), tuned by eye.
- Whether the explorer's menu placement and its `ASSETS_DIR_NAME` move to `dev_tools.rs` as
  their third user arrives (CONTRIBUTING: a third user moves a piece to where they all reach
  it).

## What changed while building it

Nothing yet.
