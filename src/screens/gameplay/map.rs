use bevy::asset::io::file::FileAssetReader;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::save::Session;
use crate::screens::Screen;

// ==== Config =================================================================

// in the assets folder, committed: the default map is part of the game, not the player's
const MAP_FILE_NAME: &str = "map.ron";
const ASSETS_DIR_NAME: &str = "assets";

// ==== Plugin =================================================================

/// The props of the game in progress, kept in its save, release builds included: on entering
/// gameplay a game with none yet, a new one or one saved before props were, gets a copy of the
/// default map, `assets/map.ron`, and each prop spawns as a plain entity, a `Prop` with its
/// `PropFile`, its `PropMode` and its `Transform`. What it looks like and how it collides are
/// the genre's, which observe `PropFile`.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_map_props);
}

// ==== Map ====================================================================

/// A map: every placed prop, and the id the next one takes. The default map is one, in
/// `assets/map.ron`, and so is the map each save keeps.
#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub(crate) struct MapFile {
    pub(crate) next_id: u32,
    pub(crate) props: Vec<PlacedProp>,
}

/// One prop where the editor put it, and where physics left it since, if it moved. `file` is
/// its asset path from `assets/` (`props/douglas_pine.glb`), the way every `.ron` under
/// `assets/` names an asset, so the file explorer asks before deleting a file the map places.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub(crate) struct PlacedProp {
    pub(crate) id: PropId,
    pub(crate) file: String,
    pub(crate) position: Vec3,
    pub(crate) rotation: Quat,
    pub(crate) scale: f32,
    pub(crate) mode: PropMode,
    // only a physics prop that moved has one, and the default map none. Always written, as
    // `None` too: the network writes every field, in order, with nothing to say one is left out
    #[serde(default)]
    pub(crate) resting: Option<PropRest>,
}

/// Where physics left a prop: kept in the save, so a pushed box loads where it came to rest.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub(crate) struct PropRest {
    pub(crate) position: Vec3,
    pub(crate) rotation: Quat,
}

/// A placed prop's id, the same on every machine, where an `Entity` is not: what an edit
/// names, so multiplayer can send it. Never reused.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct PropId(pub(crate) u32);

/// How a placed prop collides: in place, pushed around, or not at all. 2D has no physics, so
/// its props are always `NoCollision`.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) enum PropMode {
    #[default]
    Fixed,
    Physics,
    NoCollision,
}

impl PlacedProp {
    /// Returns where the prop stands, as its entity's `Transform`: where physics left it, or
    /// where it was placed.
    pub(crate) fn transform(&self) -> Transform {
        let (translation, rotation) = match self.resting {
            Some(rest) => (rest.position, rest.rotation),
            None => (self.position, self.rotation),
        };
        Transform {
            translation,
            rotation,
            scale: Vec3::splat(self.scale),
        }
    }
}

impl MapFile {
    /// Returns the map as the default map keeps it: every prop where it was placed, none where
    /// physics left it, so a test played before writing it never rearranges it. Only the
    /// editor writes the default map, and only dev builds have the editor.
    #[cfg(feature = "dev")]
    pub(crate) fn as_placed(&self) -> MapFile {
        let mut map = self.clone();
        for placed in &mut map.props {
            placed.resting = None;
        }
        map
    }
}

// ==== Components =============================================================

/// Marks a placed prop, by the id the map knows it by.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Prop(pub(crate) PropId);

/// The asset a prop shows, its path from `assets/`: what a genre observes to put the model or
/// the image on it. The editor's prop held from a thumbnail has it before it has a `Prop`.
#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub(crate) struct PropFile(pub(crate) String);

// ==== Systems ================================================================

/// Spawns every prop of the game in progress whose asset is on disk, a game with none yet
/// getting a copy of the default map first, and warns about each whose asset is gone: that one
/// stays in the map, so the file put back brings it back.
fn spawn_map_props(mut commands: Commands, mut session: ResMut<Session>) {
    let map = session
        .game
        .map
        .get_or_insert_with(|| read_map().unwrap_or_default());
    let assets = assets_dir();
    let (present, missing) = props_to_spawn(map, |file| assets.join(file).is_file());
    for placed in present {
        commands.spawn(prop_bundle(placed));
    }
    for placed in missing {
        warn!(
            "the map places {:?}, which is not in assets/: skipped",
            placed.file
        );
    }
}

// ==== Helpers ================================================================

/// Returns a placed prop's entity, as the map spawns it and the editor's edits do: gone with
/// gameplay.
pub(crate) fn prop_bundle(placed: &PlacedProp) -> impl Bundle {
    (
        Prop(placed.id),
        PropFile(placed.file.clone()),
        placed.mode,
        placed.transform(),
        DespawnOnExit(Screen::Gameplay),
    )
}

/// Returns the game's assets folder on disk, where the map file and the props are.
pub(crate) fn assets_dir() -> PathBuf {
    FileAssetReader::get_base_path().join(ASSETS_DIR_NAME)
}

/// Returns the map file's path.
pub(crate) fn map_path() -> PathBuf {
    assets_dir().join(MAP_FILE_NAME)
}

/// Reads the map file: an empty map when there is none, as in an Empty game, and `None`, with
/// an error logged, when it cannot be read, so the editor never writes over a map it could
/// not understand.
pub(crate) fn read_map() -> Option<MapFile> {
    match std::fs::read_to_string(map_path()) {
        Ok(text) => parse_map(&text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Some(MapFile::default()),
        Err(err) => {
            error!("could not read the map file: {err}");
            None
        }
    }
}

/// Writes the map file, whole or not at all, in RON laid out a prop a block, so a commit shows
/// which prop moved. Only the editor writes it, and only dev builds have the editor.
#[cfg(feature = "dev")]
pub(crate) fn write_map(map: &MapFile) -> std::io::Result<()> {
    // one line ending on every system, where ron would end lines with "\r\n" on Windows, and a
    // last one at the end: the file is committed
    let pretty = ron::ser::PrettyConfig::default().new_line("\n");
    let text = ron::ser::to_string_pretty(map, pretty).map_err(std::io::Error::other)?;
    crate::disk::write_atomic(&map_path(), text + "\n")
}

/// Returns the map in `text`, or `None` with an error logged when it is not one. `next_id` is
/// raised past the highest id held, so a map edited by hand never hands out a used id.
fn parse_map(text: &str) -> Option<MapFile> {
    match ron::from_str::<MapFile>(text) {
        Ok(mut map) => {
            let past_highest = map.props.iter().map(|placed| placed.id.0 + 1).max();
            map.next_id = map.next_id.max(past_highest.unwrap_or(0));
            Some(map)
        }
        Err(err) => {
            error!("the map file is broken, its props are not shown: {err}");
            None
        }
    }
}

/// Splits the map's props into those whose asset `exists` and those whose asset is gone.
fn props_to_spawn(
    map: &MapFile,
    exists: impl Fn(&str) -> bool,
) -> (Vec<&PlacedProp>, Vec<&PlacedProp>) {
    map.props.iter().partition(|placed| exists(&placed.file))
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(id: u32, file: &str) -> PlacedProp {
        PlacedProp {
            id: PropId(id),
            file: file.to_string(),
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_y(0.5),
            scale: 1.5,
            mode: PropMode::Physics,
            resting: None,
        }
    }

    #[test]
    fn a_map_written_and_read_back_is_the_same_map() {
        let map = MapFile {
            next_id: 2,
            props: vec![placed(0, "props/a.glb"), placed(1, "props/b.glb")],
        };
        let text = ron::to_string(&map).unwrap();
        assert_eq!(parse_map(&text), Some(map));
    }

    #[test]
    fn a_broken_map_reads_none() {
        assert_eq!(parse_map("not a map"), None);
    }

    #[test]
    fn next_id_is_raised_past_the_highest_id_held() {
        let map = MapFile {
            next_id: 1,
            props: vec![placed(0, "props/a.glb"), placed(4, "props/b.glb")],
        };
        let text = ron::to_string(&map).unwrap();
        assert_eq!(parse_map(&text).map(|map| map.next_id), Some(5));
    }

    #[test]
    fn a_prop_whose_file_is_gone_is_not_spawned() {
        let map = MapFile {
            next_id: 2,
            props: vec![placed(0, "props/here.glb"), placed(1, "props/gone.glb")],
        };
        let (present, missing) = props_to_spawn(&map, |file| file == "props/here.glb");
        assert_eq!(present.len(), 1);
        assert_eq!(missing.first().map(|placed| placed.id), Some(PropId(1)));
    }

    #[test]
    fn a_prop_physics_moved_stands_where_it_came_to_rest() {
        let mut prop = placed(0, "props/a.glb");
        assert_eq!(prop.transform().translation, prop.position);
        let rest = PropRest {
            position: Vec3::new(9.0, 0.0, 9.0),
            rotation: Quat::IDENTITY,
        };
        prop.resting = Some(rest);
        assert_eq!(prop.transform().translation, rest.position);
    }

    #[cfg(feature = "dev")]
    #[test]
    fn the_default_map_keeps_where_props_were_placed() {
        let mut map = MapFile {
            next_id: 1,
            props: vec![placed(0, "props/a.glb")],
        };
        if let Some(prop) = map.props.first_mut() {
            prop.resting = Some(PropRest {
                position: Vec3::splat(9.0),
                rotation: Quat::IDENTITY,
            });
        }
        let placed_map = map.as_placed();
        assert_eq!(placed_map.props.first().and_then(|prop| prop.resting), None);
        assert_eq!(placed_map.props.len(), 1);
    }
}
