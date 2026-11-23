# Prop editor: tasks

Roadmap row 13: its base on `main` (steps 1 to 5), then `3d/player` (steps 6 to 9) and
`2d/player` (step 10). The decisions and their reasons live in the spec,
`docs/specs/prop-editor/design.md`; this file only breaks its implementation order into tasks
small enough for one sitting each, with the facts each task needs.

## How to read this

- **Step** = one commit. Its title is the commit message.
- **Who**: *Claude* writes each step in pieces and the user reads and runs it; *you* is a
  `just run`, which only the user's eyes can do; *both* is the review at the end of a step
  (the checklist in `CLAUDE.md`: every name, the order of sections and systems, `just check`).
- **Facts**: the names that cannot be guessed, checked in the Bevy 0.20.0 and avian3d 0.8.0
  source and in this repository on 2026-11-23.
- **Where**: steps 1 to 5 are written and tried uncommitted on `main`, and reach the players by
  the merge down; steps 6 to 9 on `3d/player`, step 10 on `2d/player`. When to commit is the
  user's call each time.
- **Nothing unused, in either build**: what only the editor uses sits behind
  `#[cfg(feature = "dev")]`, and `map.rs`'s reading is in every build.
- Mark a task done by changing its `[ ]` to `[x]`.

## Review focus

What the spec implies and no step's *Try* shows; each has its test in the task named.

- **A map edited by hand** with `next_id` at or below an id it holds: reading raises `next_id`
  past the highest id, so a new prop never takes a used id (1.2).
- **A prop whose file is gone**: skipped with a warning, kept in the map, written back
  untouched by the next edit (1.3, 5.2).
- **Leaving gameplay with a prop held** (the pause menu's Load, Quit to Menu): the held prop is
  dropped unsaved, a picked-up one back where it was, and `PropEditor` is `Off` on the next
  game (2.3, 5.4).
- **A file dropped twice, or named like one already there**: the copy gets a number, never
  overwrites (4.1).
- **The map written while the game quits**: `write_atomic`, so the old map stays whole (5.2).

## Departures from the spec, to record in its "What changed while building it"

- **`PropFile` apart from `Prop`** (decided while building 1): a genre observes `PropFile` to
  show a prop, so the editor's new prop in the hand, which has a file and no id yet, shows the
  same way. `Prop` holds only the `PropId`.
- **No `Map` resource** (decided while building 5): the editor reads the map file at each edit,
  applies it and writes it back. `read_map` returns `None` for a file it cannot read, and an edit
  then writes nothing, so a map broken by hand is never written over.
- **Three edits, not four** (found while building 5): `Place`, `Update { id, transform, mode }`
  and `Delete`. `Move` and `SetMode` became `Update`: putting a prop down keeps its mode, choosing
  a mode keeps where it stands, and a `SetMode` nothing on `main` builds would have needed an
  `#[allow(dead_code)]`, which `just check` refuses.
- **A new mode is a new prop** (decided before step 6): `apply_map_edit` despawns the prop and
  spawns it again from the map, so its body is built from its mode as it spawns, rather than
  every genre taking apart the colliders a model's meshes got.
- **`PropKind`, a resource** (decided while building 3): the genre inserts the extensions it takes
  and the axis R turns about (`Dir3::Y` in 3D, `Dir3::Z` in 2D), in place of
  `register_prop_extensions`.
- **`read_dir` every second, not `load_folder`** (found while building 3): `load_folder` loads
  every model in the folder, and one file that fails to load fails the whole folder
  (`bevy_asset-0.20.0/src/server/mod.rs:990`). The window reads the folder as it opens and every
  `RESCAN_SECS` (1 s) while it is open, a `ponytail:` note beside it.
- **Dropped files are copied with the explorer's `copy_entry`** (decided while building 4), which
  already numbers a taken name, rather than with `write_atomic`.
- **`PropEditor` is a `SubStates` of `Pause::Off`** (decided while building 2): the pause menu,
  photo mode and leaving gameplay close it with no code of their own.
- **A new prop is placed when put down** (decided while building 5): taking one from a thumbnail
  writes nothing, so leaving gameplay or Esc with it in the hand leaves the map as it was.
- **The player ignores a prop's colliders being ready** (found while building 6): `spawn_player`
  spawned a player on every `ColliderConstructorHierarchyReady`, which each fixed prop also sends.
  Physics props wait, disabled (`RigidBodyDisabled`), for the level's, as the player does.
- **The pine's origin is 7 cm above its base, not 1.77 m** (found while building 6): its trunk's
  node sits 1.84 m up. Resting the lowest point on the surface stays, for models that need it.
- **The editor's camera flies at 8 m/s** (decided while building 7), photo mode's at 3: a map is
  built across the whole level. Bevy 0.20 combines run conditions with `or_else` and `and_then`.
- **The mode menu is a file of its own**, `prop_editor_3d/mode_menu.rs` (decided while building
  9), a column of buttons rather than the explorer's menu code. Its sheet is a `PropsSheet` too,
  so a click on an item never reaches the prop behind it.
- **In 2D a plain right click opens the depth menu** (decided while building 10): the 2D camera
  never turns, so the right button has no hold to tell apart. In Front and Behind are `PLAYER_Z`
  plus or minus `PROP_DEPTH_STEP` (0.5). The pine's picture is 128 px, from the front and 30°
  above, rendered by a Blender script; the model is mostly trunk, so that is what it shows.

## The actions at a glance

| Action | Default | Label on the Dev page and the panel | Heard | Step |
|--------|---------|-------------------------------------|-------|------|
| `prop_editor` | F10 | Prop Editor | in `Pause::Off`, not in photo mode | 2 |
| `turn_prop` | R | Turn Prop | while the editor is open and a prop is held | 5 |
| `delete_prop` | Delete | Delete Prop | while the editor is open | 5 |

`turn_prop` and `delete_prop` are inner keys (`register_inner_key`), shown on the window.

## Step 1: `feat: read the map file and spawn its props`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 1.1 `src/screens/gameplay/map.rs`, a `pub(crate)` module of `gameplay.rs`: `MapFile { next_id: u32, props: Vec<PlacedProp> }`, `PlacedProp { id: PropId, file: String, position: Vec3, rotation: Quat, scale: f32, mode: PropMode }`, `PropId(u32)`, `PropMode { Fixed, Physics, NoCollision }` with `Fixed` the default; all `Serialize`, `Deserialize`, `Clone`, `Default` where it fits | Claude | `ron::to_string` and `ron::from_str`, as `save.rs:243`; `Vec3` and `Quat` serialize as `PlayerState`'s `Vec3` does on `3d/player` |
| [x] | 1.2 `parse_map(text: &str) -> MapFile` (broken text is an empty map, logged) and `next_free_id`; tests: written and read back is the same map; broken text is an empty map; `next_id` at or below a held id is raised past the highest | Claude | |
| [x] | 1.3 `read_map() -> MapFile` from `assets/map.ron` (missing is empty), the `Map` resource, `spawn_map_props` on `OnEnter(Screen::Gameplay)`: one `(Prop { id, file }, PropMode, Transform, DespawnOnExit(Screen::Gameplay))` per placed prop whose file exists, a warning for each that does not; `props_to_spawn(map, exists: impl Fn(&str) -> bool)` and its test | Claude | `bevy::asset::io::file::FileAssetReader::get_base_path().join("assets")`, as `file_explorer/listing.rs:149`; the file name `MAP_FILE_NAME = "map.ron"` |
| [ ] | 1.4 `just check`; nothing to see in the Empty game. Review | both | |

## Step 2: `feat: open the prop editor on F10`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 2.1 `src/dev_tools/prop_editor.rs`: `PropEditor { Off, On }`, `init_state`, the `prop_editor` action (F10, "Prop Editor"), `toggle_prop_editor` from `Pause::Off` only; added to `dev_tools.rs` in the order of the keys, after recording (F9) and before the file explorer (Numpad 0) | Claude | `#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]`, as `RecordingState` (`recording.rs:99`); `register_action`, `Action`, `Binding::Key(KeyCode::F10)`, `Page::Dev` |
| [x] | 2.2 The drawer: an empty window at the bottom, `DespawnOnExit(PropEditor::On)`, `ClosesOnEsc` (so `3d/player` frees the cursor and stops looking around), the pointer going through around it | Claude | `drawer.rs` in the file explorer; `floating_root`, `card_glass` in `ui.rs`; the `DEV_TOOLS_Z_INDEX` layer |
| [x] | 2.3 `PropEditor` goes `Off` on `OnExit(Screen::Gameplay)` and when the pause menu opens; F3 does nothing in `PropEditor::On`, F10 nothing in `Pause::Photo` | Claude | `toggle_photo_mode`'s `run_if` in `photo_mode.rs` |
| [ ] | 2.4 `just run`: F10 opens and closes the drawer, Esc closes it, F3 does nothing while it is open. Review | both | |

## Step 3: `feat: list the props in the editor's window`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 3.1 `register_prop_extensions(app, &[&str])` and its resource; `prop_editor/library.rs`: `load_folder("props")`, the entries whose extension is registered, each with its size | Claude | `AssetServer::load_folder(path) -> Handle<LoadedFolder>`; it reloads when a file is added (`bevy_asset-0.20.0/src/server/mod.rs:942`); `std::fs::metadata(..).len()` |
| [x] | 3.2 `prop_editor/window.rs`: the segmented control (All, Favourites, Recent), the search field, the sort (Name, Size), the grid of names with a star; `PropThumbnail(Handle<Image>)` on an entry, shown when a genre puts one there | Claude | the search field: `EditableText` and `TextCursorStyle`, as `overlays/saves.rs:143` |
| [x] | 3.3 `prop_editor.ron` in the data folder: favourites and recents, `RECENT_PROPS = 20`; the star marks and unmarks | Claude | `game_data_dir()`, `write_atomic` in `disk.rs` |
| [x] | 3.4 Helpers and tests: search keeps names holding the text whatever their case; sort by name and by size; recents newest first, never twice, at most `RECENT_PROPS` | Claude | |
| [ ] | 3.5 `just run`: on `main` the grid is empty; with an extension registered by hand for the try, files show live as they are added. Review | both | |

## Step 4: `feat: drop files into the prop editor`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 4.1 `copy_dropped_props`, in `PropEditor::On`: a registered extension is copied into `assets/props/` with `write_atomic`, a taken name numbered as the explorer numbers a new folder; anything else logged; `free_name(wanted, taken)` and its tests | Claude | `MessageReader<FileDragAndDrop>`, `FileDragAndDrop::DroppedFile { window, path_buf }` (`bevy_window-0.20.0/src/event.rs:384`) |
| [ ] | 4.2 `just run`: a file dragged from the Windows explorer lands in `assets/props/`, a second copy gets a number. Review | both | |

## Step 5: `feat: place, move, turn, scale and delete props`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 5.1 `prop_editor/edits.rs`: `MapEdit { Place { file, transform }, Move { id, transform }, Delete { id }, SetMode { id, mode } }`, an `Event` | Claude | `#[derive(Event)]`, `commands.trigger(..)`, `On<MapEdit>`, as `StartGame` |
| [x] | 5.2 `apply_map_edit`: the edit to `MapFile` (pure: `applied(map, edit) -> MapFile`), the world (spawn, move, despawn, insert the mode), `write_map` with `write_atomic`; tests: each edit changes only its prop; place takes `next_id`; delete never frees an id; a prop whose file is gone survives other edits | Claude | `write_map` behind `cfg(feature = "dev")` |
| [x] | 5.3 `Held { from: Option<Transform> }` (`None` for a new prop): a click on a thumbnail spawns a held prop; R turns it by `PROP_TURN_STEP_DEG = 15.0`; the wheel scales it by `PROP_SCALE_STEP = 1.1` a notch, within `PROP_SCALE_MIN = 0.1` and `PROP_SCALE_MAX = 10.0`; tests of `turned` and `scaled` | Claude | `AccumulatedMouseScroll { unit, delta }`, `MouseScrollUnit::{Line, Pixel}`, as the 2D free camera reads them (`SCROLL_PX_PER_NOTCH`) |
| [x] | 5.4 Delete removes the held prop or `HoveredProp`'s; Esc with a prop held puts it back (`from`) or drops it, and closes the editor only the next time; leaving gameplay drops it unsaved; a placed prop goes into the recents | Claude | the Esc rule in `overlays.rs:123` |
| [ ] | 5.5 `just check`; on `main` nothing can be placed, as there is no world. Review | both | |

## Step 6 (on `3d/player`): `feat: show and collide the 3D props`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 6.1 `src/screens/gameplay/props.rs`: an observer on `Add<Prop>` puts `WorldAssetRoot(asset_server.load(file))` and the body of its `PropMode`; `Fixed`: `RigidBody::Static` and `ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)`; `Physics`: `RigidBody::Dynamic` and `ConvexHullFromMesh`; `NoCollision`: nothing | Claude | `WorldAssetRoot` replaces `SceneRoot` in 0.20 (`bevy_world_serialization-0.20.0/src/components.rs:23`); `RigidBody` is immutable, swapped by inserting it again |
| [x] | 6.2 The pine: `assets/props/douglas_pine.glb` from the user's `douglaspinetree.glb`, and `assets/map.ron` with one pine, `Fixed`, beside `PLAYER_SPAWN_POINT`, its base on the floor | Claude | the pine's origin is 1.77 m above its base (its mesh's `min.y` is −1.771) |
| [ ] | 6.3 `just run`: the pine stops the player; three pines by hand, one per mode: stops, pushed, walked through. Review | both | |

## Step 7 (on `3d/player`): `feat: fly the camera in the prop editor`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 7.1 On `OnEnter(PropEditor::On)` `PlayroomCamera` leaves the body (its `ChildOf` removed, its `GlobalTransform` kept as its `Transform`); on exit, back on the body where it was | Claude | `FreeCameraHome` in `free_camera.rs` keeps the place; `ChildOf(entity)` |
| [x] | 7.2 `free_camera.rs`'s moves run in `PropEditor::On` too; its turn only while the right button is held | Claude | `in_state(Pause::Photo).or(in_state(PropEditor::On))` |
| [x] | 7.3 The player neither walks nor looks while the editor is open: `walk_player` and `look_around` gated on `PropEditor::Off`, behind `cfg(feature = "dev")`; `is_cursor_locked` holds the cursor while the right button turns the camera | Claude | the `Speedometer` lines in `player.rs` show the `cfg` pattern |
| [ ] | 7.4 `just run`: F10, fly away, turn with the right button, F10: back in the body, looking where it looked. Review | both | |

## Step 8 (on `3d/player`): `feat: pick and place 3D props with thumbnails`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 8.1 `src/dev_tools/prop_editor_3d.rs`: registers `glb`; a thumbnail per model, one at a time, `THUMBNAIL_SIZE_PX = 128`, on a render layer of its own with its own light, the camera off after its frame; `framing(aabbs) -> Transform` and its test (a model of any size whole in view) | Claude | `RenderTarget::Image(handle.into())`, a component (`bevy_camera-0.20.0/src/camera.rs:891`); `Image::new_target_texture(w, h, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb))`; `Camera { is_active }`; `Aabb` on each glTF mesh (`bevy_gltf-0.20.0/src/loader/mod.rs:1716`) |
| [x] | 8.2 `HoveredProp` from the cursor's ray; the held prop rests its lowest point where the ray meets the world, or `HELD_PROP_DISTANCE_M` ahead; a held `Physics` prop is `RigidBody::Kinematic` until put down; `resting_on(hit, aabb_min_y) -> Vec3` and its test | Claude | `Camera::viewport_to_world(&GlobalTransform, Vec2) -> Result<Ray3d, _>`; `MeshRayCast::cast_ray(ray, &MeshRayCastSettings) -> &[(Entity, RayMeshHit)]` with a `filter` off the held prop |
| [ ] | 8.3 `just run`: the pine's thumbnail shows; place one on a table, turn it, scale it, move it, delete it, quit and come back: where it was put. Review | both | |

## Step 9 (on `3d/player`): `feat: choose a 3D prop's mode`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 9.1 A quick right click (under `RIGHT_CLICK_MAX_SECS`, the mouse barely moved) on `HoveredProp` opens the menu, Fixed, Physics, No Collision, the current one checked; a pick sends `MapEdit::SetMode` | Claude | the explorer's menu look and placement (`file_explorer/menu.rs:271`); if it becomes their third user, they move to `dev_tools.rs` |
| [ ] | 9.2 `just run`: a pine made physics is pushed, then starts back where it was placed after quitting. Review | both | |

## Step 10 (on `2d/player`): `feat: place 2D props`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 10.1 `src/screens/gameplay/props.rs`: an observer on `Add<Prop>` puts `Sprite::from_image(..)`; the pine's PNG, rendered from above by a Blender script, and a map with one pine In Front | Claude | Blender at `D:/blender`, scriptable |
| [x] | 10.2 `src/dev_tools/prop_editor_2d.rs`: registers `png` and `jpg`; each image its own `PropThumbnail`; the free camera in the editor; the wheel scales with a prop held and zooms without one; picking by the sprite's rectangle; the held prop at the cursor; the depth menu, In Front and Behind (`PROP_FRONT_Z`, `PROP_BEHIND_Z` around the player's z); tests: wheel scale against zoom, In Front draws over the player | Claude | `zoom_free_camera` in `2d/player`'s `free_camera.rs`; `Camera::viewport_to_world_2d` |
| [ ] | 10.3 `just run`: the player walks behind the pine; drop a png, place, turn, scale, In Front, zoom with nothing held. Review | both | |

## Step 11: `docs: mark the prop editor built`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 11.1 `README.md` (F10 in the dev keys), `docs/flow.md` (`PropEditor` beside `Pause`, the drawer's Z index), `docs/template.md` (the map file, `assets/props/`, the pine), the roadmap's row 13, the spec's status and "What changed" | Claude | |
| [ ] | 11.2 Sync; `just run` on `main`, `3d/player` and `2d/player` | you | |
