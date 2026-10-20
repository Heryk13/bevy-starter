use bevy::prelude::*;
use bevy::sprite_render::{TileData, TilemapChunk, TilemapChunkTileData};

use crate::screens::Screen;
use crate::screens::loading::{GameplayAssets, TILE_ART_PX};

// ==== Config =================================================================

// ---- room: one character per tile, read by `tile_index` ---------------------
// the blank line after the opening quote keeps every row lined up in the source,
// and `room_size` skips it
const ROOM_MAP: &str = "
################
#o............o#
#..xx.....#....#
#..x......#....#
#.........#..o.#
#.....#####....#
#..............#
#.o.........xx.#
#o..........xxo#
################
";

// ---- size on screen: a whole multiple keeps every art pixel the same size ---
const TILE_SCALE: u32 = 4;

// ---- derived: a tile on screen, in px ---------------------------------------
const TILE_DISPLAY_PX: u32 = TILE_ART_PX * TILE_SCALE;

// ==== Plugin =================================================================

/// Shows the playroom: a room of tiles under the screen camera.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_playroom);
    app.add_systems(OnExit(Screen::Gameplay), unload_gameplay_assets);
}

// ==== Systems ================================================================

/// Spawns the playroom's tiles, already loaded by the loading screen, centred on the
/// screen camera. It needs `GameplayAssets`, so gameplay is entered through the loading
/// screen, never straight from the menu.
fn spawn_playroom(mut commands: Commands, gameplay_assets: Res<GameplayAssets>) {
    let size = room_size(ROOM_MAP);
    commands.spawn((
        TilemapChunk {
            chunk_size: size,
            tile_display_size: UVec2::splat(TILE_DISPLAY_PX),
            tileset: gameplay_assets.tiles.clone(),
            ..default()
        },
        TilemapChunkTileData::from_y_down_tiles(size, room_tiles(ROOM_MAP)),
        DespawnOnExit(Screen::Gameplay),
    ));
}

/// Drops the playroom's handles once gameplay ends, so Bevy can free the tiles' memory
/// while the player is in the menu. A load from the pause menu loads them again.
fn unload_gameplay_assets(mut commands: Commands) {
    commands.remove_resource::<GameplayAssets>();
}

// ==== Helpers ================================================================

/// Returns the layer of `tiles.png` a map character stands for, or `None` for a
/// character that is not a tile, which leaves a hole rather than stopping the game.
fn tile_index(character: char) -> Option<u16> {
    match character {
        '.' => Some(0),
        '#' => Some(1),
        'x' => Some(2),
        'o' => Some(3),
        _ => None,
    }
}

/// Returns the tiles of an ASCII map in reading order, left to right and top row first,
/// the order `TilemapChunkTileData::from_y_down_tiles` takes.
fn room_tiles(map: &str) -> Vec<Option<TileData>> {
    let mut tiles = Vec::new();
    for line in map.lines() {
        for character in line.chars() {
            tiles.push(tile_index(character).map(TileData::from_tileset_index));
        }
    }
    tiles
}

/// Returns the size of an ASCII map in tiles, skipping blank lines such as the one
/// after the opening quote of `ROOM_MAP`, which hold no tiles.
fn room_size(map: &str) -> UVec2 {
    let width = map
        .lines()
        .find(|line| !line.is_empty())
        .map(|line| line.chars().count())
        .unwrap_or(0);
    let height = map.lines().filter(|line| !line.is_empty()).count();
    UVec2::new(width as u32, height as u32)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_the_wall_tile() {
        assert_eq!(tile_index('#'), Some(1));
    }

    #[test]
    fn unknown_character_is_a_hole() {
        assert_eq!(tile_index('!'), None);
    }

    #[test]
    fn every_map_character_becomes_one_tile() {
        assert_eq!(room_tiles("#.\nx#").len(), 4);
    }

    #[test]
    fn room_tiles_start_at_the_top_left() {
        let tiles = room_tiles("#.\nx#");
        assert_eq!(tiles[0].map(|tile| tile.tileset_index), Some(1));
        assert_eq!(tiles[2].map(|tile| tile.tileset_index), Some(2));
    }

    #[test]
    fn blank_lines_do_not_count_in_the_room_size() {
        assert_eq!(room_size("\n###\n#.#\n"), UVec2::new(3, 2));
    }

    #[test]
    fn room_has_one_tile_per_cell() {
        let size = room_size(ROOM_MAP);
        assert_eq!(room_tiles(ROOM_MAP).len(), (size.x * size.y) as usize);
    }
}
