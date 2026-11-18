use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use super::FileExplorer;
use crate::dev_tools::{DEV_TOOLS_Z_INDEX, DevUi};
use crate::ui::blur::GlassLayers;
use crate::ui::{
    BODY_FONT_SIZE_PX, BUTTON_GAP_PX, ButtonKind, ClosesOnEsc, Dismiss, TEXT, card, floating_root,
    font, words_button,
};

// ==== Config =================================================================

// ---- words: English, as every dev tool's, never read in a release build -----
const CANCEL_WORDS: &str = "Cancel";

// ---- the layer: over the explorer -------------------------------------------
// above the dev tools' layer, where the explorer is, so a question or a menu shows over it; a
// layer of its own, so its glass blurs the explorer under it
const EXPLORER_DIALOG_ABOVE: i32 = 10;

// ---- derived ----------------------------------------------------------------
pub(super) const EXPLORER_DIALOG_Z_INDEX: GlobalZIndex =
    GlobalZIndex(DEV_TOOLS_Z_INDEX.0 + EXPLORER_DIALOG_ABOVE);

// ==== Plugin =================================================================

/// What the explorer asks or offers over its window, its questions and its right-click menu:
/// a layer of their own over the explorer, closed by Esc, by Cancel, and with the explorer.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<GlassLayers>();
    app.world_mut()
        .resource_mut::<GlassLayers>()
        .0
        .push(EXPLORER_DIALOG_Z_INDEX);
    app.add_systems(
        Update,
        close_dialogs_with_explorer.run_if(any_with_component::<ExplorerDialog>),
    );
    // no run_if, as the explorer's buttons work over any screen
    app.add_observer(cancel_explorer_dialog);
    app.add_observer(dismiss_explorer_dialog);
}

// ==== Questions ==============================================================

/// Spawns a question over the explorer, `width_px` wide, and returns its card, for the caller
/// to add what it lists under the question and then its answers, with `spawn_answers`.
pub(super) fn spawn_question(commands: &mut Commands, question: String, width_px: f32) -> Entity {
    let root = commands
        .spawn((
            floating_root(EXPLORER_DIALOG_Z_INDEX),
            DevUi,
            ClosesOnEsc,
            ExplorerDialog,
        ))
        .id();
    commands
        .spawn((
            card(width_px),
            ChildOf(root),
            children![(
                Text::new(question),
                font(BODY_FONT_SIZE_PX),
                TextColor(TEXT),
                TextLayout::justify(Justify::Center),
            )],
        ))
        .id()
}

/// Spawns the answers at the bottom of a question's `card`: Cancel, and a red `yes_words`
/// carrying `on_yes`, the caller's marker for what it does, which the caller's own observer
/// reads, closing the question with `close_explorer_dialogs`.
pub(super) fn spawn_answers(
    commands: &mut Commands,
    card: Entity,
    yes_words: &'static str,
    on_yes: impl Bundle,
) {
    commands.spawn((
        Node {
            justify_content: JustifyContent::Center,
            column_gap: px(BUTTON_GAP_PX),
            ..default()
        },
        ChildOf(card),
        children![
            (words_button(CANCEL_WORDS, ButtonKind::Quiet), CancelDialog),
            (words_button(yes_words, ButtonKind::Danger), on_yes),
        ],
    ));
}

/// Closes every question and menu over the explorer.
pub(super) fn close_explorer_dialogs(commands: &mut Commands) {
    commands.despawn_all::<With<ExplorerDialog>>();
}

// ==== Components =============================================================

/// Marks the root of a question or a menu over the explorer.
#[derive(Component)]
pub(super) struct ExplorerDialog;

/// Marks a question's Cancel, which only closes it.
#[derive(Component)]
struct CancelDialog;

// ==== Systems ================================================================

/// Closes the questions and the menu when the explorer closes under them, so no answer ever
/// changes the disk from a window already gone.
fn close_dialogs_with_explorer(mut commands: Commands, explorers: Query<&FileExplorer>) {
    if explorers.iter().all(|explorer| explorer.closing) {
        close_explorer_dialogs(&mut commands);
    }
}

/// Closes the question when its Cancel is pressed.
fn cancel_explorer_dialog(
    activate: On<Activate>,
    mut commands: Commands,
    cancels: Query<(), With<CancelDialog>>,
) {
    if cancels.contains(activate.entity) {
        close_explorer_dialogs(&mut commands);
    }
}

/// Closes a question or the menu when Esc dismisses it, as Cancel does.
fn dismiss_explorer_dialog(
    dismiss: On<Dismiss>,
    mut commands: Commands,
    dialogs: Query<(), With<ExplorerDialog>>,
) {
    if dialogs.contains(dismiss.entity) {
        commands.entity(dismiss.entity).despawn();
    }
}
