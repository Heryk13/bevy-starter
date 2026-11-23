use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use lightyear::prelude::server::ClientOf;
use lightyear::prelude::*;
use std::path::PathBuf;

use super::{Network, WorldChannel, WorldState};
use crate::disk::{game_data_dir, write_atomic};
use crate::i18n::Localized;
use crate::save::{GameData, Session};
use crate::screens::Screen;

// ==== Config =================================================================

// ---- the joiner's character: where it stood in each world it joined ---------
const CHARACTERS_FILE_NAME: &str = "characters.ron";
// often enough that a crash loses little, seldom enough to be no cost
const CHARACTER_KEEP_EVERY_SECS: f32 = 10.0;

// ---- what a joiner's pause menu leaves out: the world is the host's ---------
// the pause menu's own keys: a button whose label is one of them is hidden while joined
const HOST_ONLY_KEYS: [&str; 2] = ["pause-save", "pause-load-game"];

// ==== Plugin =================================================================

/// The world and the characters, as Valheim keeps them: the host's game is the world, saved
/// by the host as ever, with an id of its own; a player who joins gets the world's props and
/// id as it connects, and keeps only its own character, in `characters.ron` on its own PC, a
/// place for each world it joined. A joiner has no Save nor Load Game, and never autosaves.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<FreshInWorld>();
    app.add_systems(OnEnter(Screen::Gameplay), name_the_world);
    app.add_observer(send_the_world);
    app.add_systems(
        Update,
        (keep_character_now_and_then, hide_host_only_buttons)
            .run_if(resource_equals(Network::Joined))
            .run_if(in_state(Screen::Gameplay)),
    );
}

// ==== Resources ==============================================================

/// Whether this joiner is in this world for the first time: it starts on the ring around the
/// spawn point, rather than where it stood last time.
#[derive(Resource, Default)]
pub(crate) struct FreshInWorld(pub(crate) bool);

// ==== Systems ================================================================

/// Gives a world with none an id as gameplay begins, a new game's or an old save's: a joiner's
/// world is the host's, and keeps the host's id.
fn name_the_world(mut session: ResMut<Session>, network: Res<Network>) {
    if session.game.world_id == 0 && *network != Network::Joined {
        session.game.world_id = new_world_id();
    }
}

/// Sends the host's world to a player the moment it connects: its id, and its props.
fn send_the_world(
    connected: On<Add<Connected>>,
    mut joiners: Query<&mut MessageSender<WorldState>, With<ClientOf>>,
    session: Res<Session>,
) {
    let Ok(mut sender) = joiners.get_mut(connected.entity) else {
        return;
    };
    sender.send::<WorldChannel>(WorldState {
        world_id: session.game.world_id,
        map: session.game.map.clone().unwrap_or_default(),
    });
}

/// Keeps the joiner's character every `CHARACTER_KEEP_EVERY_SECS`, its play counted as saved,
/// so quitting asks nothing.
fn keep_character_now_and_then(
    time: Res<Time<Real>>,
    mut since_secs: Local<f32>,
    mut session: ResMut<Session>,
) {
    *since_secs += time.delta_secs();
    if *since_secs < CHARACTER_KEEP_EVERY_SECS {
        return;
    }
    *since_secs = 0.0;
    keep_character(&mut session);
}

/// Hides Save and Load Game from a joiner's pause menu: the world is the host's to save.
// by the buttons' own label keys, so the pause menu names nothing of multiplayer
fn hide_host_only_buttons(labels: Query<(&Localized, &ChildOf)>, mut nodes: Query<&mut Node>) {
    for (label, parent) in &labels {
        if HOST_ONLY_KEYS.contains(&label.0)
            && let Ok(mut node) = nodes.get_mut(parent.parent())
            && node.display != Display::None
        {
            node.display = Display::None;
        }
    }
}

// ==== Helpers ================================================================

/// Returns the session a joiner starts with in the host's world: its character as it last
/// stood in this world, or a new one, with the host's props and the world's id; and whether it
/// is new to this world.
pub(super) fn joiner_session(state: WorldState) -> (Session, bool) {
    let kept = read_characters().remove(&state.world_id);
    let fresh = kept.is_none();
    let mut game = kept.unwrap_or_default();
    game.map = Some(state.map);
    game.world_id = state.world_id;
    (Session { game, ..default() }, fresh)
}

/// Keeps the joiner's character in `characters.ron`, under its world, without the world's
/// props, which are the host's: what this player saves of a game it joined.
pub(super) fn keep_character(session: &mut Session) {
    let Some(path) = characters_path() else {
        return;
    };
    let mut characters = read_characters();
    let mut character = session.game.clone();
    character.map = None;
    characters.insert(session.game.world_id, character);
    let written = ron::to_string(&characters)
        .map_err(std::io::Error::other)
        .and_then(|text| write_atomic(&path, text));
    match written {
        Ok(()) => session.saved_play_secs = session.play_secs,
        Err(err) => error!("could not keep this character: {err}"),
    }
}

/// Returns every character kept, by world, or none when the file is missing or broken.
fn read_characters() -> HashMap<u64, GameData> {
    characters_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| ron::from_str(&text).ok())
        .unwrap_or_default()
}

/// Returns the characters file's path, or `None` on a machine with no data folder.
fn characters_path() -> Option<PathBuf> {
    game_data_dir().map(|dir| dir.join(CHARACTERS_FILE_NAME))
}

/// Returns a world id no other world is likely to have: the clock, mixed with this process's
/// own, never 0, which means none yet.
fn new_world_id() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |time| u64::try_from(time.as_nanos()).unwrap_or(u64::MAX));
    (nanos ^ (u64::from(std::process::id()) << 32)).max(1)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_world_id_is_never_none() {
        assert_ne!(new_world_id(), 0);
    }
}
