use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::screens::gameplay::pause::Pause;
use crate::ui::Dialog;

mod credits;
mod saves;
mod settings;

// ==== Config =================================================================

// room left above it on purpose, for anything that must draw over an overlay
const OVERLAY_Z_INDEX: GlobalZIndex = GlobalZIndex(100);

const OVERLAY_BACKDROP_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
const OVERLAY_GAP_PX: f32 = 12.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Overlay>();
    app.add_plugins(credits::plugin);
    app.add_plugins(saves::plugin);
    app.add_plugins(settings::plugin);
    app.add_observer(close_overlay);
    app.add_systems(Update, go_back_on_esc);
}

// ==== Overlay Root ===========================================================

/// Returns the root every overlay spawns its panel in: the whole window, darkened, above
/// the screen and the pause menu, and blocking clicks to them.
fn overlay_root() -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: px(OVERLAY_GAP_PX),
            ..default()
        },
        BackgroundColor(OVERLAY_BACKDROP_COLOR),
        OVERLAY_Z_INDEX,
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

/// Closes the topmost thing on Esc, one layer per press: a dialog, then an overlay, and
/// with neither open it pauses or resumes gameplay.
fn go_back_on_esc(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    dialogs: Query<Entity, With<Dialog>>,
    overlay: Res<State<Overlay>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    pause: Option<Res<State<Pause>>>,
    mut next_pause: ResMut<NextState<Pause>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if !dialogs.is_empty() {
            for dialog in &dialogs {
                commands.entity(dialog).despawn();
            }
        } else if *overlay.get() != Overlay::None {
            next_overlay.set(Overlay::None);
        } else if let Some(pause) = pause {
            match pause.get() {
                Pause::On => next_pause.set(Pause::Off),
                Pause::Off => next_pause.set(Pause::On),
            }
        }
    }
}
