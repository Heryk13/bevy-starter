use bevy::asset::LoadedFolder;
use bevy::prelude::*;
use rand::seq::SliceRandom;

use super::Screen;
use crate::fade::Fade;

// ==== Config =================================================================

// ---- slideshow: each picture fades in, holds, then fades out ----------------
const SLIDE_SECS: f32 = 5.5;
const SLIDE_FADE_SECS: f32 = 0.5;

// ---- derived: what the two fades leave of a slide is spent fully shown ------
const SLIDE_FADE: Fade = Fade {
    delay_secs: 0.0,
    fade_in_secs: SLIDE_FADE_SECS,
    hold_secs: SLIDE_SECS - 2.0 * SLIDE_FADE_SECS,
    fade_out_secs: SLIDE_FADE_SECS,
};

// ---- loading bar: a track along the bottom, filled as the intro loads -------
const LOADING_BAR_BOTTOM_PX: f32 = 48.0;
const LOADING_BAR_WIDTH_PERCENT: f32 = 80.0;
const LOADING_BAR_HEIGHT_PX: f32 = 8.0;
const LOADING_BAR_TRACK_COLOR: Color = Color::BLACK;
const LOADING_BAR_FILL_COLOR: Color = Color::WHITE;

// ---- derived from the width above, so the bar stays centred -----------------
const LOADING_BAR_LEFT_PERCENT: f32 = (100.0 - LOADING_BAR_WIDTH_PERCENT) / 2.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        OnEnter(Screen::Loading),
        (spawn_loading_screen, load_intro_assets),
    );
    app.add_systems(
        Update,
        (
            (fill_slideshow, advance_slideshow, fade_slideshow).chain(),
            (update_loading_bar, leave_loading_screen),
        )
            .run_if(in_state(Screen::Loading)),
    );
}

// ==== Components =============================================================

/// Cycles the loading screen's image through every picture in a folder.
#[derive(Component)]
struct Slideshow {
    folder: Handle<LoadedFolder>,
    slides: Vec<Handle<Image>>,
    index: usize,
    timer: Timer,
}

/// Marks the part of the loading bar that grows as the intro loads.
#[derive(Component)]
struct LoadingBarFill;

// ==== Resources ==============================================================

/// The intro's frames and music, loaded here and kept past this screen, since
/// despawning with it would unload them just as the intro needs them.
#[derive(Resource)]
pub(super) struct IntroAssets {
    pub(super) frames: Handle<LoadedFolder>,
    pub(super) music: Handle<AudioSource>,
}

// ==== Systems ================================================================

/// Spawns the loading screen: a full-window slideshow over black, the colour the
/// menu fades out to.
fn spawn_loading_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ImageNode::default(),
        Slideshow {
            folder: asset_server.load_folder("loading"),
            slides: Vec::new(),
            index: 0,
            timer: Timer::from_seconds(SLIDE_SECS, TimerMode::Repeating),
        },
        children![(
            Node {
                bottom: px(LOADING_BAR_BOTTOM_PX),
                left: percent(LOADING_BAR_LEFT_PERCENT),
                width: percent(LOADING_BAR_WIDTH_PERCENT),
                height: px(LOADING_BAR_HEIGHT_PX),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(LOADING_BAR_TRACK_COLOR),
            children![(
                Node {
                    width: percent(0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(LOADING_BAR_FILL_COLOR),
                LoadingBarFill
            )]
        )],
        BackgroundColor(Color::BLACK),
        DespawnOnExit(Screen::Loading),
    ));
}

/// Starts loading the intro's frames and music, the work this screen waits on.
fn load_intro_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(IntroAssets {
        frames: asset_server.load_folder("intro/frames"),
        music: asset_server.load("intro/music.ogg"),
    });
}

/// Fills the slideshow once its folder has loaded: every image in it, shuffled,
/// starting on the first.
fn fill_slideshow(
    slideshow: Single<(&mut Slideshow, &mut ImageNode)>,
    folders: Res<Assets<LoadedFolder>>,
) {
    let (mut slideshow, mut image_node) = slideshow.into_inner();
    if !slideshow.slides.is_empty() {
        return;
    }
    let Some(folder) = folders.get(&slideshow.folder) else {
        return;
    };

    for handle in &folder.handles {
        if let Ok(image) = handle.clone().try_typed::<Image>() {
            slideshow.slides.push(image);
        }
    }

    // ponytail: shuffled once, so a second lap repeats the same order; reshuffle per
    // lap (keeping the last slide off the front) if loads ever get long enough to see it
    slideshow.slides.shuffle(&mut rand::rng());
    if let Some(first) = slideshow.slides.first() {
        image_node.image = first.clone();
    }
}

/// Moves the slideshow to its next picture every `SLIDE_SECS`.
fn advance_slideshow(slideshow: Single<(&mut Slideshow, &mut ImageNode)>, time: Res<Time>) {
    let (mut slideshow, mut image_node) = slideshow.into_inner();
    if slideshow.slides.is_empty() {
        return;
    }

    slideshow.timer.tick(time.delta());
    if slideshow.timer.just_finished() {
        slideshow.index = next_slide(slideshow.index, slideshow.slides.len());
        image_node.image = slideshow.slides[slideshow.index].clone();
    }
}

/// Fades the current picture in and out following the slideshow timer.
fn fade_slideshow(slideshow: Single<(&mut ImageNode, &Slideshow)>) {
    let (mut image_node, slideshow) = slideshow.into_inner();
    let elapsed_secs = slideshow.timer.elapsed_secs();
    let alpha = SLIDE_FADE.alpha(elapsed_secs);
    image_node.color.set_alpha(alpha);
}

/// Fills the loading bar with the share of the intro's frames and music that have
/// finished loading, the music counting as one more item.
fn update_loading_bar(
    intro_assets: Res<IntroAssets>,
    folders: Res<Assets<LoadedFolder>>,
    asset_server: Res<AssetServer>,
    mut loading_bar_fill: Single<&mut Node, With<LoadingBarFill>>,
) {
    let Some(folder) = folders.get(&intro_assets.frames) else {
        return;
    };
    let mut loaded = 0;
    for handle in &folder.handles {
        if asset_server.is_loaded_with_dependencies(handle.id()) {
            loaded += 1;
        }
    }
    if asset_server.is_loaded_with_dependencies(&intro_assets.music) {
        loaded += 1;
    }
    let progress = loading_progress(loaded, folder.handles.len() + 1);
    loading_bar_fill.width = percent(progress * 100.0);
}

/// Leaves once the intro's frames and music are in, counting failures as done: Bevy
/// already logs them, and a missing file is cosmetic where a screen that never ends
/// is not.
fn leave_loading_screen(
    intro_assets: Res<IntroAssets>,
    asset_server: Res<AssetServer>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    let frames_load_state = asset_server.recursive_dependency_load_state(&intro_assets.frames);
    let music_load_state = asset_server.load_state(&intro_assets.music);
    let frames_done = frames_load_state.is_loaded() || frames_load_state.is_failed();
    let music_done = music_load_state.is_loaded() || music_load_state.is_failed();
    if frames_done && music_done {
        next_screen.set(Screen::Intro);
    }
}

// ==== Helpers ================================================================

/// Returns the slide after `index`, wrapping back to the first after the last.
fn next_slide(index: usize, count: usize) -> usize {
    (index + 1) % count
}

/// Returns how far along the load is, from 0.0 to 1.0. Nothing to load counts as
/// done, rather than dividing by zero.
fn loading_progress(loaded: usize, total: usize) -> f32 {
    if total == 0 {
        1.0
    } else {
        loaded as f32 / total as f32
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    #[test]
    fn slide_advances_to_the_next_one() {
        assert_eq!(next_slide(0, 5), 1);
    }

    #[test]
    fn last_slide_wraps_to_the_first() {
        assert_eq!(next_slide(4, 5), 0);
    }

    #[test]
    fn loading_progress_is_zero_when_nothing_loaded() {
        assert!((loading_progress(0, 5) - 0.0).abs() < TOLERANCE);
    }

    #[test]
    fn loading_progress_is_half_when_half_loaded() {
        assert!((loading_progress(2, 4) - 0.5).abs() < TOLERANCE);
    }

    #[test]
    fn loading_progress_is_done_when_all_loaded() {
        assert!((loading_progress(5, 5) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn loading_progress_is_done_when_nothing_to_load() {
        assert!((loading_progress(0, 0) - 1.0).abs() < TOLERANCE);
    }
}
