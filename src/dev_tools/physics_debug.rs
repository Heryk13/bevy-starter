use avian3d::prelude::{PhysicsDebugPlugin, PhysicsGizmos};
use bevy::prelude::*;

use crate::input::{Action, ActionName, Binding, Page, action_just_pressed, register_action};
use crate::screens::Pause;

// ==== Config =================================================================

// ---- action: shows and hides the outlines, and the key it starts on ---------
const PHYSICS_DEBUG: ActionName = ActionName("physics_debug");
const PHYSICS_DEBUG_KEY: KeyCode = KeyCode::F6;

// ---- words: English, as every dev tool's, never read in a release build -----
const PHYSICS_DEBUG_LABEL: &str = "Physics Debug";

// ==== Plugin =================================================================

/// The physics debug view on F6: the outline of every collider, the player's capsule
/// included, to check them against the level. Off as the game starts, and hidden in photo
/// mode, where the outlines would land in the photo.
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
    app.add_systems(Startup, hide_physics_outlines);
    app.add_systems(
        Update,
        toggle_physics_outlines
            .run_if(action_just_pressed(PHYSICS_DEBUG))
            .run_if(not(in_state(Pause::Photo))),
    );
    app.add_systems(OnEnter(Pause::Photo), hide_physics_outlines_for_photo);
    app.add_systems(OnExit(Pause::Photo), restore_physics_outlines_after_photo);
}

// ==== Resources ==============================================================

/// Whether the outlines showed as photo mode began, so leaving it shows them again only then.
#[derive(Resource)]
struct OutlinesShownBeforePhoto(bool);

// ==== Systems ================================================================

/// Hides the outlines as the game starts: a debug view left on would greet every launch.
fn hide_physics_outlines(mut gizmos: ResMut<GizmoConfigStore>) {
    show_physics_outlines(&mut gizmos, false);
}

/// Shows the outlines if they are hidden, or hides them.
fn toggle_physics_outlines(mut gizmos: ResMut<GizmoConfigStore>) {
    let shown = are_physics_outlines_shown(&mut gizmos);
    show_physics_outlines(&mut gizmos, !shown);
}

/// Hides the outlines as photo mode begins, keeping whether they showed: they are gizmos,
/// not a dev window, so hiding the dev windows leaves them.
fn hide_physics_outlines_for_photo(mut commands: Commands, mut gizmos: ResMut<GizmoConfigStore>) {
    commands.insert_resource(OutlinesShownBeforePhoto(are_physics_outlines_shown(
        &mut gizmos,
    )));
    show_physics_outlines(&mut gizmos, false);
}

/// Shows the outlines again as photo mode ends, if they showed as it began.
fn restore_physics_outlines_after_photo(
    shown_before: Option<Res<OutlinesShownBeforePhoto>>,
    mut gizmos: ResMut<GizmoConfigStore>,
) {
    if let Some(shown_before) = shown_before {
        show_physics_outlines(&mut gizmos, shown_before.0);
    }
}

// ==== Helpers ================================================================

/// Tells whether avian draws the outlines now.
fn are_physics_outlines_shown(gizmos: &mut GizmoConfigStore) -> bool {
    gizmos
        .get_config_mut::<PhysicsGizmos>()
        .is_some_and(|(config, _)| config.enabled)
}

/// Has avian draw the outlines, or not.
fn show_physics_outlines(gizmos: &mut GizmoConfigStore, shown: bool) {
    if let Some((config, _)) = gizmos.get_config_mut::<PhysicsGizmos>() {
        config.enabled = shown;
    }
}
