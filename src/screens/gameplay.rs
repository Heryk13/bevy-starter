use bevy::prelude::*;

use super::Screen;
use crate::save::Session;

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
    app.add_systems(
        OnEnter(Screen::Gameplay),
        (spawn_gameplay_screen, start_session),
    );
    app.add_plugins(pause::plugin);
    app.add_systems(Update, count_play_time.run_if(in_state(Screen::Gameplay)));
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

/// Starts every game as a new one, with no play time yet.
// until `StartGame` exists (save-game step 7): then the menu and the saves
// insert the session, and a loaded game keeps its play time
fn start_session(mut commands: Commands) {
    commands.insert_resource(Session::default());
}

/// Adds the frame's virtual time to the session, so the pause menu never counts as play.
fn count_play_time(mut session: ResMut<Session>, time: Res<Time>) {
    session.play_secs += time.delta_secs();
}
