use avian3d::prelude::{PhysicsDebugPlugin, PhysicsGizmos};
use bevy::prelude::*;

use crate::input::{Action, ActionName, Binding, Page, action_just_pressed, register_action};
use crate::screens::Pause;

// ==== Config =================================================================

// ---- action: steps through the views, and the key it starts on --------------
const PHYSICS_DEBUG: ActionName = ActionName("physics_debug");
const PHYSICS_DEBUG_KEY: KeyCode = KeyCode::F6;

// ---- words: English, as every dev tool's, never read in a release build -----
const PHYSICS_DEBUG_LABEL: &str = "Physics Debug";

// ==== Plugin =================================================================

/// The physics debug view on F6, stepping through two views and back to none: the outline of
/// every collider, the player's capsule included, to check them against the level; then the
/// rays a camera casts to find what stands between it and the player, which the camera that
/// casts them draws itself. Off as the game starts, and hidden in photo mode, where it would
/// land in the photo.
pub(super) fn plugin(app: &mut App) {
    register_action(
        app,
        Action {
            name: PHYSICS_DEBUG,
            default: Binding::Key(PHYSICS_DEBUG_KEY),
            page: Page::Dev,
            label: Some(PHYSICS_DEBUG_LABEL),
        },
    );
    app.add_plugins(PhysicsDebugPlugin);
    app.init_resource::<PhysicsDebugView>();
    app.add_systems(
        Update,
        (
            step_physics_debug
                .run_if(action_just_pressed(PHYSICS_DEBUG))
                .run_if(not(in_state(Pause::Photo))),
            show_physics_outlines.run_if(resource_changed::<PhysicsDebugView>),
        )
            .chain(),
    );
    app.add_systems(OnEnter(Pause::Photo), hide_physics_debug_for_photo);
    app.add_systems(OnExit(Pause::Photo), restore_physics_debug_after_photo);
}

// ==== Resources ==============================================================

/// What F6 shows now. A camera reads it to draw its own rays.
#[derive(Resource, Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) enum PhysicsDebugView {
    #[default]
    Off,
    Colliders,
    CameraRays,
}

/// What F6 showed as photo mode began, so leaving it shows that again.
#[derive(Resource)]
struct ViewBeforePhoto(PhysicsDebugView);

// ==== Systems ================================================================

/// Steps to the next view on F6: the colliders, the camera's rays, then nothing.
fn step_physics_debug(mut view: ResMut<PhysicsDebugView>) {
    *view = next_view(*view);
}

/// Has avian draw the outlines while the colliders are the view, and not otherwise: off as
/// the game starts too, since a debug view left on would greet every launch.
fn show_physics_outlines(view: Res<PhysicsDebugView>, mut gizmos: ResMut<GizmoConfigStore>) {
    if let Some((config, _)) = gizmos.get_config_mut::<PhysicsGizmos>() {
        config.enabled = *view == PhysicsDebugView::Colliders;
    }
}

/// Hides the view as photo mode begins, keeping which it was: gizmos are not a dev window, so
/// hiding the dev windows leaves them.
fn hide_physics_debug_for_photo(mut commands: Commands, mut view: ResMut<PhysicsDebugView>) {
    commands.insert_resource(ViewBeforePhoto(*view));
    *view = PhysicsDebugView::Off;
}

/// Shows the view again as photo mode ends, the one that showed as it began.
fn restore_physics_debug_after_photo(
    before: Option<Res<ViewBeforePhoto>>,
    mut view: ResMut<PhysicsDebugView>,
) {
    if let Some(before) = before {
        *view = before.0;
    }
}

// ==== Helpers ================================================================

/// Returns the view after `view`, nothing after the last.
fn next_view(view: PhysicsDebugView) -> PhysicsDebugView {
    match view {
        PhysicsDebugView::Off => PhysicsDebugView::Colliders,
        PhysicsDebugView::Colliders => PhysicsDebugView::CameraRays,
        PhysicsDebugView::CameraRays => PhysicsDebugView::Off,
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f6_shows_the_colliders_then_the_rays_then_nothing() {
        let mut view = PhysicsDebugView::default();
        assert_eq!(view, PhysicsDebugView::Off);
        view = next_view(view);
        assert_eq!(view, PhysicsDebugView::Colliders);
        view = next_view(view);
        assert_eq!(view, PhysicsDebugView::CameraRays);
        view = next_view(view);
        assert_eq!(view, PhysicsDebugView::Off);
    }
}
