use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::screens::Screen;

mod credits;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Overlay>();
    app.add_plugins(credits::plugin);
    app.add_observer(close_overlay);
    app.add_systems(Update, go_back_on_esc);
}

// ==== State ==================================================================

/// Panels drawn on top of the current screen, which stays alive underneath.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub(crate) enum Overlay {
    #[default]
    None,
    Credits,
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

/// Closes the open overlay on Esc, or leaves gameplay for the menu when none is open.
fn go_back_on_esc(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    overlay: Res<State<Overlay>>,
    screen: Res<State<Screen>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if *overlay.get() != Overlay::None {
            next_overlay.set(Overlay::None);
        } else if *screen.get() == Screen::Gameplay {
            next_screen.set(Screen::Menu);
        }
    }
}
