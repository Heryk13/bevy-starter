use bevy::asset::LoadedFolder;
use bevy::prelude::*;

use super::Screen;
use crate::assets::images_in;
use crate::fade::Fade;

// ==== Config =================================================================

// ---- skip input, shared with the intro so both skip the same way ------------
pub(super) const SKIP_KEYS: [KeyCode; 5] = [
    KeyCode::Escape,
    KeyCode::Space,
    KeyCode::Enter,
    KeyCode::KeyE,
    KeyCode::KeyF,
];
pub(super) const SKIP_MOUSE_BUTTONS: [MouseButton; 2] = [MouseButton::Left, MouseButton::Right];

// ---- logo animation: wait, fade in, hold, fade out --------------------------
const LOGO_WAIT_SECS: f32 = 0.5;
const LOGO_FADE_IN_SECS: f32 = 0.8;
const LOGO_HOLD_SECS: f32 = 1.2;
const LOGO_FADE_OUT_SECS: f32 = 0.5;

// ---- derived from the durations above ---------------------------------------
const LOGO_FADE: Fade = Fade {
    delay_secs: LOGO_WAIT_SECS,
    fade_in_secs: LOGO_FADE_IN_SECS,
    hold_secs: LOGO_HOLD_SECS,
    fade_out_secs: LOGO_FADE_OUT_SECS,
};

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Splash), spawn_splash_screen);
    app.add_systems(
        Update,
        // chained: a skip ends the current logo, a new logo needs the folder in the
        // list, and the fade reads the timer the advance just ticked
        (
            skip_splash_logo,
            fill_splash_logos,
            advance_splash_logos,
            fade_splash_logo,
        )
            .chain()
            .run_if(in_state(Screen::Splash)),
    );
}

// ==== Components =============================================================

/// The logos to show in order, which one is on screen, and how far into it we are.
#[derive(Component)]
struct SplashSequence {
    /// The logos folder while it still has to be read, `None` once its logos are in.
    folder: Option<Handle<LoadedFolder>>,
    logos: Vec<Handle<Image>>,
    index: usize,
    timer: Timer,
}

// ==== Systems ================================================================

/// Spawns the splash screen with the Bevy logo first, the folder's logos to follow.
fn spawn_splash_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bevy_logo = asset_server.load("splash/bevy.png");
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            // a grid item never shrinks below its content by default, so a logo wider than
            // the window would stretch this past it and push the logo off centre
            min_width: px(0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Node {
                width: percent(30),
                ..default()
            },
            // starts invisible, `fade_splash_logo` fades it in
            ImageNode::new(bevy_logo.clone()).with_color(Color::WHITE.with_alpha(0.0)),
            SplashSequence {
                folder: Some(asset_server.load_folder("splash/logos")),
                logos: vec![bevy_logo.clone()],
                index: 0,
                timer: Timer::from_seconds(LOGO_FADE.total_secs(), TimerMode::Once),
            },
        )],
        // despawns the whole screen, the logo and its sequence included, on the way out
        DespawnOnExit(Screen::Splash),
    ));
}

/// Ends the current logo early on a skip key or mouse button, letting
/// `advance_splash_logos` move on to the next one or leave.
fn skip_splash_logo(
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut sequence: Single<&mut SplashSequence>,
) {
    if keyboard_input.any_just_pressed(SKIP_KEYS)
        || mouse_input.any_just_pressed(SKIP_MOUSE_BUTTONS)
    {
        sequence.timer.finish();
    }
}

// ponytail: a folder still loading when the Bevy logo ends is skipped; wait on its
// load state before leaving if slow disks or large logos ever make that happen
/// Adds the folder's logos after the Bevy one, sorted by file name, once it loads.
fn fill_splash_logos(
    mut sequence: Single<&mut SplashSequence>,
    folders: Res<Assets<LoadedFolder>>,
) {
    let Some(folder_handle) = &sequence.folder else {
        return;
    };
    let Some(folder) = folders.get(folder_handle) else {
        return;
    };
    sequence.logos.extend(images_in(folder));
    sequence.folder = None;
}

/// Shows the next logo when the current one's time is up, or leaves for the menu
/// once the last one has been shown.
fn advance_splash_logos(
    sequence: Single<(&mut SplashSequence, &mut ImageNode)>,
    time: Res<Time>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    let (mut sequence, mut logo) = sequence.into_inner();
    sequence.timer.tick(time.delta());
    if sequence.timer.is_finished() {
        if let Some(next_index) = next_logo(sequence.index, sequence.logos.len()) {
            sequence.index = next_index;
            if let Some(logo_handle) = sequence.logos.get(next_index) {
                logo.image = logo_handle.clone();
            }
            sequence.timer.reset();
        } else {
            next_screen.set(Screen::Menu);
        }
    }
}

/// Fades the logo in and out following the splash timer.
fn fade_splash_logo(logo: Single<(&mut ImageNode, &SplashSequence)>) {
    let (mut logo, sequence) = logo.into_inner();
    let elapsed_secs = sequence.timer.elapsed_secs();
    let alpha = LOGO_FADE.alpha(elapsed_secs);
    logo.color.set_alpha(alpha);
}

// ==== Helpers ================================================================

/// Returns the logo after `index`, or `None` when the last one has been shown.
fn next_logo(index: usize, count: usize) -> Option<usize> {
    (index + 1 < count).then_some(index + 1)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_advances_while_more_remain() {
        assert_eq!(next_logo(0, 3), Some(1));
    }

    #[test]
    fn logo_ends_after_the_last_one() {
        assert_eq!(next_logo(2, 3), None);
    }
}
