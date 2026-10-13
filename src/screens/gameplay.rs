use bevy::prelude::*;

use super::{Screen, StartGame};
use crate::save::{Session, autosave_is_due, new_autosave_path, saves_dir, write_save};

pub(crate) mod pause;
mod playroom;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    // from startup, not only from the first game: a default session is a valid new
    // game, so count_play_time always finds one, and a shortcut past the menu while
    // building it needs no save. that shortcut goes through Screen::Loading with
    // LoadingDestination::Gameplay, never straight to gameplay: only the loading screen
    // inserts the GameplayAssets the playroom needs
    app.init_resource::<Session>();
    app.add_observer(start_game_from_gameplay.run_if(in_state(Screen::Gameplay)));
    app.add_plugins(pause::plugin);
    app.add_plugins(playroom::plugin);
    app.add_systems(
        Update,
        (
            count_play_time.run_if(in_state(Screen::Gameplay)),
            autosave_game
                .after(count_play_time)
                .run_if(in_state(pause::Pause::Off)),
        ),
    );
}

// ==== Systems ================================================================

/// Adds the frame's virtual time to the session, so the pause menu never counts as play.
fn count_play_time(mut session: ResMut<Session>, time: Res<Time>) {
    session.play_secs += time.delta_secs();
}

/// Writes the session to a new autosave file once enough play has gone unsaved. A failure is
/// only logged: the player did not ask for this save and is in the middle of playing.
fn autosave_game(mut session: ResMut<Session>) {
    if !autosave_is_due(&session) {
        return;
    }
    session.autosave_tried_secs = session.play_secs;
    let Some(dir) = saves_dir() else {
        return;
    };
    if let Err(err) = write_save(&new_autosave_path(&dir), &mut session, "") {
        error!("Failed to autosave game: {}", err);
    }
}

/// Starts the given game from gameplay: keeps it for the screens ahead, then cuts
/// straight to the loading screen, since gameplay has no veil to fade out with.
fn start_game_from_gameplay(
    start_game: On<StartGame>,
    mut commands: Commands,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    commands.insert_resource(start_game.session.clone());
    commands.insert_resource(start_game.destination);
    next_screen.set(Screen::Loading);
}
