use bevy::prelude::*;

use super::Screen;
use crate::save::{
    AUTOSAVE_NAME, Session, autosave_is_due, new_autosave_path, saves_dir, write_save,
};

pub(crate) mod pause;

// ==== Config =================================================================

// distinct from the camera's clear colour, so an empty window cannot be mistaken
// for this screen having spawned
const BACKGROUND_COLOR: Color = Color::srgb(0.10, 0.20, 0.25);

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    // from startup, not only from the first game: the pause menu's observer reads it
    // on every button press in the app, and a missing resource there panics
    app.init_resource::<Session>();
    app.add_systems(OnEnter(Screen::Gameplay), spawn_gameplay_screen);
    app.add_plugins(pause::plugin);
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

/// Spawns the gameplay screen, a full-window background with nothing on it yet.
fn spawn_gameplay_screen(mut commands: Commands) {
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(BACKGROUND_COLOR),
        DespawnOnExit(Screen::Gameplay),
    ));
}

/// Adds the frame's virtual time to the session, so the pause menu never counts as play.
fn count_play_time(mut session: ResMut<Session>, time: Res<Time>) {
    session.play_secs += time.delta_secs();
}

/// Writes the session to the autosave once enough play has gone unsaved. A failure is only
/// logged: the player did not ask for this save and is in the middle of playing.
fn autosave_game(mut session: ResMut<Session>) {
    if !autosave_is_due(&session) {
        return;
    }
    session.autosave_tried_secs = session.play_secs;
    let Some(dir) = saves_dir() else {
        return;
    };
    if let Err(err) = write_save(&new_autosave_path(&dir), &mut session, AUTOSAVE_NAME) {
        error!("Failed to autosave game: {}", err);
    }
}
