use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use super::{Pose, REMOTE_EASE_PER_S, REMOTE_SILENT_SECS, RemotePlayer};
use crate::screens::Screen;

// ==== Plugin =================================================================

/// The others' bodies: one `RemotePlayer` per other player, spawned with its first pose, eased
/// toward each pose heard after, and gone when it stops being heard or gameplay ends. Hosting
/// and joining both hand their poses here.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<RemoteIndex>();
    app.add_systems(
        Update,
        (ease_remote_players, forget_silent_players).run_if(in_state(Screen::Gameplay)),
    );
    app.add_systems(OnExit(Screen::Gameplay), forget_every_player);
}

// ==== Components =============================================================

/// Where another body is headed, its last pose, and when it was heard.
#[derive(Component)]
pub(super) struct Heading {
    target: Transform,
    heard_secs: f32,
}

// ==== Resources ==============================================================

/// Each other player's body, by its id: a body spawned this frame is in it before it is in any
/// query, so a second pose in the same frame never spawns a second body.
#[derive(Resource, Default)]
pub(super) struct RemoteIndex(HashMap<u64, Entity>);

// ==== Systems ================================================================

/// Eases every other body toward its last pose, at the same pace whatever the frame rate.
fn ease_remote_players(time: Res<Time<Real>>, mut bodies: Query<(&mut Transform, &Heading)>) {
    let blend = ease_blend(time.delta_secs());
    for (mut transform, heading) in &mut bodies {
        transform.translation = transform
            .translation
            .lerp(heading.target.translation, blend);
        transform.rotation = transform.rotation.slerp(heading.target.rotation, blend);
    }
}

/// Takes away a body not heard for `REMOTE_SILENT_SECS`: its player left, or lost the connection.
fn forget_silent_players(
    mut commands: Commands,
    time: Res<Time<Real>>,
    bodies: Query<(Entity, &RemotePlayer, &Heading)>,
    mut index: ResMut<RemoteIndex>,
) {
    for (entity, player, heading) in &bodies {
        if time.elapsed_secs() - heading.heard_secs > REMOTE_SILENT_SECS {
            commands.entity(entity).despawn();
            index.0.remove(&player.id);
        }
    }
}

/// Forgets every other body as gameplay ends: they despawn with it.
fn forget_every_player(mut index: ResMut<RemoteIndex>) {
    index.0.clear();
}

// ==== Helpers ================================================================

/// Moves the body of `pose`'s player toward it, spawning the body with its first pose. Only in
/// gameplay, where the bodies are: a pose heard elsewhere is dropped.
pub(super) fn hear_pose(
    commands: &mut Commands,
    index: &mut RemoteIndex,
    headings: &mut Query<&mut Heading>,
    pose: &Pose,
    now_secs: f32,
) {
    let target =
        Transform::from_translation(pose.position).with_rotation(Quat::from_rotation_y(pose.yaw));
    if let Some(entity) = index.0.get(&pose.owner) {
        // spawned this frame, the body is in no query yet: the next pose moves it
        if let Ok(mut heading) = headings.get_mut(*entity) {
            heading.target = target;
            heading.heard_secs = now_secs;
        }
        return;
    }
    let entity = commands
        .spawn((
            RemotePlayer { id: pose.owner },
            target,
            Heading {
                target,
                heard_secs: now_secs,
            },
            DespawnOnExit(Screen::Gameplay),
        ))
        .id();
    index.0.insert(pose.owner, entity);
}

/// Takes away the body of the player `id`, as the host hears that player leave.
pub(super) fn forget_player(commands: &mut Commands, index: &mut RemoteIndex, id: u64) {
    if let Some(entity) = index.0.remove(&id) {
        commands.entity(entity).try_despawn();
    }
}

/// Returns how much of the way to its pose a body goes in `delta_secs`: the same share of the
/// distance left every second, whatever the frame rate.
fn ease_blend(delta_secs: f32) -> f32 {
    1.0 - (-REMOTE_EASE_PER_S * delta_secs).exp()
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    #[test]
    fn a_body_eases_part_of_the_way_and_never_past_its_pose() {
        let blend = ease_blend(1.0 / 60.0);
        assert!(blend > 0.0 && blend < 1.0);
        assert!(ease_blend(10.0) <= 1.0);
    }

    #[test]
    fn two_short_frames_go_as_far_as_one_long_one() {
        let short = ease_blend(0.01);
        let two_short = 1.0 - (1.0 - short) * (1.0 - short);
        assert!((two_short - ease_blend(0.02)).abs() < TOLERANCE);
    }
}
