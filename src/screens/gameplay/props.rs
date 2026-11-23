use avian3d::prelude::{
    ColliderConstructor, ColliderConstructorHierarchy, ColliderConstructorHierarchyReady,
    RigidBody, RigidBodyDisabled,
};
use bevy::prelude::*;

use super::map::{Prop, PropFile, PropMode};
use crate::screens::Screen;

// ==== Plugin =================================================================

/// What a 3D prop is: its model, and the body of its mode. Fixed stands where it was put, as
/// the level does; Physics is pushed around, so it waits for the level's colliders, as the
/// player does, or it would fall through the floor on the first physics step; No Collision
/// has no body at all. A prop in the editor's hand, with no `Prop` yet, is the model alone.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(show_prop.run_if(in_state(Screen::Gameplay)));
    app.add_observer(wake_physics_props.run_if(in_state(Screen::Gameplay)));
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

/// Forgets the level's colliders as gameplay ends: the next game's level is a new one.
fn forget_level_colliders(mut commands: Commands) {
    commands.remove_resource::<LevelColliders>();
}
