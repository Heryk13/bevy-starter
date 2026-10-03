use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::screens::Pause;
use crate::ui::{ButtonKind, Dialog, button, card, floating_root};

mod credits;
mod saves;
mod settings;

// ==== Config =================================================================

// room left above it on purpose, for anything that must draw over an overlay
const OVERLAY_Z_INDEX: GlobalZIndex = GlobalZIndex(100);

// wide enough for the Settings grid and a save's row with its Delete beside it
const OVERLAY_WIDTH_PX: f32 = 520.0;
// between the rows of a list or a grid inside a card
const OVERLAY_GAP_PX: f32 = 12.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Overlay>();
    app.add_plugins(credits::plugin);
    app.add_plugins(saves::plugin);
    app.add_plugins(settings::plugin);
    app.add_observer(close_overlay);
    app.add_systems(
        Update,
        go_back_on_esc.run_if(input_just_pressed(KeyCode::Escape)),
    );
}

// ==== Overlay Panel ==========================================================

/// Returns the root every overlay spawns in: floating above the screen and the pause menu,
/// blurring and dimming them, and blocking clicks to them. Its one child is `overlay_card()`.
fn overlay_root() -> impl Bundle {
    floating_root(OVERLAY_Z_INDEX)
}

/// Returns the card every overlay draws its panel on.
fn overlay_card() -> impl Bundle {
    card(OVERLAY_WIDTH_PX)
}

/// Returns the row at the bottom of a card that holds its Close, on the right.
fn close_footer() -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::End,
            ..default()
        },
        children![(button("overlay-close", ButtonKind::Quiet), CloseOverlay)],
    )
}

// ==== State ==================================================================

/// Panels drawn on top of the current screen, which stays alive underneath.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub(crate) enum Overlay {
    #[default]
    None,
    Credits,
    Settings,
    SaveGame,
    LoadGame,
}

// ==== Components =============================================================

/// Marks a button that closes whichever overlay is open.
#[derive(Component)]
struct CloseOverlay;

// ==== Systems ================================================================

/// Closes the open overlay when a close button is pressed.
fn close_overlay(
    activate: On<Activate>,
    close_buttons: Query<(), With<CloseOverlay>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
) {
    if close_buttons.contains(activate.entity) {
        next_overlay.set(Overlay::None);
    }
}

/// Closes the topmost thing on Esc, one layer per press, as `esc_action` decides.
fn go_back_on_esc(
    mut commands: Commands,
    dialogs: Query<Entity, With<Dialog>>,
    overlay: Res<State<Overlay>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    pause: Option<Res<State<Pause>>>,
    mut next_pause: ResMut<NextState<Pause>>,
) {
    let pause = pause.map(|pause| *pause.get());
    match esc_action(!dialogs.is_empty(), *overlay.get(), pause) {
        EscAction::CloseDialog => {
            for dialog in &dialogs {
                commands.entity(dialog).despawn();
            }
        }
        EscAction::CloseOverlay => next_overlay.set(Overlay::None),
        EscAction::SetPause(pause) => next_pause.set(pause),
        EscAction::Nothing => {}
    }
}

// ==== Helpers ================================================================

// ---- the Esc rule -----------------------------------------------------------

/// What one press of Esc does.
#[derive(Debug, PartialEq)]
enum EscAction {
    CloseDialog,
    CloseOverlay,
    SetPause(Pause),
    Nothing,
}

/// Returns what Esc does: it closes a dialog first, then an overlay, and with neither open
/// it pauses or resumes gameplay. Outside gameplay, where `pause` is `None`, it does nothing.
fn esc_action(dialog_open: bool, overlay: Overlay, pause: Option<Pause>) -> EscAction {
    if dialog_open {
        EscAction::CloseDialog
    } else if overlay != Overlay::None {
        EscAction::CloseOverlay
    } else {
        match pause {
            Some(Pause::On) => EscAction::SetPause(Pause::Off),
            Some(Pause::Off) => EscAction::SetPause(Pause::On),
            None => EscAction::Nothing,
        }
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_closes_a_dialog_before_anything_under_it() {
        let action = esc_action(true, Overlay::Settings, Some(Pause::On));
        assert_eq!(action, EscAction::CloseDialog);
    }

    #[test]
    fn esc_closes_an_overlay_before_resuming() {
        let action = esc_action(false, Overlay::Settings, Some(Pause::On));
        assert_eq!(action, EscAction::CloseOverlay);
    }

    #[test]
    fn esc_resumes_a_paused_game() {
        let action = esc_action(false, Overlay::None, Some(Pause::On));
        assert_eq!(action, EscAction::SetPause(Pause::Off));
    }

    #[test]
    fn esc_pauses_a_running_game() {
        let action = esc_action(false, Overlay::None, Some(Pause::Off));
        assert_eq!(action, EscAction::SetPause(Pause::On));
    }

    #[test]
    fn esc_does_nothing_outside_gameplay_with_nothing_open() {
        let action = esc_action(false, Overlay::None, None);
        assert_eq!(action, EscAction::Nothing);
    }
}
