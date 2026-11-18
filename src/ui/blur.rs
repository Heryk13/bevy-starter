use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::camera::CameraUpdateSystems;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::{Core2d, Core2dSystems, FullscreenShader};
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy::render::diagnostic::RecordDiagnostics;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer};
use bevy::render::render_resource::{
    AsBindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, Buffer,
    BufferInitDescriptor, BufferUsages, CachedRenderPipelineId, ColorTargetState, ColorWrites,
    FilterMode, FragmentState, Operations, PipelineCache, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureView,
};
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::texture::GpuImage;
use bevy::render::view::ViewTarget;
use bevy::render::{RenderApp, RenderStartup};
use bevy::shader::ShaderRef;
use bevy::ui_render::{stack_z_offsets, ui_pass};
use bevy::window::{PrimaryWindow, WindowResized};

use super::SURFACE;
use super::panel::{Card, Entrance, SCREEN_DIM};

// ==== Config =================================================================

// ---- glass: a dual Kawase on a halved copy of the image ---------------------
// the descent halves the copy until it is this tall or less, so the blur covers the same share
// of the screen at every resolution; tuned by eye at 1080p
const GLASS_TARGET_HEIGHT_PX: u32 = 270;
// how far apart each pass's bilinear reads land, in texels of the image it reads; tuned by eye
// with GLASS_TARGET_HEIGHT_PX
const KAWASE_OFFSET_TEXELS: f32 = 1.0;

const KAWASE_DOWN_SHADER_PATH: &str = "shaders/kawase_down.wesl";
const KAWASE_UP_SHADER_PATH: &str = "shaders/kawase_up.wesl";

const GLASS_SHADER_PATH: &str = "shaders/glass.wesl";

// the copies' format, which both pipelines must write: a 16-bit float keeps the colours of an
// sRGB camera and an HDR one alike, so the copies outlive gameplay switching the cameras to HDR
const GLASS_COPY_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

// ---- cards ------------------------------------------------------------------
// how far a card's tint darkens from its top edge to its bottom one, 0.0 to 1.0: a thick
// sheet lit from above, the other half of what the card's rim says
const CARD_SHADE: f32 = 0.4;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        ExtractComponentPlugin::<GlassCopies>::default(),
        ExtractComponentPlugin::<FrozenLayer>::default(),
        ExtractResourcePlugin::<ReduceTransparency>::default(),
        UiMaterialPlugin::<GlassMaterial>::default(),
    ));
    app.init_resource::<FrozenBackground>();
    app.init_resource::<ReduceTransparency>();
    app.init_resource::<ScreensCovered>();
    app.add_systems(
        Startup,
        (spawn_layer_cameras, generate_level_images).chain(),
    );
    app.add_observer(target_layer_camera);
    app.add_observer(give_card_glass);
    app.add_observer(retarget_open_glass);
    app.add_systems(
        Update,
        (
            generate_level_images.run_if(on_message::<WindowResized>),
            apply_reduce_transparency.run_if(resource_changed::<ReduceTransparency>),
        ),
    );
    app.add_systems(
        PostUpdate,
        (
            activate_layer_cameras,
            freeze_settled_layers,
            update_screens_covered,
        )
            .chain()
            // so `ScreensCovered` is settled before Bevy works out what each camera sees, and a
            // camera it turns back on is still in time for this frame
            .before(CameraUpdateSystems)
            .in_set(ScreensCoveredSystems),
    );
    // an app built without rendering has no render world, so nothing to blur in it
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app.add_systems(RenderStartup, init_kawase_pipelines);
    // the bounds ui_pass has: after everything that may swap the main image (the MSAA
    // writeback of a camera that does not clear does), before the layer's own UI is drawn
    render_app.add_systems(
        Core2d,
        blur_layer_backdrops
            .before(ui_pass)
            .after(Core2dSystems::PostProcess),
    );
}

// ==== System Sets ============================================================

/// The systems that settle `ScreensCovered`, in `PostUpdate`. Whatever stops drawing under
/// the layers runs `.after` this set and `.before(CameraUpdateSystems)`: a camera turned back
/// on later in the frame is too late for Bevy to work out what it sees, and draws nothing.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ScreensCoveredSystems;

// ==== Components =============================================================

/// Marks a UI root that floats over the screens: an overlay, the pause menu, a dialog, a dev
/// window. It is drawn by the camera of its layer, and while any that blocks the pointer is
/// alive the screens behind are blurred.
#[derive(Component)]
pub(crate) struct Floating;

/// Marks the camera that draws the `Floating` roots of one layer, the one named by this
/// `GlobalZIndex`, above the screen camera and every lower layer. The render world tells the
/// layer cameras apart by their `GlassCopies`, which no other camera has.
#[derive(Component)]
struct LayerCamera(GlobalZIndex);

/// Whether a layer may keep the copy it already blurred: the background was called still, and
/// every floating root at or below the layer has finished coming in, so nothing the layer
/// blurs changes any more. Extracted with its camera, where the passes read it.
#[derive(Component, ExtractComponent, Clone, Default)]
#[extract_app(RenderApp)]
struct FrozenLayer {
    frozen: bool,
    // whether the layer is showing a copy finished in an earlier frame. The one thing the
    // render world tells the main one: only the passes know a copy was really made, so they
    // write it and the main world reads it a frame later. Every clone the extraction makes
    // shares the one flag, which is what lets it cross between the two worlds
    copy_kept: Arc<AtomicBool>,
}

/// The images a layer camera blurs into, made for the window's size: the descent, each half
/// the one before it, from half the screen down to one level past the backdrop's (the card's),
/// and two ascents, the backdrop's and the card's, each back up toward half the screen, where
/// its glass reads it. Extracted with its camera, so the passes find the images to write in
/// the same query as the view.
#[derive(Component, ExtractComponent, Clone)]
#[extract_app(RenderApp)]
struct GlassCopies {
    descent: Vec<Handle<Image>>,
    backdrop_ascent: Vec<Handle<Image>>,
    card_ascent: Vec<Handle<Image>>,
}

impl GlassCopies {
    /// Returns the image the glass reads, where the backdrop's blur ends: the top of the
    /// backdrop's ascent, or the first level when a window so small it takes a single level
    /// leaves that ascent empty.
    fn backdrop_copy(&self) -> Option<&Handle<Image>> {
        self.backdrop_ascent.last().or_else(|| self.descent.first())
    }

    /// Returns the image a card reads, where the card's blur ends: the top of the card's
    /// ascent, which is never empty, since the card's chain always climbs from one level past
    /// the backdrop's.
    fn card_copy(&self) -> Option<&Handle<Image>> {
        self.card_ascent.last()
    }
}

// ==== Resources ==============================================================

/// The floating layers, each named by the `GlobalZIndex` of its roots. Every feature that
/// floats registers its own from its plugin, so this file never names them, and each layer
/// gets a camera of its own, drawn over the layers below it.
#[derive(Resource, Default)]
pub(crate) struct GlassLayers(pub(crate) Vec<GlobalZIndex>);

/// Whether nothing behind the floating layers can change right now, so a layer may keep the
/// copy it already blurred instead of blurring a new one every frame. Off unless a feature
/// turns it on: whatever stills the world says so from its own plugin, with
/// `freeze_background` and `unfreeze_background`, as it registers its layer, so this file
/// never names the pause menu or the main menu and a game adds its own still moments the same
/// way. A background nobody called still is blurred every frame, which is always right, only
/// slower.
#[derive(Resource, Default)]
pub(crate) struct FrozenBackground(pub(crate) bool);

/// Whether the glass is drawn flat, for a player who reads better without it or whose machine
/// runs better without it: no layer is blurred, a backdrop is the dim alone over the sharp
/// screen and a card a solid fill. The Settings overlay writes it and saves it, so this file
/// never names the overlay. Extracted, since the passes are what it stops.
#[derive(Resource, ExtractResource, Clone, Default)]
#[extract_app(RenderApp)]
pub(crate) struct ReduceTransparency(pub(crate) bool);

/// Whether the screens, the game and every root that does not float, are wholly hidden: an
/// open layer shows a blur it already made, and its backdrop covers the window. Whatever is
/// costly to draw under the layers may stop while this is on, as the 3D level's camera does,
/// and must draw again as soon as it is off, since the next copy is made from what it draws.
/// It counts on every `Floating` root covering the window, as `floating_root` makes them.
#[derive(Resource, Default, PartialEq)]
pub(crate) struct ScreensCovered(pub(crate) bool);

/// The two passes of the glass's dual Kawase, in the render world: the layout, sampler and
/// offset buffer both shaders bind, and the ids of the pipelines queued for them, which the
/// passes look up in the `PipelineCache` each frame, since a pipeline compiles in the
/// background and is not ready the moment it is asked for. The offset is written once, here,
/// since `KAWASE_OFFSET_TEXELS` never changes while the game runs.
#[derive(Resource)]
struct KawasePipelines {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    down: CachedRenderPipelineId,
    up: CachedRenderPipelineId,
    offset: Buffer,
}

// ==== Assets =================================================================

/// The glass a floating root and its card are drawn with: one of its layer's blurred copies,
/// read where the node sits on screen, under a tint that may darken toward the node's bottom,
/// all faded by the node's alpha. Flat, it draws the tint alone, as opaque as the tint's own
/// alpha: what Reduce Transparency asks for.
#[derive(AsBindGroup, Asset, TypePath, Clone)]
pub(super) struct GlassMaterial {
    #[uniform(0)]
    tint: LinearRgba,
    #[uniform(0)]
    pub(super) alpha: f32,
    // 1 while Reduce Transparency is on, 0 otherwise: a shader's uniform cannot hold a `bool`
    #[uniform(0)]
    flat: u32,
    // how far the tint has darkened by the node's bottom edge, 0.0 for none
    #[uniform(0)]
    shade: f32,
    #[texture(1)]
    #[sampler(2)]
    copy: Handle<Image>,
}

impl GlassMaterial {
    /// Returns the glass of a floating root, still invisible: its layer's backdrop copy under
    /// the dim, or flat, the dim alone over the sharp screen.
    fn backdrop(copy: Handle<Image>, flat: bool) -> Self {
        Self {
            tint: SCREEN_DIM.into(),
            alpha: 0.0,
            flat: u32::from(flat),
            shade: 0.0,
            copy,
        }
    }

    /// Returns the glass of a card, still invisible: its layer's card copy under the card's
    /// tint, or flat, the solid surface.
    fn card(copy: Handle<Image>, flat: bool) -> Self {
        Self {
            tint: card_tint(flat),
            alpha: 0.0,
            flat: u32::from(flat),
            shade: CARD_SHADE,
            copy,
        }
    }
}

impl UiMaterial for GlassMaterial {
    fn fragment_shader() -> ShaderRef {
        GLASS_SHADER_PATH.into()
    }

    // where a fill goes: over the node's shadow and under its border. Bevy draws a material
    // over the border by default, and the glass covers its whole node, so a card's rim
    // would never show
    fn stack_z_offset() -> f32 {
        stack_z_offsets::BACKGROUND_COLOR
    }
}

// ==== Systems ================================================================

/// Spawns a camera for each of the `GlassLayers`, ordered by its `GlobalZIndex` so the
/// layers draw in the order their roots stack. None clears: each draws over what the cameras
/// below it drew, blurred or not, which it would otherwise wipe.
fn spawn_layer_cameras(mut commands: Commands, glass_layers: Res<GlassLayers>) {
    for z_index in &glass_layers.0 {
        commands.spawn((
            Camera2d,
            Camera {
                order: z_index.0 as isize,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            // a Camera2d draws every sprite and tilemap in the world, and this one would draw
            // them again, sharp, over the blurred image. UI ignores render layers, so the
            // floating panels still reach it through UiTargetCamera
            RenderLayers::none(),
            LayerCamera(*z_index),
            FrozenLayer::default(),
        ));
    }
}

/// Gives every layer camera `GlassCopies` of its own, sized for the window in physical pixels:
/// each layer blurs a different image, so sharing them would let one wipe another's. Runs again
/// whenever the window resizes; the copies it replaces free their images once dropped.
fn generate_level_images(
    window: Single<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    layer_cameras: Query<(Entity, &FrozenLayer), With<LayerCamera>>,
    mut commands: Commands,
) {
    let window_size = window.physical_size();
    let levels = levels_for_height(window_size.y);
    let mut new_level_image = |level| {
        let size = level_size(window_size, level);
        let mut image = Image::new_target_texture(size.x, size.y, GLASS_COPY_FORMAT, None);
        // the passes paint every texel, so the zeros it comes filled with would only cost RAM
        // and an upload, on every frame of a window being dragged to a new size
        image.data = None;
        // a resize makes new images instead of resizing these, so there is never an old one to
        // copy from, and an image without data that asks to copy only logs that it found none
        image.copy_on_resize = false;
        images.add(image)
    };
    for (camera, frozen_layer) in &layer_cameras {
        let descent = (1..=levels + 1).map(&mut new_level_image).collect();
        let backdrop_ascent = (1..levels).rev().map(&mut new_level_image).collect();
        let card_ascent = (1..=levels).rev().map(&mut new_level_image).collect();
        let glass_copies = GlassCopies {
            descent,
            backdrop_ascent,
            card_ascent,
        };
        commands.entity(camera).insert(glass_copies);
        // the copy the layer kept is gone with the old images. Said here and not left to the
        // passes, which would only notice while making the new one: by then whatever
        // `ScreensCovered` had stopped must already be drawing again, to be in it
        frozen_layer.copy_kept.store(false, Ordering::Relaxed);
    }
}

/// Sends a new `Floating` root to the camera of its layer, the one with the same
/// `GlobalZIndex`, and draws it as glass over that layer's blurred copy. Every other root goes
/// to the screen camera, the default UI camera. Each root gets a material of its own, since
/// each fades in on its own. A root that lets the pointer through gets none: it floats over
/// a game that goes on, which stays sharp under it, and only its windows are glass.
fn target_layer_camera(
    add: On<Add<Floating>>,
    mut commands: Commands,
    cameras: Query<(Entity, &LayerCamera, &GlassCopies)>,
    roots: Query<(&GlobalZIndex, Option<&Pickable>)>,
    reduce_transparency: Res<ReduceTransparency>,
    mut materials: ResMut<Assets<GlassMaterial>>,
) {
    // floating_root always brings one, so a root without it was not built from there
    let Ok((z_index, pickable)) = roots.get(add.entity) else {
        return;
    };
    let Some((camera, _, copies)) = cameras.iter().find(|(_, layer, _)| layer.0 == *z_index) else {
        warn!("no glass layer {z_index:?}: register it in GlassLayers");
        return;
    };
    let mut root = commands.entity(add.entity);
    root.insert(UiTargetCamera(camera));
    if lets_pointer_through(pickable) {
        return;
    }
    let Some(copy) = copies.backdrop_copy() else {
        return;
    };
    let material = GlassMaterial::backdrop(copy.clone(), reduce_transparency.0);
    root.insert(MaterialNode(materials.add(material)));
}

/// Gives each new card a glass of its own, reading its layer's card copy, which is blurrier
/// than the backdrop's so the card stands out from what is behind it. Each card gets its own
/// material, since each fades in with its own root.
fn give_card_glass(
    add: On<Add<Card>>,
    parents: Query<&ChildOf>,
    z_indexes: Query<&GlobalZIndex>,
    cameras: Query<(&LayerCamera, &GlassCopies)>,
    reduce_transparency: Res<ReduceTransparency>,
    mut materials: ResMut<Assets<GlassMaterial>>,
    mut commands: Commands,
) {
    let Ok(child_of) = parents.get(add.entity) else {
        return;
    };
    let Ok(z_index) = z_indexes.get(child_of.0) else {
        return;
    };
    let Some((_, copies)) = cameras.iter().find(|(layer, _)| layer.0 == *z_index) else {
        warn!("no glass layer {z_index:?}: register it in GlassLayers");
        return;
    };
    let Some(copy) = copies.card_copy() else {
        return;
    };
    let material = GlassMaterial::card(copy.clone(), reduce_transparency.0);
    commands
        .entity(add.entity)
        .insert(MaterialNode(materials.add(material)));
}

/// Points every open glass of a layer, each root's and its card's, at the copies its camera
/// was just given. A resize remakes the copies with new handles, and the old ones are written
/// no more, so a panel left open through it would otherwise freeze on the blur from before.
/// A root with no glass of its own, one that lets the pointer through, still has its cards.
fn retarget_open_glass(
    insert: On<Insert<GlassCopies>>,
    cameras: Query<(&LayerCamera, &GlassCopies)>,
    roots: Query<
        (
            &GlobalZIndex,
            Option<&MaterialNode<GlassMaterial>>,
            &Children,
        ),
        With<Floating>,
    >,
    cards: Query<&MaterialNode<GlassMaterial>, With<Card>>,
    mut materials: ResMut<Assets<GlassMaterial>>,
) {
    let Ok((layer_camera, copies)) = cameras.get(insert.entity) else {
        return;
    };
    let (Some(backdrop_copy), Some(card_copy)) = (copies.backdrop_copy(), copies.card_copy())
    else {
        return;
    };
    for (z_index, root_glass, children) in &roots {
        if *z_index != layer_camera.0 {
            continue;
        }
        if let Some(mut material) = root_glass.and_then(|glass| materials.get_mut(&glass.0)) {
            material.copy = backdrop_copy.clone();
        }
        for card_glass in cards.iter_many(children).flatten() {
            if let Some(mut material) = materials.get_mut(&card_glass.0) {
                material.copy = card_copy.clone();
            }
        }
    }
}

/// Redraws every open glass flat, or as glass again, when Reduce Transparency changes, so the
/// setting shows at once on the panels already open, the Settings overlay it was changed in
/// included. A card also changes its tint: flat, it is the solid surface.
fn apply_reduce_transparency(
    reduce_transparency: Res<ReduceTransparency>,
    roots: Query<&MaterialNode<GlassMaterial>, With<Floating>>,
    cards: Query<&MaterialNode<GlassMaterial>, With<Card>>,
    mut materials: ResMut<Assets<GlassMaterial>>,
) {
    let flat = reduce_transparency.0;
    for root_glass in &roots {
        if let Some(mut material) = materials.get_mut(&root_glass.0) {
            material.flat = u32::from(flat);
        }
    }
    for card_glass in &cards {
        if let Some(mut material) = materials.get_mut(&card_glass.0) {
            material.flat = u32::from(flat);
            material.tint = card_tint(flat);
        }
    }
}

/// Says the world behind the floating layers went still. For a feature's plugin to run when
/// its still moment begins, as the pause menu does on `OnEnter(Pause::On)`.
pub(crate) fn freeze_background(mut frozen_background: ResMut<FrozenBackground>) {
    frozen_background.0 = true;
}

/// Says the world behind the floating layers moves again. For a feature's plugin to run when
/// its still moment ends.
pub(crate) fn unfreeze_background(mut frozen_background: ResMut<FrozenBackground>) {
    frozen_background.0 = false;
}

/// Turns each layer camera on while a `Floating` root of its layer is open and off once none
/// is, so a closed layer runs no blur pass. In `PostUpdate`, so a root that an observer opened
/// or a state closed during `Update` already counts.
fn activate_layer_cameras(
    cameras: Query<(&LayerCamera, &mut Camera)>,
    roots: Query<&GlobalZIndex, With<Floating>>,
) {
    for (layer_camera, mut camera) in cameras {
        let layer_open = roots.iter().any(|z_index| *z_index == layer_camera.0);
        camera.is_active = layer_open;
    }
}

/// Marks each layer frozen while the background is still and the layer has settled. A copy
/// kept from before that would hold a root below half faded in, or the button that opened
/// the layer still lit, for as long as the layer stays open. A root that lets the pointer
/// through never settles: whatever is under it can still be hovered and pressed, so nothing
/// promises it stays as it was. In `PostUpdate`, so the entrances `animate_entrances` moved
/// during `Update` already count.
fn freeze_settled_layers(
    frozen_background: Res<FrozenBackground>,
    cameras: Query<(&LayerCamera, &mut FrozenLayer)>,
    roots: Query<(&GlobalZIndex, &Entrance, Option<&Pickable>), With<Floating>>,
) {
    for (layer_camera, mut frozen_layer) in cameras {
        let still_roots = roots.iter().map(|(z_index, entrance, pickable)| {
            let still = entrance.is_finished() && !lets_pointer_through(pickable);
            (*z_index, still)
        });
        frozen_layer.frozen = frozen_background.0 && is_settled(layer_camera.0, still_roots);
    }
}

/// Says whether the screens are wholly hidden behind a layer that shows a blur it already
/// made. After the two systems above, whose answers it reads, and the last of
/// `ScreensCoveredSystems`, which others order themselves by.
fn update_screens_covered(
    reduce_transparency: Res<ReduceTransparency>,
    mut cameras: Query<(&Camera, &mut FrozenLayer), With<LayerCamera>>,
    mut screens_covered: ResMut<ScreensCovered>,
) {
    let layers = cameras.iter().map(|(camera, frozen_layer)| {
        let copy_kept = frozen_layer.copy_kept.load(Ordering::Relaxed);
        (camera.is_active, frozen_layer.frozen, copy_kept)
    });
    let covered = are_screens_covered(reduce_transparency.0, layers);
    if screens_covered.0 && !covered {
        // what was stopped under the layers only starts drawing again this frame, and may not
        // be whole yet (its shadows, say): no layer keeps the copy it makes now. Next frame
        // `freeze_settled_layers` freezes them again, and the copy kept is of a whole frame
        for (_, mut frozen_layer) in &mut cameras {
            frozen_layer.frozen = false;
        }
    }
    // set_if_neq: a camera that follows this is touched only when the answer changes
    screens_covered.set_if_neq(ScreensCovered(covered));
}

/// Queues the two Kawase pipelines once, at `RenderStartup`, so they compile while the game
/// loads instead of on the first pause, where the wait would show as a hitch.
fn init_kawase_pipelines(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "kawase_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<f32>(false),
            ),
        ),
    );
    // the default filters read the nearest texel only, which would cost the Kawase its trick:
    // a read between four texels averaging all four
    let sampler = render_device.create_sampler(&SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });
    let down_shader = asset_server.load(KAWASE_DOWN_SHADER_PATH);
    let up_shader = asset_server.load(KAWASE_UP_SHADER_PATH);
    let vertex = fullscreen_shader.to_vertex_state();
    let down = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("kawase_down_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: vertex.clone(),
        fragment: Some(FragmentState {
            shader: down_shader,
            targets: vec![Some(ColorTargetState {
                format: GLASS_COPY_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });
    let up = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("kawase_up_pipeline".into()),
        layout: vec![layout.clone()],
        vertex,
        fragment: Some(FragmentState {
            shader: up_shader,
            targets: vec![Some(ColorTargetState {
                format: GLASS_COPY_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });
    let offset = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("kawase_offset_buffer"),
        contents: &KAWASE_OFFSET_TEXELS.to_le_bytes(),
        usage: BufferUsages::UNIFORM,
    });
    commands.insert_resource(KawasePipelines {
        layout,
        sampler,
        down,
        up,
        offset,
    });
}

/// Blurs what the cameras below a layer drew into its `GlassCopies`: a dual Kawase that halves
/// the image down the descent and doubles it back up twice, the backdrop's ascent from the
/// level above the deepest and the card's from the deepest, before the layer's own UI. The
/// card's chain goes one level further down, so a card reads blurrier than the backdrop.
///
/// A frozen layer is blurred once: `kept_copies` remembers, for each layer camera, the copy it
/// last blurred while frozen, and the passes are skipped while that copy is still the one in
/// use. It is written only after every pass ran, so a frame that could not blur (a pipeline
/// still compiling, an image not on the GPU yet) is tried again, and a resize, which hands
/// the layer new images, is blurred again without being told.
///
/// Under Reduce Transparency no pass runs: the glass is drawn flat and reads no copy.
fn blur_layer_backdrops(
    camera: ViewQuery<(&ViewTarget, &GlassCopies, &FrozenLayer)>,
    reduce_transparency: Res<ReduceTransparency>,
    pipeline_cache: Res<PipelineCache>,
    kawase: Res<KawasePipelines>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut ctx: RenderContext,
    render_device: Res<RenderDevice>,
    mut kept_copies: Local<EntityHashMap<AssetId<Image>>>,
) {
    let camera_entity = camera.entity();
    let (view_target, copies, frozen_layer) = camera.into_inner();
    let copy_id = copies.backdrop_copy().map(Handle::id);
    // whether the layer shows, this frame, a copy finished in an earlier one
    let copy_kept = !reduce_transparency.0
        && frozen_layer.frozen
        && copy_id.is_some()
        && kept_copies.get(&camera_entity) == copy_id.as_ref();
    // told to the main world, which stops drawing what such a layer hides
    frozen_layer.copy_kept.store(copy_kept, Ordering::Relaxed);
    if copy_kept {
        return;
    }
    if reduce_transparency.0 || !frozen_layer.frozen {
        // the copy a frozen layer kept grows old while nothing blurs it again: forgetting it
        // makes the layer blur anew when it freezes, or the setting is turned back off
        kept_copies.remove(&camera_entity);
    }
    if reduce_transparency.0 {
        return;
    }
    let Some(copy_id) = copy_id else {
        return;
    };
    let Some(down_pipeline) = pipeline_cache.get_render_pipeline(kawase.down) else {
        return;
    };
    let Some(up_pipeline) = pipeline_cache.get_render_pipeline(kawase.up) else {
        return;
    };
    // the deepest level is the card's, so the backdrop's ascent starts from the one above it
    let Some((card_level, backdrop_levels)) = copies.descent.split_last() else {
        return;
    };
    let mut source = view_target.main_texture_view();
    let mut run_pass = |pipeline: &RenderPipeline,
                        read: &TextureView,
                        write: &TextureView,
                        label: &'static str| {
        let bind_group = render_device.create_bind_group(
            "kawase_layer_bind_group",
            &pipeline_cache.get_bind_group_layout(&kawase.layout),
            &BindGroupEntries::sequential((
                read,
                &kawase.sampler,
                kawase.offset.as_entire_binding(),
            )),
        );
        let pass_descriptor = RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: write,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        };

        let diagnostics = ctx.diagnostic_recorder();
        let diagnostics = diagnostics.as_deref();
        let mut render_pass = ctx.command_encoder().begin_render_pass(&pass_descriptor);
        let pass_span = diagnostics.pass_span(&mut render_pass, label);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        pass_span.end(&mut render_pass);
    };
    for handle in backdrop_levels {
        let Some(texture) = gpu_images.get(handle) else {
            return;
        };
        run_pass(
            down_pipeline,
            source,
            &texture.texture_view,
            "kawase_down_pass",
        );
        source = &texture.texture_view;
    }
    let backdrop_bottom = source;
    let Some(card_texture) = gpu_images.get(card_level) else {
        return;
    };
    run_pass(
        down_pipeline,
        backdrop_bottom,
        &card_texture.texture_view,
        "kawase_down_pass",
    );
    for handle in &copies.backdrop_ascent {
        let Some(texture) = gpu_images.get(handle) else {
            return;
        };
        run_pass(up_pipeline, source, &texture.texture_view, "kawase_up_pass");
        source = &texture.texture_view;
    }
    let mut card_source = &card_texture.texture_view;
    for handle in &copies.card_ascent {
        let Some(texture) = gpu_images.get(handle) else {
            return;
        };
        run_pass(
            up_pipeline,
            card_source,
            &texture.texture_view,
            "kawase_up_pass",
        );
        card_source = &texture.texture_view;
    }
    if frozen_layer.frozen {
        kept_copies.insert(camera_entity, copy_id);
    }
}

// ==== Helpers ================================================================

/// Returns how many halvings bring a screen this tall to `GLASS_TARGET_HEIGHT_PX` or below,
/// each one a level of the descent. Never less than one: a window already under the target
/// still gets a level, since none would leave the glass clear.
fn levels_for_height(screen_height_px: u32) -> u32 {
    let mut current_height = screen_height_px;
    let mut levels = 0;
    while current_height > GLASS_TARGET_HEIGHT_PX {
        current_height /= 2;
        levels += 1;
    }
    levels.max(1)
}

/// Returns the size of a level's image: the screen halved on both sides once per level. Never
/// under one pixel on a side, since the GPU refuses a texture with a side of zero.
fn level_size(screen_px: UVec2, level: u32) -> UVec2 {
    (screen_px / (1 << level)).max(UVec2::ONE)
}

/// Returns the tint of a card's glass. As glass, the surface laid over the backdrop's dim,
/// which the card's own copy does not carry. Flat, the surface made opaque: a solid fill, with
/// nothing of the screen behind it showing through.
fn card_tint(flat: bool) -> LinearRgba {
    if flat {
        SURFACE.with_alpha(1.0).into()
    } else {
        lay_over(SURFACE.into(), SCREEN_DIM.into())
    }
}

/// Returns the one color that looks like `top` laid over `bottom`, so the glass can draw two
/// tints with the single one its shader mixes in. Fully clear when both are, since the color
/// of nothing has no answer and would divide by zero.
fn lay_over(top: LinearRgba, bottom: LinearRgba) -> LinearRgba {
    // how much of `bottom` still shows through `top`
    let bottom_share = (1.0 - top.alpha) * bottom.alpha;
    let out_alpha = top.alpha + bottom_share;
    if out_alpha == 0.0 {
        return LinearRgba::NONE;
    }
    // each channel weighs both colors by how much of each shows, as a share of the whole
    let channel =
        |top_c: f32, bottom_c: f32| (top_c * top.alpha + bottom_c * bottom_share) / out_alpha;
    LinearRgba::new(
        channel(top.red, bottom.red),
        channel(top.green, bottom.green),
        channel(top.blue, bottom.blue),
        out_alpha,
    )
}

/// Tells whether a layer has settled: every root at or below it, each given as its layer and
/// whether it has stopped changing, is still. A root above it is drawn over the layer and
/// never reaches its copy, so it does not hold the layer back.
fn is_settled(layer: GlobalZIndex, roots: impl IntoIterator<Item = (GlobalZIndex, bool)>) -> bool {
    roots
        .into_iter()
        .filter(|(root_layer, _)| root_layer.0 <= layer.0)
        .all(|(_, still)| still)
}

/// Tells whether a root lets the pointer through to what is under it, as one that floats over
/// a game that goes on does: such a root has no backdrop, and never lets its layer freeze.
/// A node with no `Pickable` blocks the pointer, as Bevy's picking reads it.
fn lets_pointer_through(pickable: Option<&Pickable>) -> bool {
    pickable.is_some_and(|pickable| !pickable.should_block_lower)
}

/// Tells whether the screens are wholly hidden, from each layer given as whether it is open,
/// whether it is frozen and whether it shows a copy it kept. One such layer is enough: its
/// backdrop covers the window with that copy, so nothing under it shows. Flat glass covers
/// nothing: its backdrop is a dim the screens show through.
fn are_screens_covered(flat: bool, layers: impl IntoIterator<Item = (bool, bool, bool)>) -> bool {
    !flat
        && layers
            .into_iter()
            .any(|(open, frozen, copy_kept)| open && frozen && copy_kept)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // the common window heights, from 720p to 4K
    const SCREEN_HEIGHTS_PX: [u32; 4] = [720, 1080, 1440, 2160];
    const TOLERANCE: f32 = 1e-5;
    // three layers stacked, the middle one being the layer under test
    const LOWER_LAYER: GlobalZIndex = GlobalZIndex(50);
    const LAYER: GlobalZIndex = GlobalZIndex(100);
    const UPPER_LAYER: GlobalZIndex = GlobalZIndex(200);

    #[test]
    fn deepest_level_lands_within_a_halving_of_the_target() {
        for height in SCREEN_HEIGHTS_PX {
            let deepest = height >> levels_for_height(height);
            assert!(
                GLASS_TARGET_HEIGHT_PX / 2 < deepest && deepest <= GLASS_TARGET_HEIGHT_PX,
                "{height} px tall ends at {deepest} px",
            );
        }
    }

    #[test]
    fn a_4k_screen_takes_one_level_more_than_1080p() {
        assert_eq!(levels_for_height(2160), levels_for_height(1080) + 1);
    }

    #[test]
    fn a_window_under_the_target_still_takes_a_level() {
        assert_eq!(levels_for_height(GLASS_TARGET_HEIGHT_PX / 2), 1);
        assert_eq!(levels_for_height(0), 1);
    }

    #[test]
    fn each_level_halves_both_sides_of_the_screen() {
        assert_eq!(level_size(UVec2::new(1920, 1080), 2), UVec2::new(480, 270));
    }

    #[test]
    fn a_thin_window_never_gets_a_zero_sized_level() {
        // two halvings take 3 px to 0, a texture the GPU refuses
        assert_eq!(level_size(UVec2::new(1920, 3), 2).y, 1);
    }

    #[test]
    fn a_flat_card_hides_everything_behind_it() {
        assert!((card_tint(true).alpha - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn a_glass_card_lets_some_of_the_blur_through() {
        assert!(card_tint(false).alpha < 1.0 - TOLERANCE);
    }

    #[test]
    fn one_tint_looks_like_the_two_laid_in_turn() {
        let (top, bottom): (LinearRgba, LinearRgba) = (SURFACE.into(), SCREEN_DIM.into());
        let out = lay_over(top, bottom);
        // what the `mix` in glass.wesl does to one channel of the blur behind the glass
        let tinted = |behind: f32, tint: f32, alpha: f32| behind + (tint - behind) * alpha;
        let channels = [
            (top.red, bottom.red, out.red),
            (top.green, bottom.green, out.green),
            (top.blue, bottom.blue, out.blue),
        ];
        for behind in [0.0, 0.5, 1.0] {
            for (top_c, bottom_c, out_c) in channels {
                let in_turn = tinted(tinted(behind, bottom_c, bottom.alpha), top_c, top.alpha);
                let at_once = tinted(behind, out_c, out.alpha);
                assert!(
                    (in_turn - at_once).abs() < TOLERANCE,
                    "over {behind}: {in_turn} in turn, {at_once} at once",
                );
            }
        }
    }

    #[test]
    fn an_opaque_top_hides_the_bottom() {
        let out = lay_over(LinearRgba::RED, LinearRgba::BLUE);
        assert!((out.red - 1.0).abs() < TOLERANCE);
        assert!(out.blue.abs() < TOLERANCE);
        assert!((out.alpha - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn two_clear_colors_lay_over_as_nothing() {
        let out = lay_over(LinearRgba::NONE, LinearRgba::NONE);
        // exact on purpose, where a float usually gets a tolerance: the guard hands back the
        // constant itself, and the NaN a division by zero would leave equals nothing
        assert_eq!(out, LinearRgba::NONE);
    }

    #[test]
    fn a_root_still_coming_in_below_holds_the_layer() {
        let roots = [(LOWER_LAYER, false), (LAYER, true)];
        assert!(!is_settled(LAYER, roots));
    }

    #[test]
    fn a_layer_waits_for_its_own_root() {
        assert!(!is_settled(LAYER, [(LAYER, false)]));
    }

    #[test]
    fn a_root_coming_in_above_does_not_hold_the_layer() {
        let roots = [(LOWER_LAYER, true), (LAYER, true), (UPPER_LAYER, false)];
        assert!(is_settled(LAYER, roots));
    }

    #[test]
    fn a_root_that_ignores_the_pointer_lets_it_through() {
        assert!(lets_pointer_through(Some(&Pickable::IGNORE)));
    }

    #[test]
    fn a_root_with_no_pickable_blocks_the_pointer() {
        assert!(!lets_pointer_through(None));
    }

    #[test]
    fn a_root_that_is_pickable_blocks_the_pointer() {
        assert!(!lets_pointer_through(Some(&Pickable::default())));
    }

    // a layer as `are_screens_covered` takes it: open, frozen, showing a copy it kept
    const COVERING_LAYER: (bool, bool, bool) = (true, true, true);

    #[test]
    fn an_open_frozen_layer_showing_its_kept_copy_covers_the_screens() {
        assert!(are_screens_covered(false, [COVERING_LAYER]));
    }

    #[test]
    fn a_layer_still_making_its_copy_covers_nothing() {
        assert!(!are_screens_covered(false, [(true, true, false)]));
    }

    #[test]
    fn a_layer_over_a_moving_background_covers_nothing() {
        assert!(!are_screens_covered(false, [(true, false, true)]));
    }

    #[test]
    fn a_closed_layer_covers_nothing() {
        // closed, its marks are left over from the last time it was open
        assert!(!are_screens_covered(false, [(false, true, true)]));
    }

    #[test]
    fn flat_glass_covers_nothing() {
        assert!(!are_screens_covered(true, [COVERING_LAYER]));
    }

    #[test]
    fn one_covering_layer_is_enough_among_others() {
        let layers = [(true, false, false), COVERING_LAYER, (false, false, false)];
        assert!(are_screens_covered(false, layers));
    }
}
