use bevy::prelude::*;

use super::{DEV_TOOLS_Z_INDEX, DevUi};
use crate::input::{ActionName, Bindings};
use crate::overlays::{binding_legend, keycap};
use crate::ui::{SMALL_FONT_SIZE_PX, muted_text};

// ==== Config =================================================================

// ---- row: top centre, a status a tool left on, side by side -----------------
const STATUS_TOP_PX: f32 = 10.0;
// between two statuses
const STATUS_ROW_GAP_PX: f32 = 18.0;
// between a status's keycap and its words
const STATUS_GAP_PX: f32 = 8.0;

// ==== Plugin =================================================================

/// The status row, top centre: what a dev tool that stays on shows while it is on, its key on
/// a keycap beside its words (F6's view, F7's flying), side by side so two never cover each
/// other. Drawn by the screen camera, as the F1 hint is, and hidden in photo mode with every
/// dev window.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_status_row);
}

// ==== Status =================================================================

/// Spawns a status in the row: `action`'s key on a keycap, beside `words`, with `marker` for
/// the tool to find it by and despawn it when it goes off. The pointer passes through it, as
/// through the F1 hint.
pub(super) fn spawn_status(
    commands: &mut Commands,
    row: Entity,
    marker: impl Bundle,
    action: ActionName,
    bindings: &Bindings,
    words: &'static str,
) {
    let legend = bindings
        .binding(action)
        .map(binding_legend)
        .unwrap_or_default();
    commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px(STATUS_GAP_PX),
                ..default()
            },
            marker,
            ChildOf(row),
            children![
                keycap(action, legend),
                (Text::new(words), muted_text(SMALL_FONT_SIZE_PX)),
            ],
        ))
        .insert_recursive::<Children>(Pickable::IGNORE);
}

// ==== Components =============================================================

/// Marks the row the statuses are spawned in.
#[derive(Component)]
pub(super) struct StatusRow;

// ==== Systems ================================================================

/// Spawns the row, empty until a tool goes on.
fn spawn_status_row(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(STATUS_TOP_PX),
            left: px(0),
            right: px(0),
            justify_content: JustifyContent::Center,
            column_gap: px(STATUS_ROW_GAP_PX),
            ..default()
        },
        // over every screen root the screen camera draws
        DEV_TOOLS_Z_INDEX,
        Pickable::IGNORE,
        DevUi,
        StatusRow,
    ));
}
