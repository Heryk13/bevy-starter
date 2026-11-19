use bevy::core_pipeline::blit::{BlitPipeline, BlitPipelineKey};
use bevy::core_pipeline::{Core2d, Core2dSystems};
use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::gpu_readback::{ReadbackComplete, ReadbackOnce};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    CachedRenderPipelineId, Extent3d, Operations, PipelineCache, RenderPassColorAttachment,
    RenderPassDescriptor, SpecializedRenderPipelines, TextureFormat, TextureUsages,
};
use bevy::render::renderer::{RenderContext, ViewQuery};
use bevy::render::texture::GpuImage;
use bevy::render::view::{ResolvedCompositingSpace, ViewTarget};
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::ui_render::ui_pass;
use bevy::window::PrimaryWindow;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::thread::JoinHandle;

use super::RecordingState;
use super::bar::RecordingChoices;
use super::encoder::{VideoFormat, VideoFrame, part_path, start_encoder};
#[cfg(windows)]
use super::sound::{SoundCapture, start_sound};
use crate::dev_tools::{DEV_TOOLS_Z_INDEX, file_stamp, gallery_dir, numbered_file_name};
use crate::save::unix_now;
use crate::ui::blur::LayerCamera;

// ==== Config =================================================================

// the frame's format once copied: 8 bits a channel, as H.264 is fed, written in sRGB so the
// bytes are the colours the screen shows, whatever space the game draws in
const FRAME_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
const VIDEO_EXTENSION: &str = "mp4";

// ==== Plugin =================================================================

/// The recording's frames: the dev tools' layer's camera copies, before it draws its own
/// roots, what the game drew, into an image read back to the CPU at each frame of the video,
/// and the encoder writes them. Starting and stopping the recording are here too, as the
/// state enters and leaves `On`, and on `AppExit`, so a game closed mid-recording still
/// leaves its file whole.
pub(super) fn plugin(app: &mut App) {
    app.add_plugins(ExtractComponentPlugin::<RecordingTarget>::default());
    app.init_resource::<FinishingRecordings>();
    app.add_systems(OnEnter(RecordingState::On), start_capture);
    app.add_systems(
        Update,
        read_back_due_frames.run_if(resource_exists::<Recorder>),
    );
    app.add_systems(OnExit(RecordingState::On), stop_capture);
    app.add_systems(
        Last,
        finish_recordings_on_exit.run_if(on_message::<AppExit>),
    );
    // an app built without rendering has no render world, so nothing to record in it
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app.add_systems(
        Render,
        // as Bevy's own upscaling does: the pipeline cache is shared through interior
        // mutability, so there is nothing to order against
        prepare_frame_pipeline
            .in_set(RenderSystems::Prepare)
            .ambiguous_with_all(),
    );
    // the bounds the glass's blur has, in `blur.rs`: after whatever may swap the main image,
    // before the layer's own UI is drawn, so the frame is the game without the dev UI
    render_app.add_systems(
        Core2d,
        copy_frame.before(ui_pass).after(Core2dSystems::PostProcess),
    );
}

// ==== Components =============================================================

/// Put on the dev tools' layer's camera while recording: the image its frame is copied
/// into, every frame, before it draws the dev UI.
#[derive(Component, ExtractComponent, Clone)]
#[extract_app(RenderApp)]
struct RecordingTarget(Handle<Image>);

/// The blit that copies a recorded view's frame, made for its colour space.
#[derive(Component)]
struct FramePipeline(CachedRenderPipelineId);

/// Marks a frame's readback, despawned once it arrives or the recording ends.
#[derive(Component)]
struct FrameReadback;

// ==== Resources ==============================================================

/// The recording going on: when it began, its next frame, the image the frames are copied
/// into, and the threads writing it.
#[derive(Resource)]
struct Recorder {
    started_secs: f64,
    frame_rate: u32,
    next_slot: u64,
    image: Handle<Image>,
    frames: SyncSender<VideoFrame>,
    dropped_frames: u64,
    encoder: JoinHandle<()>,
    #[cfg(windows)]
    sound: Option<SoundCapture>,
}

/// The threads still writing recordings that have stopped, joined as the game closes so no
/// file is left half written.
#[derive(Resource, Default)]
struct FinishingRecordings(Vec<JoinHandle<()>>);

// ==== Systems ================================================================

/// Starts recording: an image of the window's size for the frames, the dev tools' layer's
/// camera copying into it, the sound if the bar asked for it, and the encoder.
fn start_capture(
    mut commands: Commands,
    time: Res<Time<Real>>,
    choices: Res<RecordingChoices>,
    window: Single<&Window, With<PrimaryWindow>>,
    cameras: Query<(Entity, &LayerCamera)>,
    mut images: ResMut<Assets<Image>>,
    mut next_state: ResMut<NextState<RecordingState>>,
) {
    let Some(camera) = cameras
        .iter()
        .find_map(|(entity, layer)| (layer.0 == DEV_TOOLS_Z_INDEX).then_some(entity))
    else {
        error!("no dev tools' camera to record from");
        next_state.set_if_different(RecordingState::Off);
        return;
    };
    let Some(path) = gallery_dir().and_then(|gallery| free_video_path(&gallery)) else {
        error!("no data folder to keep recordings in");
        next_state.set_if_different(RecordingState::Off);
        return;
    };
    let size = even_size(window.physical_size());
    let mut image = Image::new_target_texture(size.x, size.y, FRAME_FORMAT, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    image.texture_descriptor.size = Extent3d {
        width: size.x,
        height: size.y,
        depth_or_array_layers: 1,
    };
    let image = images.add(image);
    commands
        .entity(camera)
        .insert(RecordingTarget(image.clone()));

    #[cfg(windows)]
    let (sound, packets) = match choices.sound.then(start_sound).flatten() {
        Some((sound, packets)) => (Some(sound), Some(packets)),
        None => (None, None),
    };
    #[cfg(not(windows))]
    let packets = None;
    let format = VideoFormat {
        size,
        frame_rate: choices.frame_rate,
        sound: packets,
    };
    let (frames, encoder) = start_encoder(path, format);
    commands.insert_resource(Recorder {
        started_secs: time.elapsed_secs_f64(),
        frame_rate: choices.frame_rate,
        next_slot: 0,
        image,
        frames,
        dropped_frames: 0,
        encoder,
        #[cfg(windows)]
        sound,
    });
}

/// Reads back the frame each time the real clock passes the video's next frame, whatever
/// the game's own frame rate: slow motion slows the game and not the clock, so the video
/// comes out slowed. A frame the game was too slow to give is skipped, not sent twice.
fn read_back_due_frames(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut recorder: ResMut<Recorder>,
) {
    let elapsed_secs = time.elapsed_secs_f64() - recorder.started_secs;
    let Some(slot) = due_slot(elapsed_secs, recorder.frame_rate, recorder.next_slot) else {
        return;
    };
    recorder.next_slot = slot + 1;
    let pts_secs = slot as f64 / f64::from(recorder.frame_rate);
    commands
        .spawn((
            ReadbackOnce::texture(recorder.image.clone()),
            FrameReadback,
            DespawnOnExit(RecordingState::On),
        ))
        .observe(
            move |mut readback: On<ReadbackComplete>,
                  mut commands: Commands,
                  recorder: Option<ResMut<Recorder>>| {
                commands.entity(readback.entity).despawn();
                let Some(mut recorder) = recorder else {
                    return;
                };
                let frame = VideoFrame {
                    pts_secs,
                    // taken out of the event, not cloned: it is every pixel of the window
                    padded_rgba: std::mem::take(&mut readback.event_mut().data),
                };
                if let Err(TrySendError::Full(_)) = recorder.frames.try_send(frame) {
                    recorder.dropped_frames += 1;
                }
            },
        );
}

/// Stops recording as the state leaves `On`: the camera stops copying, the sound stops, and
/// the encoder, once its sender is dropped here, finishes the file on its own thread.
fn stop_capture(mut commands: Commands, cameras: Query<Entity, With<RecordingTarget>>) {
    for camera in &cameras {
        commands.entity(camera).remove::<RecordingTarget>();
    }
    // taken out whole, not borrowed: its threads move on to `FinishingRecordings`
    commands.queue(|world: &mut World| {
        if let Some(recorder) = world.remove_resource::<Recorder>() {
            let threads = end(recorder);
            world
                .resource_mut::<FinishingRecordings>()
                .0
                .extend(threads);
        }
    });
}

/// Finishes the recording going on and waits for every file still being written, as the
/// game closes: the state never leaves `On` then, so `stop_capture` does not run.
fn finish_recordings_on_exit(world: &mut World) {
    let mut threads = world
        .remove_resource::<Recorder>()
        .map(end)
        .unwrap_or_default();
    threads.append(&mut world.resource_mut::<FinishingRecordings>().0);
    for thread in threads {
        let _ = thread.join();
    }
}

// ---- in the render world ----------------------------------------------------

/// Makes, for each recorded view, the blit that copies its frame into an 8-bit sRGB image,
/// from the view's own colour space, as Bevy's upscaling does for the window.
fn prepare_frame_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    blit: Res<BlitPipeline>,
    views: Query<(Entity, Option<&ResolvedCompositingSpace>), With<RecordingTarget>>,
) {
    for (view, space) in &views {
        let key = BlitPipelineKey {
            target_format: FRAME_FORMAT,
            blend_state: None,
            samples: 1,
            source_space: ResolvedCompositingSpace::space(space),
        };
        let pipeline = pipelines.specialize(&pipeline_cache, &blit, key);
        commands.entity(view).insert(FramePipeline(pipeline));
    }
}

/// Copies what the game drew into the recording's image, scaled to its size, before the
/// dev tools' layer draws its roots over it.
fn copy_frame(
    view: ViewQuery<(&ViewTarget, &RecordingTarget, &FramePipeline)>,
    pipeline_cache: Res<PipelineCache>,
    blit: Res<BlitPipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut ctx: RenderContext,
) {
    let (view_target, target, pipeline) = view.into_inner();
    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline.0) else {
        return;
    };
    let Some(image) = gpu_images.get(&target.0) else {
        return;
    };
    let bind_group = blit.create_bind_group(
        ctx.render_device(),
        view_target.main_texture_view(),
        &pipeline_cache,
    );
    let pass_descriptor = RenderPassDescriptor {
        label: Some("recording_frame"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &image.texture_view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    };
    let mut render_pass = ctx.command_encoder().begin_render_pass(&pass_descriptor);
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(0, &bind_group, &[]);
    render_pass.draw(0..3, 0..1);
}

// ==== Helpers ================================================================

/// Ends a recording: stops its sound, drops its frames' sender so the encoder finishes, and
/// returns the threads still writing it.
fn end(recorder: Recorder) -> Vec<JoinHandle<()>> {
    if recorder.dropped_frames > 0 {
        warn!(
            "the encoder fell behind: {} frames of the recording were dropped",
            recorder.dropped_frames
        );
    }
    let mut threads = vec![recorder.encoder];
    #[cfg(windows)]
    if let Some(sound) = recorder.sound {
        sound.stop();
        threads.push(sound.thread);
    }
    threads
}

/// Returns the frame of the video due `elapsed_secs` into the recording at `frame_rate`, if
/// it is `next_slot` or later; the slots the game skipped are left out.
fn due_slot(elapsed_secs: f64, frame_rate: u32, next_slot: u64) -> Option<u64> {
    let current = (elapsed_secs * f64::from(frame_rate)).floor().max(0.0) as u64;
    (current >= next_slot).then_some(current)
}

/// Returns `size` rounded down to even numbers, as H.264's colour, kept at half the size,
/// needs, and never under 2.
fn even_size(size: UVec2) -> UVec2 {
    (size & !1).max(UVec2::splat(2))
}

/// Returns where a recording starting now goes in `gallery`: named by the second, `_2` after
/// a name taken, by a finished video or one still being written.
fn free_video_path(gallery: &Path) -> Option<PathBuf> {
    std::fs::create_dir_all(gallery).ok()?;
    let stamp = file_stamp(unix_now().as_secs());
    (1..)
        .map(|count| gallery.join(numbered_file_name(&stamp, count, VIDEO_EXTENSION)))
        .find(|path| !path.exists() && !part_path(path).exists())
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::fresh_test_dir;

    const FRAME_RATE: u32 = 60;
    const FRAME_SECS: f64 = 1.0 / FRAME_RATE as f64;

    #[test]
    fn the_first_frame_is_due_at_once() {
        assert_eq!(due_slot(0.0, FRAME_RATE, 0), Some(0));
    }

    #[test]
    fn no_frame_is_due_before_its_slot() {
        assert_eq!(due_slot(FRAME_SECS / 2.0, FRAME_RATE, 1), None);
        assert_eq!(due_slot(FRAME_SECS * 1.5, FRAME_RATE, 1), Some(1));
    }

    #[test]
    fn a_long_frame_of_the_game_skips_the_slots_it_missed() {
        assert_eq!(due_slot(FRAME_SECS * 5.5, FRAME_RATE, 1), Some(5));
    }

    #[test]
    fn an_odd_window_is_recorded_at_the_even_size_under_it() {
        assert_eq!(even_size(UVec2::new(1919, 1081)), UVec2::new(1918, 1080));
        assert_eq!(even_size(UVec2::new(1, 0)), UVec2::splat(2));
    }

    #[test]
    fn a_recording_never_takes_the_name_of_one_being_written() {
        let gallery = fresh_test_dir("bevy-starter-free-video-path");
        let first = free_video_path(&gallery).unwrap();
        std::fs::write(part_path(&first), []).unwrap();
        let second = free_video_path(&gallery).unwrap();
        assert_ne!(first, second);
    }
}
