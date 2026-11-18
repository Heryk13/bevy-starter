use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::input::{Action, ActionName, Binding, Page, action_just_pressed, register_action};
use crate::save::take_pause_picture;
use crate::screens::Pause;
use crate::ui::blur::GlassLayers;
use crate::ui::{ButtonKind, Dialog, button, card, floating_root};

mod credits;
mod saves;
mod settings;

#[cfg(feature = "dev")]
pub(crate) use settings::{binding_legend, keycap};

// ==== Config =================================================================

// ---- pause: the way out of every panel, and into the pause menu -------------
const PAUSE: ActionName = ActionName("pause");
// Esc, and never another key: it closes every panel, so a player who could move it could
// lose the way out of the menus
const PAUSE_KEY: KeyCode = KeyCode::Escape;

// ---- overlays ---------------------------------------------------------------
// room left above it on purpose, for anything that must draw over an overlay
const OVERLAY_Z_INDEX: GlobalZIndex = GlobalZIndex(100);

// wide enough for the Settings grid, whose language names sit side by side beside their
// label, and for a save's row with its Delete beside it
const OVERLAY_WIDTH_PX: f32 = 580.0;
// between the saves in a list, and between controls side by side: the name field and its
// New Save, a save and its Delete
const OVERLAY_GAP_PX: f32 = 12.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Overlay>();
    app.add_plugins(credits::plugin);
    app.add_plugins(saves::plugin);
    app.add_plugins(settings::plugin);
    app.add_observer(close_overlay);
    register_action(
        app,
        Action {
            name: PAUSE,
            default: Binding::Key(PAUSE_KEY),
            page: Page::Game,
            // no label: Esc is not listed, since it cannot change
            label: None,
        },
    );
    app.add_systems(Update, apply_esc_action.run_if(action_just_pressed(PAUSE)));
    app.init_resource::<GlassLayers>();
    app.world_mut()
        .resource_mut::<GlassLayers>()
        .0
        .push(OVERLAY_Z_INDEX);
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

/// Closes the topmost layer on Esc, one per press, or pauses running gameplay when nothing
/// is open, as `esc_action` decides. Pausing also takes the picture a save made under the
/// pause menu will carry.
fn apply_esc_action(
    mut commands: Commands,
    dialogs: Query<(), With<Dialog>>,
    overlay: Res<State<Overlay>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    pause: Option<Res<State<Pause>>>,
    mut next_pause: ResMut<NextState<Pause>>,
) {
    let pause = pause.map(|pause| *pause.get());
    match esc_action(!dialogs.is_empty(), *overlay.get(), pause) {
        EscAction::CloseDialog => commands.despawn_all::<With<Dialog>>(),
        EscAction::CloseOverlay => next_overlay.set(Overlay::None),
        EscAction::SetPause(pause) => {
            // now, not as the pause menu spawns: that is a frame from here, on entering
            // `Pause::On`, and this frame is the last one that shows the game alone
            if pause == Pause::On {
                take_pause_picture(&mut commands);
            }
            next_pause.set(pause);
        }
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
