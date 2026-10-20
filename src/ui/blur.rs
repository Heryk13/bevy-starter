use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::Core2d;
use bevy::core_pipeline::fullscreen_material::{
    FullscreenMaterial, FullscreenMaterialPlugin, fullscreen_material_system,
};
use bevy::core_pipeline::upscaling::upscaling;
use bevy::ecs::schedule::{ScheduleConfigs, ScheduleLabel};
use bevy::ecs::system::BoxedSystem;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;
use bevy::ui_render::ui_pass;

use crate::fade::approach;

// ==== Config =================================================================

// above the screen camera's default order of 0, so it draws last
const FLOATING_CAMERA_ORDER: isize = 1;
const BLUR_SHADER_PATH: &str = "shaders/blur.wesl";
// the Gaussian's standard deviation at full strength: wide enough that text behind a panel
// cannot be read, so it never competes with the panel's own
const BLUR_SIGMA_PX: f32 = 9.0;
// how long the blur takes to come in when something floats, and to leave when nothing does
const BLUR_RAMP_SECS: f32 = 0.18;
const BLUR_EASING: EaseFunction = EaseFunction::SmoothStep;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        FullscreenMaterialPlugin::<BlurHorizontal>::default(),
        FullscreenMaterialPlugin::<BlurVertical>::default(),
    ));
    app.add_systems(Startup, spawn_floating_camera);
    app.add_observer(target_floating_camera);
    app.add_systems(Update, update_blur);
}

// ==== Components =============================================================

/// Marks a UI root that floats over the screens: an overlay, the pause menu, a dialog. It is
/// drawn by the floating camera, and while any is alive the screens behind are blurred.
#[derive(Component)]
pub(crate) struct Floating;

/// Marks the camera that draws `Floating` roots, above the screen camera and its blur.
#[derive(Component)]
struct FloatingCamera;

/// How blurred the screen camera's image is, from 0.0 to 1.0, before easing. It goes on the
/// screen camera, the one that is `IsDefaultUiCamera`.
#[derive(Component, Default)]
pub(crate) struct BlurStrength(f32);

/// The blur's pass across the screen.
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
#[extract_app(RenderApp)]
struct BlurHorizontal {
    direction: Vec2,
    sigma_px: f32,
}

/// The blur's pass down the screen, run on what the horizontal pass wrote.
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
#[extract_app(RenderApp)]
struct BlurVertical {
    direction: Vec2,
    sigma_px: f32,
}

// both passes run after the UI is drawn, so they blur it too, and before the image reaches
// the window; Bevy's own post-processing runs before the UI, which is why neither is in it
impl FullscreenMaterial for BlurHorizontal {
    fn fragment_shader() -> ShaderRef {
        BLUR_SHADER_PATH.into()
    }

    fn schedule() -> impl ScheduleLabel + Clone {
        Core2d
    }

    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system.after(ui_pass).before(upscaling)
    }
}

impl FullscreenMaterial for BlurVertical {
    fn fragment_shader() -> ShaderRef {
        BLUR_SHADER_PATH.into()
    }

    fn schedule() -> impl ScheduleLabel + Clone {
        Core2d
    }

    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system
            .after(fullscreen_material_system::<BlurHorizontal>)
            .before(upscaling)
    }
}

// ==== Systems ================================================================

/// Spawns the camera that draws everything `Floating`. It never clears: it draws over the
/// screen camera's image, blurred or not, which it would otherwise wipe.
fn spawn_floating_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: FLOATING_CAMERA_ORDER,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        // a Camera2d draws every sprite and tilemap in the world, and this one would draw
        // them again, sharp, over the blurred image. UI ignores render layers, so the
        // floating panels still reach it through UiTargetCamera
        RenderLayers::none(),
        FloatingCamera,
    ));
}

/// Sends a new `Floating` root to the floating camera. Every other root goes to the screen
/// camera, the default UI camera.
fn target_floating_camera(
    add: On<Add<Floating>>,
    mut commands: Commands,
    camera: Single<Entity, With<FloatingCamera>>,
) {
    commands.entity(add.entity).insert(UiTargetCamera(*camera));
}

/// Moves the blur toward full while anything floats and toward none once nothing does,
/// counting real time, since virtual time is frozen under the pause menu. At none, both
/// passes are removed, so a frame with nothing floating costs nothing.
fn update_blur(
    mut commands: Commands,
    time: Res<Time<Real>>,
    floating: Query<(), With<Floating>>,
    camera: Single<(Entity, &mut BlurStrength, Has<BlurHorizontal>)>,
) {
    let (entity, mut strength, blurring) = camera.into_inner();
    let target = if floating.is_empty() { 0.0 } else { 1.0 };
    strength.0 = approach(strength.0, target, time.delta_secs() / BLUR_RAMP_SECS);
    if strength.0 > 0.0 {
        let sigma_px = BLUR_SIGMA_PX * BLUR_EASING.sample_clamped(strength.0);
        commands.entity(entity).insert((
            BlurHorizontal {
                direction: Vec2::X,
                sigma_px,
            },
            BlurVertical {
                direction: Vec2::Y,
                sigma_px,
            },
        ));
    } else if blurring {
        commands
            .entity(entity)
            .remove::<(BlurHorizontal, BlurVertical)>();
    }
}
