use avian3d::prelude::{
    ColliderConstructor, ColliderConstructorHierarchy, ColliderConstructorHierarchyReady,
    RigidBody, RigidBodyDisabled,
};
use bevy::prelude::*;

use super::map::{PlacedProp, Prop, PropFile, PropMode, PropRest};
use crate::save::Session;
use crate::screens::{Pause, Screen};

// ==== Config =================================================================

// how far a physics prop moves, or turns, from where it was placed before its resting place
// is kept: less is the settling of a prop put down, not a push
const REST_MOVE_M: f32 = 0.01;
const REST_TURN_RAD: f32 = 0.01;

// ==== Plugin =================================================================

/// What a 3D prop is: its model, and the body of its mode. Fixed stands where it was put, as
/// the level does; Physics is pushed around, so it waits for the level's colliders, as the
/// player does, or it would fall through the floor on the first physics step, and where it
/// comes to rest goes into the game in progress, so a save keeps it there; No Collision has
/// no body at all. A prop in the editor's hand, with no `Prop` yet, is the model alone.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(show_prop.run_if(in_state(Screen::Gameplay)));
    app.add_observer(wake_physics_props.run_if(in_state(Screen::Gameplay)));
    // only in Pause::Off, as the player's own state: a save loaded from the pause menu
    // replaces the session while the props of the game before are still here
    app.add_systems(Update, record_prop_rests.run_if(in_state(Pause::Off)));
    app.add_systems(OnExit(Screen::Gameplay), forget_level_colliders);
}

// ==== Components =============================================================

/// Marks a physics prop waiting, disabled, for the level's colliders.
#[derive(Component)]
struct WaitsForLevel;

// ==== Resources ==============================================================

/// Says the level's colliders are in, so a physics prop placed from now on falls at once.
#[derive(Resource)]
struct LevelColliders;

// ==== Systems ================================================================

/// Puts the model on a prop as it spawns and, on a placed one, the body of its mode.
fn show_prop(
    add: On<Add<PropFile>>,
    mut commands: Commands,
    props: Query<(&PropFile, &PropMode, Has<Prop>)>,
    asset_server: Res<AssetServer>,
    level_colliders: Option<Res<LevelColliders>>,
) {
    let Ok((file, mode, placed)) = props.get(add.entity) else {
        return;
    };
    let model = asset_server.load(GltfAssetLabel::Scene(0).from_asset(file.0.clone()));
    let mut prop = commands.entity(add.entity);
    prop.insert(WorldAssetRoot(model));
    if !placed {
        return;
    }
    match mode {
        PropMode::Fixed => {
            prop.insert((
                RigidBody::Static,
                ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
            ));
        }
        // a hull rather than a trimesh: a trimesh is for what does not move
        PropMode::Physics => {
            prop.insert((
                RigidBody::Dynamic,
                ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
            ));
            if level_colliders.is_none() {
                prop.insert((RigidBodyDisabled, WaitsForLevel));
            }
        }
        PropMode::NoCollision => {}
    }
}

/// Lets the physics props fall once the level's colliders are in: the first hierarchy ready
/// that is not a prop's is the level's.
fn wake_physics_props(
    ready: On<ColliderConstructorHierarchyReady>,
    mut commands: Commands,
    props: Query<(), With<Prop>>,
    waiting: Query<Entity, With<WaitsForLevel>>,
) {
    if props.contains(ready.entity) {
        return;
    }
    commands.insert_resource(LevelColliders);
    for prop in &waiting {
        commands
            .entity(prop)
            .remove::<(RigidBodyDisabled, WaitsForLevel)>();
    }
}

/// Keeps where each physics prop came to rest in the game in progress, a body held still in
/// the editor's hand aside: it is kinematic, not dynamic, until it is put down.
// ponytail: each prop looked up in the map by id, every frame; a map of ids to entries if a
// level ever holds thousands of physics props
fn record_prop_rests(mut session: ResMut<Session>, props: Query<(&Prop, &RigidBody, &Transform)>) {
    let Some(map) = session.game.map.as_mut() else {
        return;
    };
    for (prop, body, transform) in &props {
        if *body != RigidBody::Dynamic {
            continue;
        }
        let Some(placed) = map.props.iter_mut().find(|placed| placed.id == prop.0) else {
            continue;
        };
        let rest = rest_of(placed, transform);
        if placed.resting != rest {
            placed.resting = rest;
        }
    }
}

/// Forgets the level's colliders as gameplay ends: the next game's level is a new one.
fn forget_level_colliders(mut commands: Commands) {
    commands.remove_resource::<LevelColliders>();
}

// ==== Helpers ================================================================

/// Returns where a prop placed as `placed` rests, standing at `live`: nowhere of its own while
/// it is still where it was placed.
fn rest_of(placed: &PlacedProp, live: &Transform) -> Option<PropRest> {
    let moved = live.translation.distance(placed.position) > REST_MOVE_M;
    let turned = live.rotation.angle_between(placed.rotation) > REST_TURN_RAD;
    (moved || turned).then_some(PropRest {
        position: live.translation,
        rotation: live.rotation,
    })
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::gameplay::map::PropId;

    fn placed_at(position: Vec3) -> PlacedProp {
        PlacedProp {
            id: PropId(0),
            file: "props/box.glb".to_string(),
            position,
            rotation: Quat::IDENTITY,
            scale: 1.0,
            mode: PropMode::Physics,
            resting: None,
        }
    }

    #[test]
    fn a_prop_where_it_was_placed_has_no_resting_place() {
        let placed = placed_at(Vec3::ZERO);
        let live = Transform::from_translation(Vec3::splat(REST_MOVE_M / 2.0));
        assert_eq!(rest_of(&placed, &live), None);
    }

    #[test]
    fn a_pushed_prop_rests_where_it_stopped() {
        let placed = placed_at(Vec3::ZERO);
        let live = Transform::from_xyz(3.0, 0.0, 0.0);
        assert_eq!(
            rest_of(&placed, &live).map(|rest| rest.position),
            Some(Vec3::new(3.0, 0.0, 0.0))
        );
    }

    #[test]
    fn a_prop_tipped_over_in_place_rests_tipped() {
        let placed = placed_at(Vec3::ZERO);
        let live = Transform::from_rotation(Quat::from_rotation_z(REST_TURN_RAD * 10.0));
        assert!(rest_of(&placed, &live).is_some());
    }
}
