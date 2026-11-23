use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::PrimaryWindow;

use super::prop_editor::edits::{Held, HoveredProp, MapEdit, PropsSheet};
use super::prop_editor::library::{PropLibrary, PropThumbnails};
use super::prop_editor::{PropEditor, PropKind};
use super::{DEV_TOOLS_Z_INDEX, DevUi, dev_window};
use crate::screens::gameplay::map::{Prop, PropMode};
use crate::screens::gameplay::player::PLAYER_Z;
use crate::ui::{ButtonKind, ClosesOnEsc, Dismiss, floating_root, words_button};

// ==== Config =================================================================

// the files a 2D game's props are, and the axis R turns one about: in the plane
const PROP_KIND: PropKind = PropKind {
    extensions: &["png", "jpg"],
    turn_axis: Dir3::Z,
};

// ---- depth: either side of the player, who stands at `PLAYER_Z` -------------
// a step that keeps both above the room's tiles, which sit at 0
const PROP_DEPTH_STEP: f32 = 0.5;

// ---- words: English, as every dev tool's, never read in a release build -----
const IN_FRONT_WORDS: &str = "In Front";
const BEHIND_WORDS: &str = "Behind";

// ---- menu: over the props window, as the explorer's menu is over the explorer
const MENU_Z_INDEX: GlobalZIndex = GlobalZIndex(DEV_TOOLS_Z_INDEX.0 + 10);

// ---- derived ----------------------------------------------------------------
const PROP_FRONT_Z: f32 = PLAYER_Z + PROP_DEPTH_STEP;
const PROP_BEHIND_Z: f32 = PLAYER_Z - PROP_DEPTH_STEP;

// ==== Plugin =================================================================

/// The prop editor in 2D: the props are images, each its own thumbnail, R turns them in the
/// plane, and a held prop follows the cursor. A right click on a placed prop opens a menu
/// putting it in front of the player or behind; a new prop starts behind, as floor and
/// scenery. The camera is photo mode's free camera (`free_camera.rs`).
pub(super) fn plugin(app: &mut App) {
    app.insert_resource(PROP_KIND);
    app.add_systems(
        Update,
        (
            show_thumbnails.run_if(resource_changed::<PropLibrary>),
            (aim_at_props, hold_prop_at_cursor, open_depth_menu).chain(),
            close_depth_menu_on_click_outside,
        )
            .run_if(in_state(PropEditor::On)),
    );
    app.add_observer(start_behind);
    app.add_observer(choose_depth.run_if(in_state(PropEditor::On)));
    app.add_observer(dismiss_depth_menu.run_if(in_state(PropEditor::On)));
}

// ==== Components =============================================================

/// Marks the root of the depth menu, with the prop it is for.
#[derive(Component)]
struct DepthMenu(Entity);

/// Marks the depth menu's sheet, whose `Hovered` tells a click on it from one outside.
#[derive(Component)]
struct DepthMenuSheet;

/// A depth menu item, with the depth it gives.
#[derive(Component)]
struct DepthItem(f32);

// ==== Systems ================================================================

/// Gives every image in the library its thumbnail: the image itself.
fn show_thumbnails(
    library: Res<PropLibrary>,
    mut thumbnails: ResMut<PropThumbnails>,
    asset_server: Res<AssetServer>,
) {
    for entry in &library.0 {
        if !thumbnails.0.contains_key(&entry.file) {
            let image = asset_server.load(entry.file.clone());
            thumbnails.0.insert(entry.file.clone(), image);
        }
    }
}

/// Finds the placed prop under the mouse, the one drawn on top where they overlap, or none
/// over the window.
fn aim_at_props(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<IsDefaultUiCamera>>,
    sheets: Query<&Hovered, With<PropsSheet>>,
    props: Query<(Entity, &Sprite, &GlobalTransform), (With<Prop>, Without<Held>)>,
    images: Res<Assets<Image>>,
    mut hovered: ResMut<HoveredProp>,
) {
    let found = cursor_point(&window, *camera)
        .filter(|_| !sheets.iter().any(Hovered::get))
        .and_then(|point| {
            props
                .iter()
                .filter(|(_, sprite, global)| {
                    let size = sprite_size(sprite, &images);
                    is_inside(point, global, size)
                })
                .max_by(|(_, _, a), (_, _, b)| a.translation().z.total_cmp(&b.translation().z))
                .map(|(entity, _, _)| entity)
        });
    if hovered.0 != found {
        hovered.0 = found;
    }
}

/// Moves the held prop to the cursor, keeping its depth. Not over the window, where the cursor
/// is choosing another prop.
fn hold_prop_at_cursor(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<IsDefaultUiCamera>>,
    sheets: Query<&Hovered, With<PropsSheet>>,
    mut held: Query<&mut Transform, With<Held>>,
) {
    if sheets.iter().any(Hovered::get) {
        return;
    }
    let Some(point) = cursor_point(&window, *camera) else {
        return;
    };
    for mut transform in &mut held {
        transform.translation = point.extend(transform.translation.z);
    }
}

/// Opens the depth menu at the cursor on a right click on a placed prop, with nothing held.
fn open_depth_menu(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    hovered: Res<HoveredProp>,
    held: Query<(), With<Held>>,
    props: Query<&Transform, With<Prop>>,
    menus: Query<Entity, With<DepthMenu>>,
) {
    if !mouse.just_pressed(MouseButton::Right) || !held.is_empty() {
        return;
    }
    let (Some(prop), Some(at)) = (hovered.0, window.cursor_position()) else {
        return;
    };
    let Ok(transform) = props.get(prop) else {
        return;
    };
    for menu in &menus {
        commands.entity(menu).despawn();
    }
    let in_front = transform.translation.z > PLAYER_Z;
    let item = |words, depth, current| {
        let kind = if current {
            ButtonKind::Primary
        } else {
            ButtonKind::Choice
        };
        (words_button(words, kind), DepthItem(depth))
    };
    commands.spawn((
        floating_root(MENU_Z_INDEX),
        Pickable::IGNORE,
        DevUi,
        ClosesOnEsc,
        DepthMenu(prop),
        DespawnOnExit(PropEditor::On),
        children![(
            dev_window(Node {
                left: px(at.x),
                top: px(at.y),
                ..default()
            }),
            DepthMenuSheet,
            // a sheet of the editor's, so a click on an item never reaches the prop behind it
            PropsSheet,
            Hovered::default(),
            children![
                item(IN_FRONT_WORDS, PROP_FRONT_Z, in_front),
                item(BEHIND_WORDS, PROP_BEHIND_Z, !in_front),
            ],
        )],
    ));
}

/// Closes the depth menu when a press lands outside it. That press goes on to what is under
/// it, as in the OS's own menus.
fn close_depth_menu_on_click_outside(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    sheets: Query<&Hovered, With<DepthMenuSheet>>,
    menus: Query<Entity, With<DepthMenu>>,
) {
    if !mouse.just_pressed(MouseButton::Left) || sheets.iter().any(Hovered::get) {
        return;
    }
    for menu in &menus {
        commands.entity(menu).despawn();
    }
}

/// Starts a new prop in the hand behind the player, as floor and scenery.
fn start_behind(add: On<Add<Held>>, mut held: Query<&mut Transform, (With<Held>, Without<Prop>)>) {
    if let Ok(mut transform) = held.get_mut(add.entity) {
        transform.translation.z = PROP_BEHIND_Z;
    }
}

/// Puts the prop at the depth pressed, keeping where it stands and its mode, and closes the
/// menu.
fn choose_depth(
    activate: On<Activate>,
    mut commands: Commands,
    items: Query<&DepthItem>,
    menus: Query<(Entity, &DepthMenu)>,
    props: Query<(&Prop, &Transform, &PropMode)>,
) {
    let Ok(item) = items.get(activate.entity) else {
        return;
    };
    for (menu, for_prop) in &menus {
        if let Ok((prop, transform, mode)) = props.get(for_prop.0) {
            let mut transform = *transform;
            transform.translation.z = item.0;
            commands.trigger(MapEdit::Update {
                id: prop.0,
                transform,
                mode: *mode,
            });
        }
        commands.entity(menu).despawn();
    }
}

/// Closes the depth menu when Esc dismisses it, before the editor's window.
fn dismiss_depth_menu(
    dismiss: On<Dismiss>,
    mut commands: Commands,
    menus: Query<(), With<DepthMenu>>,
) {
    if menus.contains(dismiss.entity) {
        commands.entity(dismiss.entity).despawn();
    }
}

// ==== Helpers ================================================================

/// Returns the point in the room under the cursor, or `None` outside the window.
fn cursor_point(window: &Window, (camera, global): (&Camera, &GlobalTransform)) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    camera.viewport_to_world_2d(global, cursor).ok()
}

/// Returns a sprite's size before its transform: its own `custom_size`, or its image's.
fn sprite_size(sprite: &Sprite, images: &Assets<Image>) -> Vec2 {
    sprite.custom_size.unwrap_or_else(|| {
        images
            .get(&sprite.image)
            .map_or(Vec2::ZERO, Image::size_f32)
    })
}

/// Tells whether `point` in the room falls on a sprite `size` big, centred on its entity and
/// turned and scaled with it.
fn is_inside(point: Vec2, global: &GlobalTransform, size: Vec2) -> bool {
    let local = global
        .affine()
        .inverse()
        .transform_point3(point.extend(global.translation().z));
    local.x.abs() <= size.x / 2.0 && local.y.abs() <= size.y / 2.0
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_on_a_sprite_is_inside_and_one_beside_it_is_not() {
        let global = GlobalTransform::from(Transform::from_xyz(100.0, 50.0, PROP_BEHIND_Z));
        let size = Vec2::new(64.0, 128.0);
        assert!(is_inside(Vec2::new(120.0, 100.0), &global, size));
        assert!(!is_inside(Vec2::new(140.0, 50.0), &global, size));
    }

    #[test]
    fn a_turned_and_scaled_sprite_is_hit_where_it_is_drawn() {
        let global = GlobalTransform::from(
            Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(2.0)),
        );
        // 10 wide and 40 tall, turned a quarter and doubled: 80 wide and 20 tall on screen
        let size = Vec2::new(10.0, 40.0);
        assert!(is_inside(Vec2::new(35.0, 0.0), &global, size));
        assert!(!is_inside(Vec2::new(0.0, 15.0), &global, size));
    }

    #[test]
    fn in_front_draws_over_the_player_and_behind_under_it() {
        const { assert!(PROP_FRONT_Z > PLAYER_Z) };
        const { assert!(PROP_BEHIND_Z < PLAYER_Z) };
        // still over the room's tiles, which sit at 0
        const { assert!(PROP_BEHIND_Z > 0.0) };
    }
}
