# The playroom: a 2D level every 2D game starts from

Status: **approved, not built**.
Date: 2026-10-20
Bevy: 0.20.0
Branch: `2d/base`

## Goal

`Screen::Gameplay` is still a coloured background. On `2d/base` it becomes a place: entering
gameplay shows a test room seen from above, drawn in tiles: a stone floor, brick walls around
it, and a few crates and plants standing on the floor.

That is all `2d/base` does. Nothing moves and nothing can be controlled: no player, no input,
no collisions. Whoever walks the room, and how, is the business of the branches cut from here,
so they start from a level on screen without inheriting a body or a camera they would have to
take apart. It is the 2D twin of `3d/base`'s playroom, and shares its name so a reader moving
between the two genres finds the level in the same place.

## Decided

| Question | Decision | Why |
| --- | --- | --- |
| What the base holds | The room in tiles and nothing else. No player, no input, no collisions | The user's call, the same line `3d/base` drew: the base is the map in front of you. Anything that moves belongs to the branches cut from here |
| Drawing the tiles | Bevy's own `TilemapChunk`, one entity for the whole room, with its tiles in a `TilemapChunkTileData` | It ships with Bevy 0.20 (`bevy::sprite_render`, not in the prelude), so it moves to each new release with it and adds no crate. One chunk is one mesh, where a sprite per tile would be one entity each. `bevy_ecs_tilemap` would be a crate for what Bevy already does |
| Where the room's layout comes from | A `const` ASCII map in `playroom.rs`, one character per tile, read by a pure function into the chunk's tile data | The user's call over a map file or an editor. It needs no asset loader and no crate, and the reading is pure, so the tests can hold it: a game replaces it with Tiled or LDtk when it outgrows a hand-drawn room |
| Tile characters | `.` floor, `#` wall, `x` crate, `o` plant, the order of the tiles in `tiles.png` | One character per tile keeps the map readable as the room it draws. An unknown character is a hole (`None`), not a panic: a typo should cost one tile, never the game |
| The tile art | `assets/gameplay/tiles.png`: 16×16 px tiles stacked in one column, 16×64 px for the four. Made by the user with an image AI, then cut and scaled to 16 px by nearest neighbour | 16 px is the common pixel-art size. The crate and the plant are drawn over the floor in their own tile, since one chunk is one layer and nothing shows through it |
| Loading the tiles | The loading screen loads `tiles.png` for both destinations, as `3d/base` does with its glb. `load_with_settings` sets `array_layout: ImageArrayLayout::RowHeight { pixels: 16 }` and a nearest sampler | `TilemapChunk` reads an array texture, one tile per layer, and the loader can build it from the column with no system of its own. `RowHeight` rather than `RowCount` means a fifth tile is drawn, not counted in code. The sampler goes on this image only: `ImagePlugin::default_nearest()` would also make the loading photos and the intro frames blocky |
| Size on screen | Each tile drawn at 64 px (`tile_display_size`), four times its art | A whole multiple of 16 keeps every art pixel the same size. A room of about 16×10 tiles fits the default 1280×720 window |
| Camera | The screen camera that is already there (`main.rs`, `Camera2d` with `IsDefaultUiCamera`). No camera of its own | A 2D level is drawn by a `Camera2d`, and that one already exists. Its blur pass then blurs the room under the pause menu with no extra work. `3d/base` needed a second camera, and the whole dance around `Hdr` and `Msaa`, only because a `Camera3d` cannot be that one |
| Where the code lives | `src/screens/gameplay/playroom.rs`, registered by `gameplay.rs` like `pause.rs`, in place of `spawn_gameplay_screen` and its background | The same file `3d/base` uses. The full-window background is a UI node, drawn over the room, so it has to go |
| Bevy's features | `default-features = false` with `["2d", "ui", "audio", "jpeg", "wav"]` | The set `docs/template.md` already gives for a 2D game, the twin of `3d/base`'s trim |

## Risks to check while building

- **Where the chunk sits.** The room should be centred in the window. Check whether a
  `TilemapChunk` is centred on its `Transform` or starts at its corner, and place it by that.
- **Layers and indices.** A tile index past the last layer of the array texture should be
  caught by the reading function's tests, not by the GPU. A test ties the characters to the
  number of tiles.
- **The single-layer image.** `ImageArrayLayout` on an image exactly one tile tall would give
  a single layer, which Bevy may load as a plain 2D texture. Not a problem with four tiles,
  worth a line in `docs/template.md` for whoever trims the tileset down.

## Not in this design

A player, movement, collisions, a camera that follows anything, input of any kind. Objects
standing on the floor as sprites of their own (with y-sorting), a second chunk for a layer over
the floor, animated tiles, rooms larger than one chunk, and loading the room from Tiled or
LDtk. Those belong to the branches cut from here, or to a game.

## Implementation order

One commit per step. `just check` passes after each one, and `just run` shows it.

1. **Show the playroom.** `GameplayAssets` in `loading.rs` with the tileset handle, loaded for
   both destinations and counted by the bar. `playroom.rs` with the ASCII map, the pure
   function that reads it (and its tests), and the system that spawns the chunk with
   `DespawnOnExit(Screen::Gameplay)`, in place of the coloured background. Check the pause
   menu still blurs it.
   `feat: show the playroom in gameplay`
2. **Trim Bevy.** The features above in `Cargo.toml`.
   `build: build only the bevy features 2d uses`
3. **Docs.** `docs/flow.md` (Loading, Gameplay), `README.md` (features), `docs/template.md`
   (structure, the tileset's format, replacing the room), this spec's status.
   `docs: document the playroom`

After this spec, outside it: `main`'s `README.md` lists `2d/base` in its branch tree and
merges down, and `create-bevy-starter` stops marking 2D "coming soon" and clones `2d/base`.

## What changed while building it

Nothing yet.
