use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::{Core2d, Core2dSystems, FullscreenShader};
use bevy::prelude::*;
use bevy::render::diagnostic::RecordDiagnostics;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
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
use bevy::ui_render::ui_pass;
use bevy::window::{PrimaryWindow, WindowResized};

use super::panel::SCREEN_DIM;

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

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        ExtractComponentPlugin::<GlassCopies>::default(),
        UiMaterialPlugin::<GlassMaterial>::default(),
    ));
    app.add_systems(
        Startup,
        (spawn_layer_cameras, generate_level_images).chain(),
    );
    app.add_observer(target_layer_camera);
    app.add_observer(retarget_open_glass);
    app.add_systems(
        Update,
        generate_level_images.run_if(on_message::<WindowResized>),
    );
    app.add_systems(PostUpdate, activate_layer_cameras);
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

// ==== Components =============================================================

/// Marks a UI root that floats over the screens: an overlay, the pause menu, a dialog. It is
/// drawn by the camera of its layer, and while any is alive the screens behind are blurred.
#[derive(Component)]
pub(crate) struct Floating;

/// Marks the camera that draws the `Floating` roots of one layer, the one named by this
/// `GlobalZIndex`, above the screen camera and every lower layer. The render world tells the
/// layer cameras apart by their `GlassCopies`, which no other camera has.
#[derive(Component)]
struct LayerCamera(GlobalZIndex);

/// The images a layer camera blurs into, made for the window's size: the descent, each half
/// the one before it, from half the screen down to one level past the backdrop's (the card's),
/// and the ascent, back up toward half the screen, where the glass reads it. Extracted with
/// its camera, so the passes find the images to write in the same query as the view.
#[derive(Component, ExtractComponent, Clone)]
#[extract_app(RenderApp)]
struct GlassCopies {
    descent: Vec<Handle<Image>>,
    ascent: Vec<Handle<Image>>,
}

impl GlassCopies {
    /// Returns the image the glass reads, where the backdrop's blur ends: the top of the
    /// ascent, or the first level when a window so small it takes a single level leaves the
    /// ascent empty.
    fn glass_copy(&self) -> Option<&Handle<Image>> {
        self.ascent.last().or_else(|| self.descent.first())
    }
}

// ==== Resources ==============================================================

/// The floating layers, each named by the `GlobalZIndex` of its roots. Every feature that
/// floats registers its own from its plugin, so this file never names them, and each layer
/// gets a camera of its own, drawn over the layers below it.
#[derive(Resource, Default)]
pub(crate) struct GlassLayers(pub(crate) Vec<GlobalZIndex>);

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

/// The glass a floating root is drawn with: its layer's blurred copy, read where the node sits
/// on screen, under a tint, all faded by the node's alpha.
#[derive(AsBindGroup, Asset, TypePath, Clone)]
pub(super) struct GlassMaterial {
    #[uniform(0)]
    tint: LinearRgba,
    #[uniform(0)]
    pub(super) alpha: f32,
    #[texture(1)]
    #[sampler(2)]
    copy: Handle<Image>,
}

impl UiMaterial for GlassMaterial {
    fn fragment_shader() -> ShaderRef {
        GLASS_SHADER_PATH.into()
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
        ));
    }
}

/// Gives every layer camera `GlassCopies` of its own, sized for the window in physical pixels:
/// each layer blurs a different image, so sharing them would let one wipe another's. Runs again
/// whenever the window resizes; the copies it replaces free their images once dropped.
fn generate_level_images(
    window: Single<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    layer_cameras: Query<Entity, With<LayerCamera>>,
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
    for camera in &layer_cameras {
        let descent = (1..=levels + 1).map(&mut new_level_image).collect();
        let ascent = (1..levels).rev().map(&mut new_level_image).collect();
        let glass_copies = GlassCopies { descent, ascent };
        commands.entity(camera).insert(glass_copies);
    }
}

/// Sends a new `Floating` root to the camera of its layer, the one with the same
/// `GlobalZIndex`, and draws it as glass over that layer's blurred copy. Every other root goes
/// to the screen camera, the default UI camera. Each root gets a material of its own, since
/// each fades in on its own.
fn target_layer_camera(
    add: On<Add<Floating>>,
    mut commands: Commands,
    cameras: Query<(Entity, &LayerCamera, &GlassCopies)>,
    z_indexes: Query<&GlobalZIndex>,
    mut materials: ResMut<Assets<GlassMaterial>>,
) {
    // floating_root always brings one, so a root without it was not built from there
    let Ok(z_index) = z_indexes.get(add.entity) else {
        return;
    };
    let Some((camera, _, copies)) = cameras.iter().find(|(_, layer, _)| layer.0 == *z_index) else {
        warn!("no glass layer {z_index:?}: register it in GlassLayers");
        return;
    };
    let Some(copy) = copies.glass_copy() else {
        return;
    };
    let handle = materials.add(GlassMaterial {
        tint: SCREEN_DIM.into(),
        alpha: 0.0,
        copy: copy.clone(),
    });
    commands
        .entity(add.entity)
        .insert((UiTargetCamera(camera), MaterialNode(handle)));
}

/// Points every open glass of a layer at the copies its camera was just given. A resize
/// remakes the copies with new handles, and the old ones are written no more, so a panel left
/// open through it would otherwise freeze on the blur from before.
fn retarget_open_glass(
    insert: On<Insert<GlassCopies>>,
    cameras: Query<(&LayerCamera, &GlassCopies)>,
    roots: Query<(&GlobalZIndex, &MaterialNode<GlassMaterial>), With<Floating>>,
    mut materials: ResMut<Assets<GlassMaterial>>,
) {
    let Ok((layer_camera, copies)) = cameras.get(insert.entity) else {
        return;
    };
    let Some(copy) = copies.glass_copy() else {
        return;
    };
    for (z_index, node) in &roots {
        if *z_index == layer_camera.0
            && let Some(mut material) = materials.get_mut(&node.0)
        {
            material.copy = copy.clone();
        }
    }
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
/// the image down the descent and doubles it back up the ascent, before the layer's own UI.
fn blur_layer_backdrops(
    camera: ViewQuery<(&ViewTarget, &GlassCopies)>,
    pipeline_cache: Res<PipelineCache>,
    kawase: Res<KawasePipelines>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut ctx: RenderContext,
    render_device: Res<RenderDevice>,
) {
    let (view_target, copies) = camera.into_inner();
    let Some(down_pipeline) = pipeline_cache.get_render_pipeline(kawase.down) else {
        return;
    };
    let Some(up_pipeline) = pipeline_cache.get_render_pipeline(kawase.up) else {
        return;
    };
    // the deepest level is the card's, so the backdrop's ascent starts from the one above it
    let Some((_, backdrop_levels)) = copies.descent.split_last() else {
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
    for handle in &copies.ascent {
        let Some(texture) = gpu_images.get(handle) else {
            return;
        };
        run_pass(up_pipeline, source, &texture.texture_view, "kawase_up_pass");
        source = &texture.texture_view;
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

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // the common window heights, from 720p to 4K
    const SCREEN_HEIGHTS_PX: [u32; 4] = [720, 1080, 1440, 2160];

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
}
