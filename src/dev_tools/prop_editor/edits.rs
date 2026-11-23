use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use super::super::register_inner_key;
use super::library::PropPreferences;
use super::window::PropCell;
use super::{PropEditor, PropKind};
use crate::input::{Action, ActionName, Binding, Controls, Page, register_action};
use crate::screens::Screen;
use crate::screens::gameplay::map::{
    MapFile, PlacedProp, Prop, PropFile, PropId, PropMode, prop_bundle, read_map, write_map,
};

// ==== Config =================================================================

// ---- actions: inner keys, shown on the window rather than the keys panel ----
pub(super) const TURN_PROP: ActionName = ActionName("turn_prop");
const TURN_PROP_KEY: KeyCode = KeyCode::KeyR;
const TURN_PROP_LABEL: &str = "Turn Prop";
pub(super) const DELETE_PROP: ActionName = ActionName("delete_prop");
const DELETE_PROP_KEY: KeyCode = KeyCode::Delete;
const DELETE_PROP_LABEL: &str = "Delete Prop";

// ---- turning and scaling the held prop: fixed steps -------------------------
const PROP_TURN_STEP_DEG: f32 = 15.0;
// a notch of the wheel grows the prop by this, or shrinks it by as much
const PROP_SCALE_STEP: f32 = 1.1;
const PROP_SCALE_MIN: f32 = 0.1;
const PROP_SCALE_MAX: f32 = 10.0;
// a wheel or a touchpad that reports pixels: this many make a notch, as the 2D free camera
// counts them
const SCROLL_PX_PER_NOTCH: f32 = 100.0;

// ---- derived ----------------------------------------------------------------
const PROP_TURN_STEP_RAD: f32 = PROP_TURN_STEP_DEG.to_radians();

// ==== Plugin =================================================================

/// What the editor does to props: every change goes through one `MapEdit`, applied to the
/// world and written to the map file in one place, so multiplayer can later send the same
/// edit to the host first. A click on a thumbnail takes a new prop into the hand, a click on
/// a placed one picks it up, and another click puts it down; R turns the held prop, the wheel
/// scales it, Delete removes it, Esc puts it back. Where the held prop goes, and which prop is
/// under the mouse, are the genre's.
pub(super) fn plugin(app: &mut App) {
    for (name, key, label) in [
        (TURN_PROP, TURN_PROP_KEY, TURN_PROP_LABEL),
        (DELETE_PROP, DELETE_PROP_KEY, DELETE_PROP_LABEL),
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
        register_inner_key(app, name);
    }
    app.init_resource::<HoveredProp>();
    app.add_observer(apply_map_edit.run_if(in_state(Screen::Gameplay)));
    app.add_observer(take_new_prop.run_if(in_state(PropEditor::On)));
    app.add_systems(
        Update,
        (
            put_down_or_pick_up,
            turn_held_prop,
            scale_held_prop,
            delete_prop,
        )
            .chain()
            .run_if(in_state(PropEditor::On)),
    );
    // a prop still held as the editor closes, by Esc's last press or by leaving
    // `Pause::Off`, goes back where it was
    app.add_systems(OnExit(PropEditor::On), put_back_held_prop);
}

// ==== Events =================================================================

/// A change to the map, the one way the editor changes it: applied to the world and written
/// to the map file by `apply_map_edit`. It names props by `PropId`, never by `Entity`, so the
/// same edit means the same on another machine.
#[derive(Event, Clone, PartialEq, Debug)]
pub(crate) enum MapEdit {
    Place {
        file: String,
        transform: Transform,
    },
    // where a prop stands and how it collides, both: putting a prop down keeps its mode, and
    // choosing a mode keeps where it stands, so one edit serves both, in every genre
    Update {
        id: PropId,
        transform: Transform,
        mode: PropMode,
    },
    Delete {
        id: PropId,
    },
}

// ==== Components =============================================================

/// Marks the prop stuck to the mouse, with where it was picked up from, or `None` for a new
/// one taken from a thumbnail, which has no `Prop` until it is put down.
#[derive(Component)]
pub(crate) struct Held {
    pub(crate) from: Option<Transform>,
}

/// Marks the props window's sheet, whose `Hovered` tells a click on the window from a click
/// on the world.
#[derive(Component)]
pub(super) struct PropsSheet;

// ==== Resources ==============================================================

/// The placed prop under the mouse, the held one aside, which the genre finds each frame.
#[derive(Resource, Default)]
pub(crate) struct HoveredProp(pub(crate) Option<Entity>);

// ==== Systems ================================================================

/// Applies an edit to the map file, then to the world: a new prop spawned, a prop moved or
/// given a mode, or removed. A map that cannot be read is left alone, never written over.
fn apply_map_edit(
    edit: On<MapEdit>,
    mut commands: Commands,
    mut props: Query<(Entity, &Prop, &mut Transform, &PropMode)>,
) {
    let Some(map) = read_map() else {
        error!("the map file could not be read, so this edit is not kept");
        return;
    };
    let map = applied(map, &edit);
    if let Err(err) = write_map(&map) {
        error!("could not write the map file: {err}");
        return;
    }
    let entity_of = |id: PropId| {
        props
            .iter()
            .find(|(_, prop, _, _)| prop.0 == id)
            .map(|(entity, _, _, _)| entity)
    };
    match &*edit {
        MapEdit::Place { .. } => {
            if let Some(placed) = map.props.last() {
                commands.spawn(prop_bundle(placed));
            }
        }
        MapEdit::Update {
            id,
            transform,
            mode,
        } => {
            if let Some(entity) = entity_of(*id)
                && let Ok((_, _, mut current, current_mode)) = props.get_mut(entity)
            {
                *current = *transform;
                // only on a change: a genre rebuilds the body when the mode is inserted
                if current_mode != mode {
                    commands.entity(entity).insert(*mode);
                }
            }
        }
        MapEdit::Delete { id } => {
            if let Some(entity) = entity_of(*id) {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Takes a new prop into the hand when its thumbnail is pressed, in place of one already
/// held: it has the file and starts with no `Prop`, at the origin until the genre moves it to
/// the mouse.
fn take_new_prop(
    activate: On<Activate>,
    mut commands: Commands,
    cells: Query<&PropCell>,
    held: Query<(Entity, &Held, Option<&Prop>)>,
) {
    let Ok(cell) = cells.get(activate.entity) else {
        return;
    };
    for (entity, held, prop) in &held {
        put_back(&mut commands, entity, held, prop);
    }
    commands.spawn((
        PropFile(cell.0.clone()),
        PropMode::default(),
        Transform::default(),
        Held { from: None },
        DespawnOnExit(Screen::Gameplay),
    ));
}

/// On a left click over the world, not the window: puts the held prop down where it is, a new
/// one placed and the recents told; with none held, picks up the prop under the mouse.
fn put_down_or_pick_up(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    sheets: Query<&Hovered, With<PropsSheet>>,
    held: Query<(Entity, &PropFile, &Transform, &PropMode, Option<&Prop>), With<Held>>,
    placed: Query<&Transform, With<Prop>>,
    hovered: Res<HoveredProp>,
    mut preferences: ResMut<PropPreferences>,
) {
    let over_window = sheets.iter().any(Hovered::get);
    if !mouse.just_pressed(MouseButton::Left) || over_window {
        return;
    }
    if let Ok((entity, file, transform, mode, prop)) = held.single() {
        match prop {
            Some(prop) => {
                commands.entity(entity).remove::<Held>();
                commands.trigger(MapEdit::Update {
                    id: prop.0,
                    transform: *transform,
                    mode: *mode,
                });
            }
            None => {
                commands.entity(entity).despawn();
                commands.trigger(MapEdit::Place {
                    file: file.0.clone(),
                    transform: *transform,
                });
                preferences.place_recent(&file.0);
            }
        }
        return;
    }
    if let Some(entity) = hovered.0
        && let Ok(transform) = placed.get(entity)
    {
        commands.entity(entity).insert(Held {
            from: Some(*transform),
        });
    }
}

/// Turns the held prop a step about the genre's axis on R.
fn turn_held_prop(
    controls: Controls,
    kind: Option<Res<PropKind>>,
    mut held: Query<&mut Transform, With<Held>>,
) {
    let Some(kind) = kind else {
        return;
    };
    if !controls.just_pressed(TURN_PROP) {
        return;
    }
    for mut transform in &mut held {
        transform.rotation = turned(transform.rotation, kind.turn_axis);
    }
}

/// Scales the held prop by the wheel, a step a notch, while the mouse is over the world: over
/// the window the wheel scrolls the grid.
fn scale_held_prop(
    scroll: Res<AccumulatedMouseScroll>,
    sheets: Query<&Hovered, With<PropsSheet>>,
    mut held: Query<&mut Transform, With<Held>>,
) {
    if sheets.iter().any(Hovered::get) {
        return;
    }
    let notches = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / SCROLL_PX_PER_NOTCH,
    };
    if notches == 0.0 {
        return;
    }
    for mut transform in &mut held {
        transform.scale = Vec3::splat(scaled(transform.scale.x, notches));
    }
}

/// Removes the held prop on Delete, or with none held the one under the mouse: a new one
/// held was never placed, so it only goes.
fn delete_prop(
    mut commands: Commands,
    controls: Controls,
    held: Query<(Entity, Option<&Prop>), With<Held>>,
    placed: Query<&Prop>,
    hovered: Res<HoveredProp>,
) {
    if !controls.just_pressed(DELETE_PROP) {
        return;
    }
    let target = match held.single() {
        Ok((entity, prop)) => {
            if prop.is_none() {
                commands.entity(entity).despawn();
            }
            prop.copied()
        }
        Err(_) => hovered
            .0
            .and_then(|entity| placed.get(entity).ok())
            .copied(),
    };
    if let Some(prop) = target {
        commands.trigger(MapEdit::Delete { id: prop.0 });
    }
}

/// Puts a prop still held back where it was picked up as the editor closes, or drops a new
/// one: nothing was written for it yet.
fn put_back_held_prop(mut commands: Commands, held: Query<(Entity, &Held, Option<&Prop>)>) {
    for (entity, held, prop) in &held {
        put_back(&mut commands, entity, held, prop);
    }
}

// ==== Helpers ================================================================

/// Lets go of the held prop without keeping its move: a picked-up one goes back where it was,
/// a new one goes. Esc does this, as does closing the editor.
pub(super) fn put_back(commands: &mut Commands, entity: Entity, held: &Held, prop: Option<&Prop>) {
    match (held.from, prop) {
        (Some(from), Some(_)) => {
            commands
                .entity(entity)
                .try_insert(from)
                .try_remove::<Held>();
        }
        _ => {
            commands.entity(entity).try_despawn();
        }
    }
}

/// Returns `map` with `edit` applied: a new prop takes `next_id`, which then moves on, and a
/// deleted prop's id is never handed out again. An edit naming a prop the map lacks changes
/// nothing.
fn applied(mut map: MapFile, edit: &MapEdit) -> MapFile {
    match edit {
        MapEdit::Place { file, transform } => {
            map.props.push(PlacedProp {
                id: PropId(map.next_id),
                file: file.clone(),
                position: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale.x,
                mode: PropMode::default(),
            });
            map.next_id += 1;
        }
        MapEdit::Update {
            id,
            transform,
            mode,
        } => {
            if let Some(placed) = map.props.iter_mut().find(|placed| placed.id == *id) {
                placed.position = transform.translation;
                placed.rotation = transform.rotation;
                placed.scale = transform.scale.x;
                placed.mode = *mode;
            }
        }
        MapEdit::Delete { id } => map.props.retain(|placed| placed.id != *id),
    }
    map
}

/// Returns `rotation` turned one step about `axis`.
fn turned(rotation: Quat, axis: Dir3) -> Quat {
    Quat::from_axis_angle(*axis, PROP_TURN_STEP_RAD) * rotation
}

/// Returns `scale` after `notches` of the wheel, a step each, kept between the limits.
fn scaled(scale: f32, notches: f32) -> f32 {
    (scale * PROP_SCALE_STEP.powf(notches)).clamp(PROP_SCALE_MIN, PROP_SCALE_MAX)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-4;

    fn map_of(ids: &[u32]) -> MapFile {
        MapFile {
            next_id: ids.iter().max().map_or(0, |id| id + 1),
            props: ids
                .iter()
                .map(|&id| PlacedProp {
                    id: PropId(id),
                    file: format!("props/{id}.glb"),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    scale: 1.0,
                    mode: PropMode::Fixed,
                })
                .collect(),
        }
    }

    #[test]
    fn placing_a_prop_gives_it_the_next_id_and_moves_it_on() {
        let map = applied(
            map_of(&[0, 1]),
            &MapEdit::Place {
                file: "props/pine.glb".to_string(),
                transform: Transform::from_xyz(1.0, 0.0, 2.0),
            },
        );
        let placed = map.props.last();
        assert_eq!(placed.map(|placed| placed.id), Some(PropId(2)));
        assert_eq!(
            placed.map(|placed| placed.position),
            Some(Vec3::new(1.0, 0.0, 2.0))
        );
        assert_eq!(map.next_id, 3);
    }

    #[test]
    fn moving_a_prop_changes_only_that_prop() {
        let moved = Transform::from_xyz(5.0, 0.0, 0.0).with_scale(Vec3::splat(2.0));
        let map = applied(
            map_of(&[0, 1]),
            &MapEdit::Update {
                id: PropId(1),
                transform: moved,
                mode: PropMode::Fixed,
            },
        );
        assert_eq!(map.props[0], map_of(&[0, 1]).props[0]);
        assert_eq!(map.props[1].position, Vec3::new(5.0, 0.0, 0.0));
        assert!((map.props[1].scale - 2.0).abs() < TOLERANCE);
    }

    #[test]
    fn a_deleted_id_is_never_handed_out_again() {
        let map = applied(map_of(&[0, 1]), &MapEdit::Delete { id: PropId(1) });
        assert_eq!(map.props.len(), 1);
        let map = applied(
            map,
            &MapEdit::Place {
                file: "props/pine.glb".to_string(),
                transform: Transform::IDENTITY,
            },
        );
        assert_eq!(map.props.last().map(|placed| placed.id), Some(PropId(2)));
    }

    #[test]
    fn choosing_a_mode_keeps_where_the_prop_stands() {
        let map = applied(
            map_of(&[0]),
            &MapEdit::Update {
                id: PropId(0),
                transform: Transform::IDENTITY,
                mode: PropMode::Physics,
            },
        );
        assert_eq!(map.props[0].mode, PropMode::Physics);
        assert_eq!(map.props[0].position, Vec3::ZERO);
    }

    #[test]
    fn an_edit_on_a_prop_the_map_lacks_leaves_the_others_untouched() {
        let map = applied(map_of(&[0]), &MapEdit::Delete { id: PropId(7) });
        assert_eq!(map, map_of(&[0]));
    }

    #[test]
    fn turning_goes_one_step_about_the_axis() {
        let rotation = turned(Quat::IDENTITY, Dir3::Y);
        assert!((rotation.to_euler(EulerRot::YXZ).0 - PROP_TURN_STEP_RAD).abs() < TOLERANCE);
    }

    #[test]
    fn scaling_stops_at_its_limits() {
        assert!((scaled(1.0, 1.0) - PROP_SCALE_STEP).abs() < TOLERANCE);
        assert!((scaled(PROP_SCALE_MAX, 3.0) - PROP_SCALE_MAX).abs() < TOLERANCE);
        assert!((scaled(PROP_SCALE_MIN, -3.0) - PROP_SCALE_MIN).abs() < TOLERANCE);
    }
}
