use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::fade::Fade;
use crate::i18n::{Language, Translations};
use crate::overlays::Overlay;
use crate::save::{SaveFinished, Session, has_unsaved_progress};
use crate::screens::Screen;
use crate::ui::{
    ACCENT, ButtonKind, DANGER, SMALL_FONT_SIZE_PX, TEXT_MUTED, card, dialog, floating_root, font,
    heading, menu_button,
};

// ==== Config =================================================================

// below the overlays, so settings opened from the pause menu draws over it
const PAUSE_Z_INDEX: GlobalZIndex = GlobalZIndex(50);
// narrower than an overlay's card: it only holds a column of buttons
const PAUSE_WIDTH_PX: f32 = 340.0;
const PAUSE_BUTTON_GAP_PX: f32 = 10.0;

// translation keys
const QUIT_QUESTION: &str = "dialog-quit-without-saving";
const SAVED_TEXT: &str = "status-saved";
const SAVE_FAILED_TEXT: &str = "status-save-failed";

// no delay and no fade in, so the player learns how the save went at once; only the
// leaving is gentle
const STATUS_FADE: Fade = Fade {
    delay_secs: 0.0,
    fade_in_secs: 0.0,
    hold_secs: 2.0,
    fade_out_secs: 0.2,
};

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_sub_state::<Pause>();
    app.add_systems(OnEnter(Pause::On), (spawn_pause_menu, pause_time));
    app.add_systems(OnExit(Pause::On), unpause_time);
    app.add_observer(activate_pause_button.run_if(in_state(Pause::On)));
    app.add_observer(confirm_quit.run_if(in_state(Pause::On)));
    app.add_observer(show_save_status.run_if(in_state(Pause::On)));
    app.add_systems(Update, fade_save_status.run_if(in_state(Pause::On)));
}

// ==== State ==================================================================

/// Whether the game is frozen. It only exists during gameplay, and starts `Off`.
#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(Screen = Screen::Gameplay)]
pub(crate) enum Pause {
    #[default]
    Off,
    On,
}

// ==== Components =============================================================

/// Names what a pause menu button does when it is pressed.
#[derive(Component)]
enum PauseAction {
    Resume,
    Save,
    LoadGame,
    Settings,
    QuitToMenu,
    QuitToDesktop,
}

/// Where a quit from the pause menu leaves for.
enum QuitTo {
    Menu,
    Desktop,
}

/// Marks the Yes of `QUIT_QUESTION`, with where to go once it is pressed.
#[derive(Component)]
struct ConfirmQuit(QuitTo);

/// Marks the pause menu's line of text that says how the last save went, with how long it
/// has been shown. `show_save_status` writes the line and sets `shown_secs` back to zero.
#[derive(Component)]
struct SaveStatus {
    shown_secs: f32,
}

// ==== Systems ================================================================

/// Spawns the pause menu, floating over the frozen game, blurring it and blocking clicks to it.
fn spawn_pause_menu(mut commands: Commands) {
    commands.spawn((
        floating_root(PAUSE_Z_INDEX),
        children![(
            card(PAUSE_WIDTH_PX),
            children![
                heading("pause-title"),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(PAUSE_BUTTON_GAP_PX),
                        ..default()
                    },
                    children![
                        (menu_button("pause-resume"), PauseAction::Resume),
                        (menu_button("pause-save"), PauseAction::Save),
                        (menu_button("pause-load-game"), PauseAction::LoadGame),
                        (menu_button("pause-settings"), PauseAction::Settings),
                        (menu_button("pause-quit-to-menu"), PauseAction::QuitToMenu),
                        (
                            menu_button("pause-quit-to-desktop"),
                            PauseAction::QuitToDesktop
                        ),
                    ],
                ),
                (
                    Text::new(""),
                    font(SMALL_FONT_SIZE_PX),
                    TextColor(TEXT_MUTED),
                    SaveStatus { shown_secs: 0.0 },
                ),
            ],
        )],
        DespawnOnExit(Pause::On),
    ));
}

/// Freezes virtual time, so everything that reads `Res<Time>` stops with the game.
fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

/// Runs the action of the pause menu button that was pressed.
fn activate_pause_button(
    activate: On<Activate>,
    mut commands: Commands,
    actions: Query<&PauseAction>,
    mut next_pause: ResMut<NextState<Pause>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    mut next_screen: ResMut<NextState<Screen>>,
    session: Res<Session>,
) {
    if let Ok(action) = actions.get(activate.entity) {
        match action {
            PauseAction::Resume => {
                next_pause.set(Pause::Off);
            }
            PauseAction::Save => {
                next_overlay.set(Overlay::SaveGame);
            }
            PauseAction::LoadGame => {
                next_overlay.set(Overlay::LoadGame);
            }
            PauseAction::Settings => {
                next_overlay.set(Overlay::Settings);
            }
            PauseAction::QuitToMenu => {
                if has_unsaved_progress(&session) {
                    commands.spawn((
                        dialog(QUIT_QUESTION, ButtonKind::Danger, ConfirmQuit(QuitTo::Menu)),
                        DespawnOnExit(Pause::On),
                    ));
                } else {
                    next_screen.set(Screen::Menu);
                }
            }
            PauseAction::QuitToDesktop => {
                if has_unsaved_progress(&session) {
                    commands.spawn((
                        dialog(
                            QUIT_QUESTION,
                            ButtonKind::Danger,
                            ConfirmQuit(QuitTo::Desktop),
                        ),
                        DespawnOnExit(Pause::On),
                    ));
                } else {
                    commands.write_message(AppExit::Success);
                }
            }
        }
    }
}

/// Quits for real once `QUIT_QUESTION` is answered Yes.
fn confirm_quit(
    activate: On<Activate>,
    mut commands: Commands,
    confirms: Query<&ConfirmQuit>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if let Ok(confirm) = confirms.get(activate.entity) {
        match confirm {
            ConfirmQuit(QuitTo::Menu) => {
                next_screen.set(Screen::Menu);
            }
            ConfirmQuit(QuitTo::Desktop) => {
                commands.write_message(AppExit::Success);
            }
        }
    }
}

/// Writes how the last save went on the status line, in the accent or in red, and starts
/// its fade over.
fn show_save_status(
    finished: On<SaveFinished>,
    translations: Res<Translations>,
    language: Res<Language>,
    mut statuses: Query<(&mut Text, &mut TextColor, &mut SaveStatus)>,
) {
    let (key, new_color) = if finished.saved {
        (SAVED_TEXT, ACCENT)
    } else {
        (SAVE_FAILED_TEXT, DANGER)
    };
    for (mut text, mut color, mut status) in &mut statuses {
        text.0 = translations.text(*language, key);
        color.0 = new_color;
        status.shown_secs = 0.0;
    }
}

/// Fades the save status line out by `STATUS_FADE`. It counts real time, because virtual
/// time is frozen while paused and a line timed by it would never leave.
fn fade_save_status(time: Res<Time<Real>>, mut statuses: Query<(&mut SaveStatus, &mut TextColor)>) {
    for (mut status, mut color) in &mut statuses {
        status.shown_secs += time.delta_secs();
        if status.shown_secs > STATUS_FADE.total_secs() {
            status.shown_secs = STATUS_FADE.total_secs();
        }
        color.0.set_alpha(STATUS_FADE.alpha(status.shown_secs));
    }
}

/// Starts virtual time again. Also runs when gameplay is left while paused, so the menu
/// never inherits a frozen clock.
fn unpause_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_outside_gameplay_needs_no_session() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<Screen>();
        app.init_state::<Overlay>();
        plugin(&mut app);
        app.update();
        let entity = app.world_mut().spawn_empty().id();
        app.world_mut().trigger(Activate { entity });
        app.update();
        // reaching this line is the point: an unscoped observer asked for the
        // missing Session and panicked before it
        assert_eq!(
            app.world().resource::<State<Screen>>().get(),
            &Screen::Splash
        );
    }
}
