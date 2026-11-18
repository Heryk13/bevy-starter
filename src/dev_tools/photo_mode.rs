use bevy::prelude::*;

use super::{DevUi, dev_window, spawn_dev_window};
use crate::input::{
    Action, ActionName, Binding, Bindings, Page, action_just_pressed, register_action,
};
use crate::overlays::{binding_legend, keycap};
use crate::screens::Pause;
use crate::ui::{BORDER, BORDER_PX, Icon, SMALL_FONT_SIZE_PX, TEXT, font, icon};

mod photo;

// ==== Config =================================================================

// enters and leaves photo mode: the action, and the key it starts on. Esc leaves too,
// through the Esc rule in `overlays.rs`
const PHOTO_MODE: ActionName = ActionName("photo_mode");
const PHOTO_MODE_KEY: KeyCode = KeyCode::F3;

// ---- words: English, as every dev tool's, never read in a release build -----
const PHOTO_MODE_LABEL: &str = "Photo Mode";
const EXIT_WORDS: &str = "Exit";

// ---- hint bar: the bottom centre, what each key does ------------------------
const BAR_BOTTOM_PX: f32 = 24.0;
// between a hint and the divider beside it
const BAR_GAP_PX: f32 = 14.0;
// between an icon, its keycap and its words
const BAR_PART_GAP_PX: f32 = 8.0;
const DIVIDER_HEIGHT_PX: f32 = 26.0;

// ==== Plugin =================================================================

/// Photo mode on F3: the game freezes without the pause menu, and every dev window and the
/// FPS hide, so what shows is the game alone, with a bar saying which keys do what.
/// `pause.rs` freezes the game on entering `Pause::Photo`, as it does for the pause menu.
/// Taking the photo itself is `photo.rs`'s.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PhotoHints>();
    register_action(
        app,
        Action {
            name: PHOTO_MODE,
            default: Binding::Key(PHOTO_MODE_KEY),
            page: Page::Dev,
            label: Some(PHOTO_MODE_LABEL),
        },
    );
    // after Photo Mode, so the Dev page and the keys panel list the photo's key under it
    app.add_plugins(photo::plugin);
    app.add_systems(
        Update,
        toggle_photo_mode.run_if(action_just_pressed(PHOTO_MODE)),
    );
    app.add_systems(OnEnter(Pause::Photo), (hide_dev_ui, spawn_hint_bar));
    app.add_systems(OnExit(Pause::Photo), show_dev_ui);
}

// ==== Hints ==================================================================

/// One part of photo mode's bar: an icon, the actions whose keycaps it shows side by side,
/// and what they do.
#[derive(Clone, Copy)]
pub(super) struct PhotoHint {
    pub(super) icon: Icon,
    pub(super) actions: &'static [ActionName],
    pub(super) words: &'static str,
}

/// Adds `hint` to photo mode's bar, after those added before it and before Exit, which always
/// ends the bar. Called from the plugin of the tool whose keys work in photo mode, so a branch
/// adds its own, a free camera's, without changing this file.
pub(super) fn register_photo_hint(app: &mut App, hint: PhotoHint) {
    app.init_resource::<PhotoHints>();
    app.world_mut().resource_mut::<PhotoHints>().0.push(hint);
}

// ==== Components =============================================================

/// Marks the root of what photo mode itself shows, its bar, its flash and its notice: kept
/// when `hide_dev_ui` hides the other dev windows, and hidden only for the frame a photo is
/// taken.
#[derive(Component)]
struct PhotoModeUi;

// ==== Resources ==============================================================

/// The hints the tools registered for photo mode's bar, in the order they did.
#[derive(Resource, Default)]
struct PhotoHints(Vec<PhotoHint>);

// ==== Systems ================================================================

/// Enters photo mode from the running game, or leaves it. Does nothing over the pause menu,
/// nor outside gameplay, where there is no `Pause`.
fn toggle_photo_mode(pause: Option<Res<State<Pause>>>, mut next_pause: ResMut<NextState<Pause>>) {
    let Some(pause) = pause else {
        return;
    };
    if let Some(next) = photo_key_target(*pause.get()) {
        next_pause.set(next);
    }
}

/// Hides everything the dev tools draw as photo mode begins, but photo mode's own windows.
fn hide_dev_ui(mut roots: Query<&mut Visibility, (With<DevUi>, Without<PhotoModeUi>)>) {
    for mut visibility in &mut roots {
        *visibility = Visibility::Hidden;
    }
}

/// Spawns the bar at the bottom centre saying what each key does in photo mode, each hint
/// beside the keycaps of its bindings and a line between two; it goes when photo mode ends.
fn spawn_hint_bar(mut commands: Commands, bindings: Res<Bindings>, hints: Res<PhotoHints>) {
    // the legends read now: the hints are spawned later, as the bar is
    let parts: Vec<(PhotoHint, Vec<String>)> = bar_hints(&hints.0)
        .into_iter()
        .map(|hint| {
            let legends = hint
                .actions
                .iter()
                .map(|&action| {
                    bindings
                        .binding(action)
                        .map(binding_legend)
                        .unwrap_or_default()
                })
                .collect();
            (hint, legends)
        })
        .collect();
    let window = (
        // left to the root's centring along the bottom edge
        dev_window(Node {
            bottom: px(BAR_BOTTOM_PX),
            ..default()
        }),
        children![(
            Node {
                align_items: AlignItems::Center,
                column_gap: px(BAR_GAP_PX),
                ..default()
            },
            // spawned with the bar, not after it: `spawn_dev_window` lets the pointer through
            // everything the window holds as it spawns
            Children::spawn(SpawnWith(move |bar: &mut ChildSpawner| {
                for (index, (hint, legends)) in parts.into_iter().enumerate() {
                    if index > 0 {
                        bar.spawn(divider());
                    }
                    spawn_hint(bar, hint, legends);
                }
            })),
        )],
    );
    spawn_dev_window(
        &mut commands,
        (PhotoModeUi, DespawnOnExit(Pause::Photo)),
        window,
    );
}

/// Shows everything the dev tools draw again, as photo mode ends.
fn show_dev_ui(mut roots: Query<&mut Visibility, With<DevUi>>) {
    for mut visibility in &mut roots {
        *visibility = Visibility::Inherited;
    }
}

// ==== Helpers ================================================================

/// Returns the hints the bar shows: those the tools registered, in order, then Exit.
fn bar_hints(registered: &[PhotoHint]) -> Vec<PhotoHint> {
    let exit = PhotoHint {
        icon: Icon::LogOut,
        actions: &[PHOTO_MODE],
        words: EXIT_WORDS,
    };
    registered.iter().copied().chain([exit]).collect()
}

/// Spawns one hint into the bar: its icon, a keycap for each of its actions, labelled with
/// `legends`, and its words.
fn spawn_hint(bar: &mut ChildSpawner, hint: PhotoHint, legends: Vec<String>) {
    bar.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: px(BAR_PART_GAP_PX),
        ..default()
    })
    .with_children(|part| {
        part.spawn(icon(hint.icon));
        for (&action, legend) in hint.actions.iter().zip(legends) {
            part.spawn(keycap(action, legend));
        }
        part.spawn((
            Text::new(hint.words),
            font(SMALL_FONT_SIZE_PX),
            TextColor(TEXT),
        ));
    });
}

/// Returns the upright line between two hints.
fn divider() -> impl Bundle {
    (
        Node {
            width: px(BORDER_PX),
            height: px(DIVIDER_HEIGHT_PX),
            ..default()
        },
        BackgroundColor(BORDER),
    )
}

/// Returns the `Pause` that F3 goes to from `pause`: into photo mode from the running game,
/// back out of it, and nowhere from the pause menu, which keeps its own way out.
fn photo_key_target(pause: Pause) -> Option<Pause> {
    match pause {
        Pause::Off => Some(Pause::Photo),
        Pause::Photo => Some(Pause::Off),
        Pause::On => None,
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photo_key_enters_photo_mode_from_the_running_game() {
        assert_eq!(photo_key_target(Pause::Off), Some(Pause::Photo));
    }

    #[test]
    fn photo_key_leaves_photo_mode_for_the_running_game() {
        assert_eq!(photo_key_target(Pause::Photo), Some(Pause::Off));
    }

    #[test]
    fn photo_key_does_nothing_over_the_pause_menu() {
        assert_eq!(photo_key_target(Pause::On), None);
    }

    #[test]
    fn the_bar_lists_the_hints_in_the_order_registered_and_exit_last() {
        let hint = |words| PhotoHint {
            icon: Icon::Camera,
            actions: &[],
            words,
        };
        let words: Vec<&str> = bar_hints(&[hint("Take Photo"), hint("Move")])
            .iter()
            .map(|hint| hint.words)
            .collect();
        assert_eq!(words, ["Take Photo", "Move", EXIT_WORDS]);
    }
}
