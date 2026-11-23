use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::first_person::FirstPersonView; // camera: first-person
use super::third_person::ThirdPersonView; // camera: third-person
use crate::input::{
    Action, ActionName, Binding, Page, action_just_pressed, register_action, register_pad,
};
use crate::save::Session;
use crate::screens::Pause;
use crate::screens::gameplay::playroom::PlayroomCamera;

// ==== Config =================================================================

// goes to the next camera: the action, and the key it starts on
const SWITCH_CAMERA: ActionName = ActionName("switch_camera");
const SWITCH_CAMERA_KEY: KeyCode = KeyCode::F5;
// Y, the face button no other action takes
const SWITCH_CAMERA_PAD_BUTTON: GamepadButton = GamepadButton::North;
const SWITCH_CAMERA_LABEL: &str = "action-switch-camera";

// the cameras F5 steps through, in order, a new game starting with the first; each on a line
// of its own, so a camera a game leaves out takes its line with it
const CAMERA_VIEWS: &[CameraView] = &[
    CameraView::FirstPerson, // camera: first-person
    CameraView::ThirdPerson, // camera: third-person
];

// ==== Plugin =================================================================

/// F5: the next camera, at once, with no effect, as Minecraft's. The camera chosen goes into
/// the save, so Continue resumes with it. Only in a game with two cameras or more: a game that
/// keeps one deletes this file and every line marked `// camera: switch`.
pub(super) fn plugin(app: &mut App) {
    register_action(
        app,
        Action {
            name: SWITCH_CAMERA,
            default: Binding::Key(SWITCH_CAMERA_KEY),
            page: Page::Game,
            label: Some(SWITCH_CAMERA_LABEL),
        },
    );
    register_pad(app, SWITCH_CAMERA, SWITCH_CAMERA_PAD_BUTTON);
    app.add_systems(
        Update,
        (
            show_saved_view,
            switch_camera
                .run_if(action_just_pressed(SWITCH_CAMERA))
                .run_if(in_state(Pause::Off)),
        )
            .chain(),
    );
}

// ==== Save ===================================================================

/// The camera a game is seen through, kept in its save.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CameraView {
    FirstPerson, // camera: first-person
    ThirdPerson, // camera: third-person
}

// written by hand: the first camera of `CAMERA_VIEWS`, so a camera left out never leaves a
// `#[default]` behind on a line of its own
impl Default for CameraView {
    fn default() -> Self {
        CAMERA_VIEWS[0]
    }
}

// ==== Systems ================================================================

/// Shows the saved camera's view as the camera spawns: each camera puts its own view on it,
/// and this keeps only the saved one's.
fn show_saved_view(
    mut commands: Commands,
    cameras: Query<Entity, Added<PlayroomCamera>>,
    session: Res<Session>,
) {
    for camera in &cameras {
        show_view(&mut commands, camera, session.game.camera);
    }
}

/// Goes to the next camera on F5, and keeps it in the game for the next save.
fn switch_camera(
    mut commands: Commands,
    camera: Single<Entity, With<PlayroomCamera>>,
    mut session: ResMut<Session>,
) {
    let next = next_view(session.game.camera);
    show_view(&mut commands, *camera, next);
    session.game.camera = next;
}

// ==== Helpers ================================================================

/// Puts `view` on the camera, and takes every other view off.
fn show_view(commands: &mut Commands, camera: Entity, view: CameraView) {
    let mut camera = commands.entity(camera);
    camera.remove::<FirstPersonView>(); // camera: first-person
    camera.remove::<ThirdPersonView>(); // camera: third-person
    match view {
        CameraView::FirstPerson => camera.insert(FirstPersonView), // camera: first-person
        CameraView::ThirdPerson => camera.insert(ThirdPersonView::default()), // camera: third-person
    };
}

/// Returns the camera after `view` in `CAMERA_VIEWS`, the first after the last.
fn next_view(view: CameraView) -> CameraView {
    let at = CAMERA_VIEWS
        .iter()
        .position(|other| *other == view)
        .unwrap_or(0);
    CAMERA_VIEWS
        .get((at + 1) % CAMERA_VIEWS.len())
        .copied()
        .unwrap_or(view)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f5_steps_through_every_camera_and_comes_back() {
        let mut view = CameraView::default();
        let first = view;
        for _ in CAMERA_VIEWS {
            view = next_view(view);
        }
        assert_eq!(view, first);
    }

    #[test]
    fn f5_goes_to_another_camera() {
        let view = CameraView::default();
        assert_ne!(next_view(view), view);
    }

    #[test]
    fn a_new_game_starts_with_the_first_camera() {
        assert_eq!(Some(&CameraView::default()), CAMERA_VIEWS.first());
    }
}
