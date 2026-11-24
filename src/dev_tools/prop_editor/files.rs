use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::super::file_explorer::{new_folder, trash_asset};
use super::PropKind;
use super::edits::{Held, MapEdit};
use super::library::{
    PROPS_DIR_NAME, PropPreferences, inside_folder, is_prop_file, parent_folder, props_dir,
};
use crate::multiplayer::is_joined; // multiplayer
use crate::save::Session;
use crate::screens::Screen;
use crate::screens::gameplay::map::{Prop, PropFile, prop_bundle, read_map, write_map};

// ==== Plugin =================================================================

/// The props folder changed from the editor: a folder made, a file or a folder renamed, one
/// deleted. Every change goes through one `PropFileEdit`, the way the map's go through
/// `MapEdit`: `apply_prop_file_edit` settles it into the `PropFileChange` it makes, which
/// `apply_prop_file_change` makes on the disk and in the game; in a game joined, the edit goes
/// to the host, which sends its change back. A rename carries every asset path that starts
/// with the old name to the new one: the game's props, the default map's, the favourites and
/// recents. A delete goes to the file explorer's Trash, and its props go with it.
pub(super) fn plugin(app: &mut App) {
    let apply_edits = apply_prop_file_edit.run_if(in_state(Screen::Gameplay));
    // a joined game sends its file edits to the host, which sends their changes back // multiplayer
    let apply_edits = apply_edits.run_if(not(is_joined)); // multiplayer
    app.add_observer(apply_edits);
    app.add_observer(apply_prop_file_change.run_if(in_state(Screen::Gameplay)));
}

// ==== Events =================================================================

/// What a player asks of the props folder, every path inside it (`trees/pine.glb`): a folder
/// made in `parent`, a file or a folder renamed, in the folder it is in, one deleted.
#[derive(Event, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub(crate) enum PropFileEdit {
    NewFolder { parent: String },
    Rename { from: String, to: String },
    Delete { path: String },
}

/// What a file edit made, its names settled: only the host names a new folder, as only it
/// hands out prop ids. Every game makes it on its own disk.
#[derive(Event, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub(crate) enum PropFileChange {
    FolderMade { path: String },
    Renamed { from: String, to: String },
    Deleted { path: String },
}

// ==== Systems ================================================================

/// Settles an edit into the change it makes, refusing a path that leaves the props folder, a
/// rename to a name taken or out of its folder, and a prop file renamed to an extension its
/// genre does not take. A new folder is made here, so its name is the one free on this disk;
/// a delete asks the map to delete every prop placed from it first, and a rename carries the
/// default map's props along, which only this game may write.
fn apply_prop_file_edit(
    edit: On<PropFileEdit>,
    mut commands: Commands,
    session: Res<Session>,
    kind: Option<Res<PropKind>>,
) {
    let change = match &*edit {
        PropFileEdit::NewFolder { parent } => {
            if !parent.is_empty() && !is_safe_path(parent) {
                warn!("refused a folder made in {parent:?}");
                return;
            }
            match new_folder(&props_dir().join(parent)) {
                Ok(name) => PropFileChange::FolderMade {
                    path: inside_folder(parent, &name),
                },
                Err(err) => {
                    error!("could not make a folder in props/{parent}: {err}");
                    return;
                }
            }
        }
        PropFileEdit::Rename { from, to } => {
            let extensions = kind.as_deref().map_or(&[][..], |kind| kind.extensions);
            if !is_rename_allowed(from, to, extensions) {
                warn!("refused renaming {from:?} to {to:?}");
                return;
            }
            rename_in_default_map(from, to);
            PropFileChange::Renamed {
                from: from.clone(),
                to: to.clone(),
            }
        }
        PropFileEdit::Delete { path } => {
            if !is_safe_path(path) {
                warn!("refused deleting {path:?}");
                return;
            }
            let placed = session.game.map.iter().flat_map(|map| &map.props);
            for prop in placed.filter(|prop| is_under(&prop.file, &asset_path(path))) {
                commands.trigger(MapEdit::Delete { id: prop.id });
            }
            PropFileChange::Deleted { path: path.clone() }
        }
    };
    commands.trigger(change);
}

/// Makes a change on this game's disk, and in the game: a renamed prop file's props spawn
/// again from their new path, the favourites and recents follow it, and a new prop in the hand
/// of a file renamed or deleted is dropped.
fn apply_prop_file_change(
    change: On<PropFileChange>,
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut preferences: ResMut<PropPreferences>,
    props: Query<(Entity, &Prop)>,
    in_hand: Query<(Entity, &PropFile), (With<Held>, Without<Prop>)>,
) {
    let gone = match &*change {
        PropFileChange::FolderMade { path } => {
            if let Err(err) = std::fs::create_dir_all(props_dir().join(path)) {
                error!("could not make the folder props/{path}: {err}");
            }
            return;
        }
        PropFileChange::Renamed { from, to } => {
            // a joined game may lack it, a folder with nothing in it never sent: the paths
            // still change, so the props follow once the files come
            let (old, new) = (props_dir().join(from), props_dir().join(to));
            if old.exists()
                && let Err(err) = std::fs::rename(&old, &new)
            {
                error!("could not rename props/{from} to props/{to}: {err}");
            }
            let (from, to) = (asset_path(from), asset_path(to));
            if let Some(map) = session.game.map.as_mut() {
                for placed in &mut map.props {
                    let Some(renamed) = renamed(&placed.file, &from, &to) else {
                        continue;
                    };
                    placed.file = renamed;
                    for (entity, prop) in &props {
                        if prop.0 == placed.id {
                            commands.entity(entity).despawn();
                            commands.spawn(prop_bundle(placed));
                        }
                    }
                }
            }
            let preferences = &mut *preferences;
            for file in preferences
                .favourites
                .iter_mut()
                .chain(preferences.recent.iter_mut())
            {
                if let Some(renamed) = renamed(file, &from, &to) {
                    *file = renamed;
                }
            }
            from
        }
        PropFileChange::Deleted { path } => {
            let asset = asset_path(path);
            if props_dir().join(path).exists()
                && let Err(err) = trash_asset(Path::new(&asset))
            {
                error!("could not delete {asset}: {err}");
            }
            asset
        }
    };
    for (entity, file) in &in_hand {
        if is_under(&file.0, &gone) {
            commands.entity(entity).despawn();
        }
    }
}

// ==== Helpers ================================================================

/// Writes the default map again with every prop placed from `from` placed from `to`, both paths
/// inside the props folder, when it has any: a new game would miss them otherwise.
fn rename_in_default_map(from: &str, to: &str) {
    let Some(mut map) = read_map() else {
        return;
    };
    let (from, to) = (asset_path(from), asset_path(to));
    let mut changed = false;
    for placed in &mut map.props {
        if let Some(renamed) = renamed(&placed.file, &from, &to) {
            placed.file = renamed;
            changed = true;
        }
    }
    if changed && let Err(err) = write_map(&map) {
        error!("could not write the default map after renaming {from}: {err}");
    }
}

/// Returns the asset path of `path`, inside the props folder: `props/trees/pine.glb`.
fn asset_path(path: &str) -> String {
    format!("{PROPS_DIR_NAME}/{path}")
}

/// Returns what follows `path` in the asset path `file` when `file` is `path` (nothing), or
/// inside the folder `path` (`/pine.glb`), or `None` when it is neither.
fn rest_under<'a>(file: &'a str, path: &str) -> Option<&'a str> {
    file.strip_prefix(path)
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// Tells whether the asset path `file` is `path`, or inside the folder `path`.
fn is_under(file: &str, path: &str) -> bool {
    rest_under(file, path).is_some()
}

/// Returns `file` with `from` at its start changed to `to`, when it is `from` or inside it:
/// what a rename makes of an asset path, a file's or one in a renamed folder.
fn renamed(file: &str, from: &str, to: &str) -> Option<String> {
    rest_under(file, from).map(|rest| format!("{to}{rest}"))
}

/// Tells whether `path` is a path inside the props folder and nothing else: plain names
/// between its `/`, none of them empty, `.`, `..`, hidden, nor holding a drive, a `\` or a
/// control character. A path a joined game sends is a door into the PC; this keeps it inside.
pub(crate) fn is_safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.split('/').all(|part| {
            !part.is_empty()
                && !part.starts_with('.')
                && !part
                    .chars()
                    .any(|character| character.is_control() || matches!(character, '\\' | ':'))
        })
}

/// Tells whether `from` can be renamed to `to`: both safe, in the same folder, `from` there and
/// `to` not, unless only the case of its letters changes; a prop file keeps an extension its
/// genre takes.
fn is_rename_allowed(from: &str, to: &str, extensions: &[&str]) -> bool {
    if !is_safe_path(from) || !is_safe_path(to) || parent_folder(from) != parent_folder(to) {
        return false;
    }
    let (old, new) = (props_dir().join(from), props_dir().join(to));
    let case_only = from.eq_ignore_ascii_case(to);
    let keeps_kind = old.is_dir() || is_prop_file(to, extensions);
    old.exists() && (case_only || !new.exists()) && keeps_kind
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rename_carries_the_file_and_everything_in_a_folder() {
        assert_eq!(
            renamed("props/pine.glb", "props/pine.glb", "props/fir.glb"),
            Some("props/fir.glb".to_string())
        );
        assert_eq!(
            renamed("props/trees/pine.glb", "props/trees", "props/woods"),
            Some("props/woods/pine.glb".to_string())
        );
    }

    #[test]
    fn a_rename_leaves_a_name_that_only_starts_alike() {
        assert_eq!(
            renamed("props/trees2/pine.glb", "props/trees", "props/woods"),
            None
        );
        assert_eq!(
            renamed("props/pine.glbx", "props/pine.glb", "props/fir.glb"),
            None
        );
    }

    #[test]
    fn only_a_path_inside_the_props_folder_is_safe() {
        assert!(is_safe_path("pine.glb"));
        assert!(is_safe_path("trees/pines/pine.glb"));
        assert!(!is_safe_path(""));
        assert!(!is_safe_path("../pine.glb"));
        assert!(!is_safe_path("trees/../../pine.glb"));
        assert!(!is_safe_path("/pine.glb"));
        assert!(!is_safe_path("trees//pine.glb"));
        assert!(!is_safe_path("C:/pine.glb"));
        assert!(!is_safe_path("trees\\pine.glb"));
        assert!(!is_safe_path(".hidden/pine.glb"));
    }

    #[test]
    fn a_rename_out_of_its_folder_is_refused() {
        assert!(!is_rename_allowed("trees/pine.glb", "pine.glb", &["glb"]));
        assert!(!is_rename_allowed("pine.glb", "../pine.glb", &["glb"]));
    }
}
