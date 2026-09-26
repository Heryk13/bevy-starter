use bevy::prelude::*;

use super::Screen;

pub(crate) mod pause;

// ==== Config =================================================================

// distinct from the camera's clear colour, so an empty window cannot be mistaken
// for this screen having spawned
const BACKGROUND_COLOR: Color = Color::srgb(0.10, 0.20, 0.25);

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_gameplay_screen);
    app.add_plugins(pause::plugin);
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
