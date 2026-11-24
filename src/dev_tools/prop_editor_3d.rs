use avian3d::prelude::{AngularVelocity, Collider, ColliderDisabled, LinearVelocity, RigidBody};
use bevy::camera::RenderTarget;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::RenderLayers;
use bevy::picking::hover::Hovered;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::render::gpu_readback::{ReadbackComplete, ReadbackOnce};
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::window::PrimaryWindow;
use bevy::world_serialization::WorldInstanceReady;

use super::prop_editor::edits::{Held, HoveredProp, PropsSheet};
use super::prop_editor::library::{PropLibrary, PropThumbnails};
use super::prop_editor::{PropEditor, PropKind};
use crate::screens::Screen;
use crate::screens::gameplay::map::{Prop, PropMode};
use crate::screens::gameplay::playroom::PlayroomCamera;

mod mode_menu;

// ==== Config =================================================================

// the files a 3D game's props are, and the axis R turns one about
const PROP_KIND: PropKind = PropKind {
    extensions: &["glb"],
    turn_axis: Dir3::Y,
};

// ---- holding: where a prop in the hand goes when the mouse points at the sky -
const HELD_PROP_DISTANCE_M: f32 = 6.0;

// ---- thumbnails: each model drawn once, alone, by a camera of its own -------
const THUMBNAIL_SIZE_PX: u32 = 128;
// a layer no other camera draws, so neither the playroom nor its sun shows in a thumbnail
const THUMBNAIL_LAYER: usize = 7;
// far under the level, out of every other camera's view while it loads
const THUMBNAIL_STAGE: Vec3 = Vec3::new(0.0, -500.0, 0.0);
// from the front, a little to the right and above, as a catalogue shows a thing
const THUMBNAIL_VIEW_FROM: Vec3 = Vec3::new(0.6, 0.5, 1.0);
// room around the model, so its edges never touch the thumbnail's
const THUMBNAIL_MARGIN: f32 = 1.15;
const THUMBNAIL_FOV_DEG: f32 = 35.0;
// bright, and lit all round as well, so a dark model still reads against the dark glass
const THUMBNAIL_LIGHT_LUX: f32 = 20_000.0;
const THUMBNAIL_AMBIENT_BRIGHTNESS: f32 = 2_000.0;
// a model not drawn by then never will be: a broken file, given up on so the others go on;
// long enough for the first thumbnail, which waits for its camera's shaders to compile
const THUMBNAIL_GIVE_UP_SECS: f32 = 20.0;

// ---- derived ----------------------------------------------------------------
const THUMBNAIL_FOV_RAD: f32 = THUMBNAIL_FOV_DEG.to_radians();

// ==== Plugin =================================================================

/// The prop editor in 3D: the props are models, R turns them about the vertical, and a held
/// prop rests its lowest point where the cursor points. Every model gets a thumbnail, drawn
/// once by a camera of its own. A held physics prop is kinematic, so it neither falls from
/// the mouse nor is pushed while held; a held prop of any mode collides with nothing, so it
/// never shoves the player holding it; and a quick right click on a placed prop opens its
/// mode menu (`mode_menu.rs`). The camera is photo mode's free camera
/// (`free_camera.rs`), which the editor lifts off the body.
pub(super) fn plugin(app: &mut App) {
    app.insert_resource(PROP_KIND);
    app.init_resource::<Thumbnails>();
    app.add_plugins(mode_menu::plugin);
    app.add_systems(
        Update,
        (
            (aim_at_props, start_thumbnail, finish_thumbnail),
            hold_prop_at_cursor,
        )
            .chain()
            .run_if(in_state(PropEditor::On)),
    );
    app.add_observer(show_thumbnail_model);
    app.add_observer(check_thumbnail_drawn);
    app.add_observer(hold_physics_still);
    app.add_observer(let_physics_go);
    app.add_observer(disable_held_colliders);
    app.add_observer(enable_put_down_colliders);
}

// ==== Components =============================================================

/// Marks the model a thumbnail is being drawn of.
#[derive(Component)]
struct ThumbnailModel;

/// Marks a read-back of the thumbnail being drawn.
#[derive(Component)]
struct ThumbnailReadback;

// ==== Resources ==============================================================

/// The thumbnail being drawn, one at a time, and every file tried, so one that never loads is
/// not tried again and again.
#[derive(Resource, Default)]
struct Thumbnails {
    drawing: Option<ThumbnailJob>,
    tried: HashSet<String>,
}

/// One thumbnail being drawn: its file, the image, the model, camera and light, when it
/// started, and how far along it is: the model in, the camera placed, a read-back under way,
/// something drawn.
struct ThumbnailJob {
    file: String,
    image: Handle<Image>,
    rig: [Entity; 3],
    started_secs: f32,
    ready: bool,
    framed: bool,
    reading: bool,
    drawn: bool,
}

// ==== Systems ================================================================

/// Finds the placed prop under the mouse, or none over the window or the sky.
fn aim_at_props(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<PlayroomCamera>>,
    sheets: Query<&Hovered, With<PropsSheet>>,
    mut ray_cast: MeshRayCast,
    held: Query<Entity, With<Held>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    props: Query<(), With<Prop>>,
    mut hovered: ResMut<HoveredProp>,
) {
    let held = descendants(held.iter(), &children);
    let found = cursor_ray(&window, *camera)
        .filter(|_| !sheets.iter().any(Hovered::get))
        .and_then(|ray| {
            let filter = |entity| !held.contains(&entity);
            let settings = MeshRayCastSettings::default().with_filter(&filter);
            let (entity, _) = ray_cast.cast_ray(ray, &settings).first()?;
            parents
                .iter_ancestors(*entity)
                .chain([*entity])
                .find(|ancestor| props.contains(*ancestor))
        });
    if hovered.0 != found {
        hovered.0 = found;
    }
}

/// Moves the held prop to where the cursor points, its lowest point resting on what it
/// points at, or ahead of the camera when it points at the sky. Not over the window, where
/// the cursor is choosing another prop.
fn hold_prop_at_cursor(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<PlayroomCamera>>,
    sheets: Query<&Hovered, With<PropsSheet>>,
    mut ray_cast: MeshRayCast,
    mut held: Query<(Entity, &mut Transform), With<Held>>,
    children: Query<&Children>,
    bounds: Query<(&Aabb, &GlobalTransform)>,
) {
    if sheets.iter().any(Hovered::get) {
        return;
    }
    let Some(ray) = cursor_ray(&window, *camera) else {
        return;
    };
    for (entity, mut transform) in &mut held {
        let parts = descendants([entity], &children);
        let filter = |part| !parts.contains(&part);
        let settings = MeshRayCastSettings::default().with_filter(&filter);
        let lowest = parts
            .iter()
            .filter_map(|part| bounds.get(*part).ok())
            .map(|(aabb, global)| world_bounds(aabb, global).0.y)
            .reduce(f32::min);
        let surface = match ray_cast.cast_ray(ray, &settings).first() {
            Some((_, hit)) => hit.point,
            None => ray.origin + *ray.direction * HELD_PROP_DISTANCE_M,
        };
        transform.translation = resting_on(surface, transform.translation.y, lowest);
    }
}

/// Starts drawing the thumbnail of the first file in the library that has none yet: its model
/// on the thumbnail stage, and a camera and light on the thumbnail layer, the camera off until
/// the model is in.
fn start_thumbnail(
    mut commands: Commands,
    time: Res<Time<Real>>,
    library: Res<PropLibrary>,
    thumbnail_images: Res<PropThumbnails>,
    mut thumbnails: ResMut<Thumbnails>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    if thumbnails.drawing.is_some() {
        return;
    }
    let Some(entry) = library.0.iter().find(|entry| {
        !thumbnail_images.0.contains_key(&entry.file) && !thumbnails.tried.contains(&entry.file)
    }) else {
        return;
    };
    let mut target = Image::new_target_texture(
        THUMBNAIL_SIZE_PX,
        THUMBNAIL_SIZE_PX,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    );
    // read back to tell when the camera has drawn it, as recording reads its frames
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let image = images.add(target);
    let layer = RenderLayers::layer(THUMBNAIL_LAYER);
    let model = commands
        .spawn((
            WorldAssetRoot(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(entry.file.clone())),
            ),
            Transform::from_translation(THUMBNAIL_STAGE),
            ThumbnailModel,
            DespawnOnExit(Screen::Gameplay),
        ))
        .id();
    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                is_active: false,
                clear_color: Color::NONE.into(),
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
            AmbientLight {
                brightness: THUMBNAIL_AMBIENT_BRIGHTNESS,
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: THUMBNAIL_FOV_RAD,
                ..default()
            }),
            layer.clone(),
            DespawnOnExit(Screen::Gameplay),
        ))
        .id();
    let light = commands
        .spawn((
            DirectionalLight {
                illuminance: THUMBNAIL_LIGHT_LUX,
                ..default()
            },
            Transform::from_translation(THUMBNAIL_VIEW_FROM).looking_at(Vec3::ZERO, Vec3::Y),
            layer,
            DespawnOnExit(Screen::Gameplay),
        ))
        .id();
    let file = entry.file.clone();
    thumbnails.tried.insert(file.clone());
    thumbnails.drawing = Some(ThumbnailJob {
        file,
        image,
        rig: [model, camera, light],
        started_secs: time.elapsed_secs(),
        ready: false,
        framed: false,
        reading: false,
        drawn: false,
    });
}

/// Frames the model the frame after it is in, once its parts have their places, and lets the
/// camera draw it; then reads the image back until it holds something drawn, since Bevy draws
/// nothing for a new camera until its shaders are compiled, which takes frames of its own. The
/// drawn image becomes the file's thumbnail, and the model, camera and light go. A model still
/// not drawn after `THUMBNAIL_GIVE_UP_SECS` is given up on, so a broken file never holds up the
/// others.
fn finish_thumbnail(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut thumbnails: ResMut<Thumbnails>,
    mut thumbnail_images: ResMut<PropThumbnails>,
    children: Query<&Children>,
    bounds: Query<(&Aabb, &GlobalTransform)>,
    mut cameras: Query<(&mut Camera, &mut Transform)>,
) {
    let Some(job) = thumbnails.drawing.as_mut() else {
        return;
    };
    let [model, camera, _] = job.rig;
    if job.ready && !job.framed {
        let (low, high) = descendants([model], &children)
            .iter()
            .filter_map(|part| bounds.get(*part).ok())
            .map(|(aabb, global)| world_bounds(aabb, global))
            .reduce(|(low, high), (part_low, part_high)| (low.min(part_low), high.max(part_high)))
            .unwrap_or((THUMBNAIL_STAGE - Vec3::ONE, THUMBNAIL_STAGE + Vec3::ONE));
        if let Ok((mut lens, mut place)) = cameras.get_mut(camera) {
            *place = framing(low, high);
            lens.is_active = true;
        }
        job.framed = true;
        return;
    }
    let given_up = time.elapsed_secs() - job.started_secs > THUMBNAIL_GIVE_UP_SECS;
    if job.drawn || given_up {
        if job.drawn {
            thumbnail_images
                .0
                .insert(job.file.clone(), job.image.clone());
        } else {
            warn!("no thumbnail for {}: it was never drawn", job.file);
        }
        for part in job.rig {
            commands.entity(part).try_despawn();
        }
        thumbnails.drawing = None;
        return;
    }
    if job.framed && !job.reading {
        commands.spawn((ReadbackOnce::texture(job.image.clone()), ThumbnailReadback));
        job.reading = true;
    }
}

/// Puts every part of a thumbnail's model on the thumbnail layer once it is in, since a layer
/// is not handed down from the root to the meshes.
fn show_thumbnail_model(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    models: Query<(), With<ThumbnailModel>>,
    children: Query<&Children>,
    mut thumbnails: ResMut<Thumbnails>,
) {
    if !models.contains(ready.entity) {
        return;
    }
    for part in children.iter_descendants(ready.entity) {
        commands
            .entity(part)
            .insert(RenderLayers::layer(THUMBNAIL_LAYER));
    }
    if let Some(job) = thumbnails.drawing.as_mut() {
        job.ready = true;
    }
}

/// Tells the thumbnail being drawn whether its image, read back, holds anything drawn yet.
fn check_thumbnail_drawn(
    readback: On<ReadbackComplete>,
    mut commands: Commands,
    readbacks: Query<(), With<ThumbnailReadback>>,
    mut thumbnails: ResMut<Thumbnails>,
) {
    if !readbacks.contains(readback.entity) {
        return;
    }
    commands.entity(readback.entity).despawn();
    if let Some(job) = thumbnails.drawing.as_mut() {
        job.reading = false;
        job.drawn = is_drawn(&readback.data);
    }
}

/// Holds a physics prop still while it is in the hand: kinematic, it follows the mouse and is
/// neither pulled down nor pushed.
fn hold_physics_still(
    add: On<Add<Held>>,
    mut commands: Commands,
    props: Query<&PropMode, With<Prop>>,
) {
    if props.get(add.entity) == Ok(&PropMode::Physics) {
        // still, too: a kinematic body keeps the speed it had, so one picked up while it fell
        // would go on falling out of the hand
        commands.entity(add.entity).insert((
            RigidBody::Kinematic,
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
    }
}

/// Lets a physics prop fall again as it leaves the hand.
fn let_physics_go(
    remove: On<Remove<Held>>,
    mut commands: Commands,
    props: Query<&PropMode, With<Prop>>,
) {
    if props.get(remove.entity) == Ok(&PropMode::Physics) {
        commands
            .entity(remove.entity)
            .try_insert(RigidBody::Dynamic);
    }
}

/// Turns off every collider of a prop as it is taken into the hand: moved by the mouse, it
/// would shove whatever it passes through, the player holding it first of all. The model is
/// still aimed at by its meshes, which the editor ray casts, not by its colliders.
fn disable_held_colliders(
    add: On<Add<Held>>,
    mut commands: Commands,
    children: Query<&Children>,
    colliders: Query<(), With<Collider>>,
) {
    for part in descendants([add.entity], &children) {
        if colliders.contains(part) {
            commands.entity(part).insert(ColliderDisabled);
        }
    }
}

/// Turns a prop's colliders back on as it leaves the hand: put down, put back, or gone.
fn enable_put_down_colliders(
    remove: On<Remove<Held>>,
    mut commands: Commands,
    children: Query<&Children>,
    disabled: Query<(), With<ColliderDisabled>>,
) {
    for part in descendants([remove.entity], &children) {
        if disabled.contains(part) {
            commands.entity(part).try_remove::<ColliderDisabled>();
        }
    }
}

// ==== Helpers ================================================================

/// Returns the ray from the camera through the cursor, or `None` while the cursor is held for
/// turning, or outside the window.
fn cursor_ray(window: &Window, (camera, global): (&Camera, &GlobalTransform)) -> Option<Ray3d> {
    let cursor = window.cursor_position()?;
    camera.viewport_to_world(global, cursor).ok()
}

/// Returns `roots` and everything under them.
fn descendants(
    roots: impl IntoIterator<Item = Entity>,
    children: &Query<&Children>,
) -> HashSet<Entity> {
    let mut all = HashSet::new();
    for root in roots {
        all.insert(root);
        all.extend(children.iter_descendants(root));
    }
    all
}

/// Tells whether an RGBA8 image read back holds anything drawn: a pixel not fully see-through,
/// on a camera that clears to nothing.
fn is_drawn(rgba: &[u8]) -> bool {
    let (pixels, _) = rgba.as_chunks::<4>();
    pixels.iter().any(|[_, _, _, alpha]| *alpha > 0)
}

/// Returns the lowest and highest corners, in the world, of a mesh's local box.
fn world_bounds(aabb: &Aabb, global: &GlobalTransform) -> (Vec3, Vec3) {
    let center = Vec3::from(aabb.center);
    let half = Vec3::from(aabb.half_extents);
    let mut low = Vec3::INFINITY;
    let mut high = Vec3::NEG_INFINITY;
    for corner in 0..8 {
        let sign = Vec3::new(
            if corner & 1 == 0 { -1.0 } else { 1.0 },
            if corner & 2 == 0 { -1.0 } else { 1.0 },
            if corner & 4 == 0 { -1.0 } else { 1.0 },
        );
        let point = global.transform_point(center + half * sign);
        low = low.min(point);
        high = high.max(point);
    }
    (low, high)
}

/// Returns where a prop's origin goes for its lowest point, `lowest` in the world while its
/// origin is at `origin_y`, to rest on `surface`. A prop whose model is not in yet rests by
/// its origin.
fn resting_on(surface: Vec3, origin_y: f32, lowest: Option<f32>) -> Vec3 {
    let above_lowest = lowest.map_or(0.0, |lowest| origin_y - lowest);
    surface + Vec3::Y * above_lowest
}

/// Returns where a thumbnail's camera goes to show a model filling `low` to `high` whole,
/// looking at its middle from `THUMBNAIL_VIEW_FROM`'s side.
fn framing(low: Vec3, high: Vec3) -> Transform {
    let middle = (low + high) / 2.0;
    let radius = (high - low).length() / 2.0;
    let distance = radius / (THUMBNAIL_FOV_RAD / 2.0).sin() * THUMBNAIL_MARGIN;
    Transform::from_translation(middle + THUMBNAIL_VIEW_FROM.normalize() * distance)
        .looking_at(middle, Vec3::Y)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-4;

    #[test]
    fn a_model_of_any_size_is_framed_whole() {
        for size in [0.1, 1.0, 40.0] {
            let low = Vec3::splat(-size);
            let high = Vec3::splat(size);
            let camera = framing(low, high);
            let radius = (high - low).length() / 2.0;
            let distance = camera.translation.length();
            // the sphere around the model fits inside the view's cone
            assert!(radius / distance <= (THUMBNAIL_FOV_RAD / 2.0).sin() + TOLERANCE);
            let forward = camera.forward();
            assert!(forward.dot(-camera.translation.normalize()) > 1.0 - TOLERANCE);
        }
    }

    #[test]
    fn a_prop_whose_origin_is_above_its_base_rests_its_base_on_the_surface() {
        let surface = Vec3::new(2.0, 1.0, 3.0);
        // the origin 1.77 m above the lowest point
        let position = resting_on(surface, 5.0, Some(5.0 - 1.77));
        assert!((position.y - (1.0 + 1.77)).abs() < TOLERANCE);
        assert!((position.x - 2.0).abs() < TOLERANCE);
    }

    #[test]
    fn a_prop_not_in_yet_rests_by_its_origin() {
        let surface = Vec3::new(0.0, 1.0, 0.0);
        assert_eq!(resting_on(surface, 4.0, None), surface);
    }

    #[test]
    fn an_image_still_see_through_is_not_drawn_yet() {
        assert!(!is_drawn(&[0; 16]));
        assert!(is_drawn(&[0, 0, 0, 0, 40, 30, 20, 255]));
    }
}
