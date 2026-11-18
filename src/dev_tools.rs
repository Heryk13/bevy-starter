use bevy::prelude::*;

mod fps;

// ==== Plugin =================================================================

/// The tools a game is made with, left out of release builds: `main.rs` adds this plugin
/// only with the `dev` feature. Each tool is a file of its own under `dev_tools/`.
pub(super) fn plugin(app: &mut App) {
    app.add_plugins(fps::plugin);
}
