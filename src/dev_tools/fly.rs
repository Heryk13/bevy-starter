use avian3d::prelude::{ColliderDisabled, GravityScale, LinearVelocity};
use bevy::prelude::*;

use super::register_inner_key;
use super::speed::SpeedMultiplier;
use super::status::{StatusRow, spawn_status};
use crate::input::{
    Action, ActionName, Binding, Bindings, Controls, Page, action_just_pressed, register_action,
};
use crate::screens::Pause;
use crate::screens::gameplay::player::{Player, walk_input, walk_player};
use crate::screens::gameplay::playroom::PlayroomCamera;

// ==== Config =================================================================

// ---- actions: fly, and the keys that rise and sink while flying -------------
const FLY: ActionName = ActionName("fly");
const FLY_KEY: KeyCode = KeyCode::F7;
// read by photo mode's free camera too, which moves the same way
pub(super) const RISE: ActionName = ActionName("rise");
const RISE_KEY: KeyCode = KeyCode::Space;
pub(super) const SINK: ActionName = ActionName("sink");
const SINK_KEY: KeyCode = KeyCode::ShiftLeft;

// ---- words: English, as every dev tool's, never read in a release build -----
const FLY_LABEL: &str = "Fly";
const RISE_LABEL: &str = "Rise";
const SINK_LABEL: &str = "Sink";
const FLYING_WORDS: &str = "Flying";

// ---- speed ------------------------------------------------------------------
// fast enough to cross the map and climb above it in a few seconds; speed's multiplier
// (F8) multiplies it, as it does walking's
const FLY_SPEED_M_PER_S: f32 = 12.0;

// ==== Plugin =================================================================

/// Fly on F7: the player's body leaves gravity and passes through everything, moving where
/// the camera looks, to judge from high and far how far the map can be seen. F7 again gives
/// gravity back, and the body falls.
pub(super) fn plugin(app: &mut App) {
    for (name, key, label) in [
        (FLY, FLY_KEY, FLY_LABEL),
        (RISE, RISE_KEY, RISE_LABEL),
        (SINK, SINK_KEY, SINK_LABEL),
    ] {
        register_action(
            app,
            Action {
                name,
                default: Binding::Key(key),
                page: Page::Dev,
                label: Some(label),
            },
        );
    }
    // they only do something while flying or in photo mode, neither of which the keys panel
    // shows over
    register_inner_key(app, RISE);
    register_inner_key(app, SINK);
    app.add_systems(
        Update,
        (
            toggle_fly.run_if(action_just_pressed(FLY)),
            // after walking, which set the velocity across the ground: flying sets all of it
            fly_player.after(walk_player),
        )
            .run_if(in_state(Pause::Off)),
    );
    app.add_observer(show_flying_status);
    app.add_observer(hide_flying_status);
}

// ==== Components =============================================================

/// Marks the player's body while it flies, beside avian's `GravityScale` and
/// `ColliderDisabled`, which come and go with it.
#[derive(Component)]
struct Flying;

/// Marks the status that says the body flies.
#[derive(Component)]
struct FlyingStatus;

// ==== Systems ================================================================

/// Starts flying, the body out of gravity and through every collider, or stops.
fn toggle_fly(mut commands: Commands, body: Single<(Entity, Has<Flying>), With<Player>>) {
    let (body, flying) = *body;
    if flying {
        commands
            .entity(body)
            .remove::<(Flying, GravityScale, ColliderDisabled)>();
    } else {
        commands
            .entity(body)
            .insert((Flying, GravityScale(0.0), ColliderDisabled));
    }
}

/// Says the body flies in the status row (`status.rs`), as it starts flying.
fn show_flying_status(
    _: On<Add<Flying>>,
    mut commands: Commands,
    bindings: Res<Bindings>,
    row: Single<Entity, With<StatusRow>>,
) {
    spawn_status(
        &mut commands,
        *row,
        FlyingStatus,
        FLY,
        &bindings,
        FLYING_WORDS,
    );
}

/// Takes the status away as the body stops flying, or goes with gameplay.
fn hide_flying_status(
    _: On<Remove<Flying>>,
    mut commands: Commands,
    statuses: Query<Entity, With<FlyingStatus>>,
) {
    for status in &statuses {
        commands.entity(status).despawn();
    }
}

/// Moves the flying body where the camera looks, by the walk actions, Rise and Sink, as many
/// times faster as speed's multiplier says; letting go stops it in the air.
fn fly_player(
    controls: Controls,
    camera: Single<&GlobalTransform, With<PlayroomCamera>>,
    mut body: Single<&mut LinearVelocity, (With<Player>, With<Flying>)>,
    multiplier: Res<SpeedMultiplier>,
) {
    body.0 = fly_velocity(
        walk_input(&controls),
        camera.rotation(),
        controls.pressed(RISE),
        controls.pressed(SINK),
        FLY_SPEED_M_PER_S * multiplier.0,
    );
}

// ==== Helpers ================================================================

/// Returns the velocity of something flying at `speed` that looks along `look`: `input` is
/// right and forward, where it looks, up and down included, and `rise` and `sink` move it
/// straight up and down. Made one long first, so a diagonal is no faster than straight ahead.
pub(super) fn fly_velocity(input: Vec2, look: Quat, rise: bool, sink: bool, speed: f32) -> Vec3 {
    let up = f32::from(u8::from(rise)) - f32::from(u8::from(sink));
    let direction = look * Vec3::new(input.x, 0.0, -input.y) + Vec3::Y * up;
    direction.normalize_or_zero() * speed
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-4;
    const SPEED: f32 = FLY_SPEED_M_PER_S;

    #[test]
    fn forward_while_looking_up_climbs() {
        let look_up = Quat::from_rotation_x(45_f32.to_radians());
        let velocity = fly_velocity(Vec2::Y, look_up, false, false, SPEED);
        assert!(velocity.y > 0.0);
        assert!((velocity.length() - SPEED).abs() < TOLERANCE);
    }

    #[test]
    fn rise_alone_goes_straight_up() {
        let velocity = fly_velocity(Vec2::ZERO, Quat::IDENTITY, true, false, SPEED);
        assert!((velocity - Vec3::Y * SPEED).length() < TOLERANCE);
    }

    #[test]
    fn nothing_held_stands_still_in_the_air() {
        let velocity = fly_velocity(Vec2::ZERO, Quat::IDENTITY, true, true, SPEED);
        assert_eq!(velocity, Vec3::ZERO);
    }
}
