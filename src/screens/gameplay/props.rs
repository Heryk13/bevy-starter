use bevy::prelude::*;

use super::map::PropFile;
use crate::screens::Screen;

// ==== Plugin =================================================================

/// What a 2D prop is: its image, drawn at its own size. It has no body, since the 2D player
/// has no physics: a prop is scenery, in front of the player or behind it by its depth.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(show_prop.run_if(in_state(Screen::Gameplay)));
}

// ==== Systems ================================================================

/// Puts the image on a prop as it spawns.
fn show_prop(
    add: On<Add<PropFile>>,
    mut commands: Commands,
    files: Query<&PropFile>,
    asset_server: Res<AssetServer>,
) {
    if let Ok(file) = files.get(add.entity) {
        commands
            .entity(add.entity)
            .insert(Sprite::from_image(asset_server.load(file.0.clone())));
    }
}
