use bevy::prelude::*;

use crate::ui::blur::GlassLayers;
use crate::ui::{BORDER_PX, card_glass, floating_root};

mod fps;
mod keys_panel;

// ==== Config =================================================================

// above the dialog (200), so a dev window is drawn by a camera of its own over every other
// layer: sharp over the pause menu, with whatever is open under it blurred in its glass
const DEV_TOOLS_Z_INDEX: GlobalZIndex = GlobalZIndex(300);

// ---- windows: small glass read-outs in a corner, from the mockup's popover --
const DEV_WINDOW_PADDING_X_PX: f32 = 12.0;
const DEV_WINDOW_PADDING_Y_PX: f32 = 10.0;
const DEV_WINDOW_RADIUS_PX: f32 = 22.0;
const DEV_WINDOW_GAP_PX: f32 = 6.0;

// ==== Plugin =================================================================

/// The tools a game is made with, left out of release builds: `main.rs` adds this plugin
/// only with the `dev` feature. Each tool is a file of its own under `dev_tools/`.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<GlassLayers>();
    app.world_mut()
        .resource_mut::<GlassLayers>()
        .0
        .push(DEV_TOOLS_Z_INDEX);
    // in the order of their keys, which is the order the keys panel and the Dev page list them
    app.add_plugins((keys_panel::plugin, fps::plugin));
}

// ==== Windows ================================================================

/// Spawns `window` under a root of its own carrying `root_marker`, which the tool despawns
/// to close it: the whole window on the dev tools' layer, like every floating root, but
/// letting the pointer through, so the game under it stays sharp and keeps taking clicks.
/// Every node inside lets it through too, since Bevy's picking stops at a node with no
/// `Pickable`, and the window is built from widgets that bring none.
fn spawn_dev_window(commands: &mut Commands, root_marker: impl Bundle, window: impl Bundle) {
    commands
        // on the root as it spawns, not left to the pass below: `blur.rs` reads it there to
        // give this root no backdrop
        .spawn((
            floating_root(DEV_TOOLS_Z_INDEX),
            Pickable::IGNORE,
            root_marker,
            children![window],
        ))
        .insert_recursive::<Children>(Pickable::IGNORE);
}

/// Returns a dev window placed where `place` says (its `top`, `left` or `right`), for
/// `spawn_dev_window`: a glass sheet that lays its children in a column. Not a `card()`,
/// though floating UI is drawn on one: a card is a centered panel sized for reading, this a
/// corner read-out sized to its few lines.
fn dev_window(place: Node) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            row_gap: px(DEV_WINDOW_GAP_PX),
            padding: UiRect::axes(px(DEV_WINDOW_PADDING_X_PX), px(DEV_WINDOW_PADDING_Y_PX)),
            border: UiRect::all(px(BORDER_PX)),
            border_radius: BorderRadius::all(px(DEV_WINDOW_RADIUS_PX)),
            ..place
        },
        card_glass(),
    )
}
