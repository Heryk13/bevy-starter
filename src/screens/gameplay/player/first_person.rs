use bevy::prelude::*;

use super::look::EYE_OFFSET_M;
use crate::screens::Screen;
use crate::screens::gameplay::playroom::PlayroomCamera;

// ==== Plugin =================================================================

/// Shows the playroom through the player's eyes: the camera at eye height, where `look.rs`
/// hangs it, while `FirstPersonView` is on it. A game that keeps another camera deletes this
/// file and every line marked `// camera: first-person`.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(view_in_first_person.run_if(in_state(Screen::Gameplay)));
    app.add_systems(Update, put_camera_at_eyes);
}

// ==== Components =============================================================

/// Marks the camera while it sees through the player's eyes.
#[derive(Component)]
pub(crate) struct FirstPersonView;

// ==== Systems ================================================================

/// Puts the camera in first person as it spawns. With other cameras in the game, the switch
/// (`camera_switch.rs`) then keeps only the saved one's view.
fn view_in_first_person(add: On<Add<PlayroomCamera>>, mut commands: Commands) {
    commands.entity(add.entity).insert(FirstPersonView);
}

/// Puts the camera back at the eyes as first person comes on, from wherever another camera
/// left it, the tilt kept.
fn put_camera_at_eyes(mut cameras: Query<&mut Transform, Added<FirstPersonView>>) {
    for mut transform in &mut cameras {
        transform.translation = Vec3::new(0.0, EYE_OFFSET_M, 0.0);
    }
}
