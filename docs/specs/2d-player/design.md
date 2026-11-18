# 2D player: a square that walks the room, and photo mode's 2D free camera

Status: **built** on 2026-11-19, approved the same day; waiting for the user's run.
Date: 2026-11-19
Bevy: 0.20.0
Roadmap: row 6 of `docs/roadmap/roadmap.md`, from its "2D player, as agreed" section, and the
2D free camera of its "Photo mode, as agreed" section.

## Goal

`2d/base` is a room of tiles seen from above, with nothing that moves. `2d/player` puts a
player in it: a square one tile big that walks with WASD, a camera that follows it once it
nears an edge of the screen, and its place kept in a save, as `3d/player` does for 3D. Photo
mode gets its 2D free camera, which pans and zooms. And `create-bevy-starter` asks, for a 2D
game, whether it starts with the player.

## Decided

| Question | Decision |
| --- | --- |
| Branch | A new one, `2d/player`, cut from `2d/base`, as `3d/player` is from `3d/base`. A new pair in `tools/sync.sh`'s tree, `2d/base:2d/player`, and a row in the README's branches |
| Files | `src/screens/gameplay/player.rs` on `2d/player`: the body, walking, the camera that follows, the save. `src/dev_tools/free_camera.rs` on `2d/player`: photo mode's 2D free camera, the name of `3d/player`'s, which it never meets |
| `2d/base` changes | `TILE_DISPLAY_PX` becomes `pub(crate)` in `playroom.rs`, so the body is one tile without a second spelling of 64; `playroom` becomes `pub(crate) mod`. Committed on `2d/base`, as `3d/base` opened its camera for row 5 |
| The body | A solid-colour square one tile big, `TILE_DISPLAY_PX` (64 px), a `Sprite` of `ACCENT`'s white, above the tiles. No animation. A new game spawns it in the middle of the room, the origin |
| Walking | Four game actions on the Controls page, Move Up, Move Down, Move Left and Move Right, on W, S, A and D: up the screen, not "forward", seen from above. Diagonals no faster. `PLAYER_WALK_SPEED_PX_PER_S`, 256 px/s, four tiles a second, which row 7's multiplier will scale. `Res<Time>`, so pause and slow motion hold it |
| Collisions | None: it walks through walls and crates (the roadmap's call). No physics crate on `2d/player` |
| The camera | The screen camera, the one `Camera2d` with `IsDefaultUiCamera`, which already draws the room: no second camera. A dead zone: it stays still while the body is more than a tile from every edge of the screen, and is pushed just enough to keep it there once it gets closer (`DEAD_ZONE_MARGIN_PX`, one tile). The room fits the 1280 × 720 window with a tile to spare, so today it moves only once the body walks out through a wall. The camera is pushed, not eased: no lag to tune |
| The UI | Bevy lays the UI out on the screen, not in the world: moving or zooming the camera moves no panel. Something to see in step 3's run, not to write |
| Leaving gameplay | The camera goes back to the origin, and its zoom to 1, so the menu's screens and the next game start from where they always did |
| Saved | `GameData::player`, a `PlayerState { position: Vec2 }`, as `3d/player`'s keeps its body; `serde(default)`, so a save written before it loads at the middle of the room. Continue resumes there, and the camera starts on it |
| Photo mode's 2D free camera | The walk actions pan the screen camera, at `FREE_CAMERA_PAN_PX_PER_S`, 512 px/s on screen, so a zoomed-out view crosses more of the room in the same time; the mouse wheel zooms, the camera's orthographic `scale`, each notch by `ZOOM_STEP` (1.1×), between `ZOOM_MIN` (0.25) and `ZOOM_MAX` (4). `Time<Real>`, the game being frozen. Leaving photo mode puts position and zoom back |
| The wheel | Not an action: `Binding` is a key or a mouse button, and a wheel has neither. Read from `AccumulatedMouseScroll`, with that reason beside it |
| The hint bar | `register_photo_hint`, from row 5: Move (the four walk keycaps) and Zoom (no keycap, the mouse icon, "Zoom: wheel") |
| Dead zone in photo mode | Off: the free camera moves the same camera, and the dead zone only follows the body while the game runs |
| The CLI | `create-bevy-starter` asks, after 2D, "Player:" with None and Top-down square; None clones `2d/base`, the other `2d/player`. `Kind::TwoD` becomes `Kind::TwoD(bool)`, `branch_for` and its test follow, the README's steps too. A release, 0.3.0, published by the user |

## Out of scope

- **Collisions and a physics crate**: walking through walls is what the roadmap asks for now.
- **Animation, a sprite sheet, facing**: a square.
- **A bigger map** to try the dead zone fully: the room stays as it is.
- **Row 7's speed multiplier**: walking and panning at their base speeds.
- **Easing the camera**: pushed by the dead zone, never late.

## Structure

### `2d/base`

`playroom.rs`'s `TILE_DISPLAY_PX` and the `playroom` module open to the crate. Nothing else.

### `2d/player`: `player.rs`

Registers the four walk actions. `spawn_player` on `OnEnter(Screen::Gameplay)`, at the
session's position, with `DespawnOnExit`. `walk_player` (`Pause::Off`) moves the `Transform`
by the pure `walk_velocity`. `follow_player` (`Pause::Off`, after walking) pushes the camera by
the pure `followed`, from the camera's place, the body's, and half the window's size less the
margin. `record_player_state` copies the position into the session, as in 3D. `OnExit` puts the
camera home.

### `2d/player`: `free_camera.rs`

Registers its two hints, keeps the camera's `Transform` and scale on entering photo mode, pans
and zooms while in it (`panned`, `zoomed`, pure), and puts both back on leaving.

### The CLI

`choices.rs`: `Kind::TwoD(bool)`, `branch_for`. `prompts.rs`: `ask_player()`. Its tests, the
README, a version bump.

## Text

The four walk labels, in every `main.ftl` of `2d/player`, in the 3D ones' style:

| Key | en | pt-BR | ja |
| --- | --- | --- | --- |
| `action-move-up` | Move Up | Mover para Cima | 上へ移動 |
| `action-move-down` | Move Down | Mover para Baixo | 下へ移動 |
| `action-move-left` | Move Left | Mover para a Esquerda | 左へ移動 |
| `action-move-right` | Move Right | Mover para a Direita | 右へ移動 |

The free camera's words are English, as every dev tool's: "Move", "Zoom".

## Tests

- `player.rs`: a diagonal is no faster; up is up the screen; the dead zone leaves the camera
  still while the body is inside it; the camera is pushed just enough once the body crosses it,
  on each side; a save from before `player` loads at the origin.
- `free_camera.rs`: a notch zooms by `ZOOM_STEP`; the zoom stops at both limits; a pan is faster
  in the world when zoomed out.
- The CLI: 2D with the player clones `2d/player`, without it `2d/base`.

## Implementation order

Each step compiles, passes `just check` and runs.

1. **`2d/base` opens the tile size**, and **`2d/player` is cut from it**, with the new pair in
   `tools/sync.sh` and the README's branch row (both on `main`, merged down). *Try:* `2d/player`
   runs as `2d/base` does.
2. **The square walks.** The body, the four actions and their labels, `walk_player`, the save.
   *Try:* WASD walks it, through the walls; Continue puts it back where it was.
3. **The camera follows.** The dead zone, the camera home on leaving. *Try:* out through a wall
   the view follows; the UI does not move; back in the menu nothing is shifted.
4. **The 2D free camera.** *Try:* in photo mode WASD pans, the wheel zooms, a photo shows it,
   and leaving puts the view back.
5. **The CLI.** The question, the branch, the tests, the README, 0.3.0. *Try:* a 2D game with
   the player made by the CLI runs.
6. **Docs.** `README.md`, `docs/flow.md`, `docs/template.md`, the roadmap's row 6, this spec's
   status.

## What changed while building it

- **Steps 1 to 6 were written at once**, as row 5's were, the user's call: no tasks file.
- **`gameplay` opens to the crate on `2d/player` too** (`pub(crate) mod gameplay` in
  `screens.rs`), as on `3d/player`: `save.rs` reads `PlayerState`, and the free camera the walk
  actions, from inside it.
- **The dead zone measures the body's edge**, not its middle: half the body is taken off the
  zone too, so "a tile from the edge" is the square's side, a tile from it.
- **The Zoom hint is the mouse icon and "Zoom"**, with no keycap, rather than "Zoom: wheel":
  the icon says where it is.
- **Leaving gameplay puts only the camera's place back**: its zoom is the free camera's,
  which already puts it back as photo mode ends, before gameplay can.
