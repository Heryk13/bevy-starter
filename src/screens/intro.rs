use bevy::asset::LoadedFolder;
use bevy::prelude::*;

use super::Screen;
use super::loading::IntroAssets;
use super::splash::{SKIP_KEYS, SKIP_MOUSE_BUTTONS};
use crate::assets::images_in;

// ==== Config =================================================================

// set by the soundtrack: 90 frames over its 6 seconds, so picture and sound end
// together; recalibrate against the new track whenever the intro is re-exported
const INTRO_FPS: f32 = 15.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        OnEnter(Screen::Intro),
        (spawn_intro_screen, play_intro_music),
    );
    app.add_systems(
        Update,
        (
            (show_intro_frame, leave_intro_screen).chain(),
            skip_intro_screen,
        )
            .run_if(in_state(Screen::Intro)),
    );
    app.add_systems(OnExit(Screen::Intro), unload_intro_assets);
}

// ==== Components =============================================================

/// The intro's frames in playing order, and how far into them it is.
#[derive(Component)]
struct IntroPlayback {
    frames: Vec<Handle<Image>>,
    timer: Timer,
}

// ==== Systems ================================================================

/// Spawns the intro screen over black, the colour it starts and ends on, with its
/// frames sorted by file name: the zero-padded names are what keep them in order.
fn spawn_intro_screen(
    mut commands: Commands,
    intro_assets: Res<IntroAssets>,
    folders: Res<Assets<LoadedFolder>>,
) {
    let frames = folders
        .get(&intro_assets.frames)
        .map(images_in)
        .unwrap_or_default();
    let frame_count = frames.len();
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ImageNode::default(),
        IntroPlayback {
            frames,
            timer: Timer::from_seconds(frame_count as f32 / INTRO_FPS, TimerMode::Once),
        },
        BackgroundColor(Color::BLACK),
        DespawnOnExit(Screen::Intro),
    ));
}

/// Starts the intro's soundtrack, already loaded, so it begins with the first frame.
fn play_intro_music(mut commands: Commands, intro_assets: Res<IntroAssets>) {
    commands.spawn((
        AudioPlayer::new(intro_assets.music.clone()),
        PlaybackSettings::ONCE,
        DespawnOnExit(Screen::Intro),
    ));
}

/// Shows the frame that matches how long the intro has been playing.
fn show_intro_frame(time: Res<Time>, intro: Single<(&mut IntroPlayback, &mut ImageNode)>) {
    let (mut playback, mut image_node) = intro.into_inner();
    playback.timer.tick(time.delta());
    let frame = frame_index(playback.timer.elapsed_secs(), playback.frames.len());
    if let Some(frame_handle) = playback.frames.get(frame) {
        image_node.image = frame_handle.clone();
    }
}

/// Moves on to gameplay once the last frame has had its time on screen.
fn leave_intro_screen(
    playback: Single<&IntroPlayback>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if playback.timer.is_finished() {
        next_screen.set(Screen::Gameplay);
    }
}

/// Skips to gameplay on the same keys and buttons that skip the splash screen.
fn skip_intro_screen(
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if keyboard_input.any_just_pressed(SKIP_KEYS)
        || mouse_input.any_just_pressed(SKIP_MOUSE_BUTTONS)
    {
        next_screen.set(Screen::Gameplay);
    }
}

/// Drops the last handles to the intro's frames and music, so Bevy unloads them
/// instead of holding them for the rest of the game.
fn unload_intro_assets(mut commands: Commands) {
    commands.remove_resource::<IntroAssets>();
}

// ==== Helpers ================================================================

/// Returns which frame is showing at the given time into the intro, holding on the
/// last one once it runs out.
fn frame_index(elapsed_secs: f32, frame_count: usize) -> usize {
    ((elapsed_secs * INTRO_FPS) as usize).min(frame_count.saturating_sub(1))
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_is_first_at_the_start() {
        assert_eq!(frame_index(0.0, 91), 0);
    }

    #[test]
    fn frame_advances_with_fps() {
        assert_eq!(frame_index(1.5 / INTRO_FPS, 91), 1);
    }

    #[test]
    fn frame_holds_on_last() {
        assert_eq!(frame_index(100.0, 91), 90);
    }

    #[test]
    fn frame_is_zero_when_there_are_no_frames() {
        assert_eq!(frame_index(0.0, 0), 0);
    }
}
