use bevy::prelude::*;

use super::{DEV_TOOLS_Z_INDEX, DevUi, dev_window, spawn_dev_window};
use crate::input::{
    Action, ActionName, Binding, Bindings, Page, action_just_pressed, register_action,
};
use crate::overlays::{binding_legend, keycap};
use crate::screens::Pause;
use crate::ui::{Icon, SMALL_FONT_SIZE_PX, TEXT, font, icon, muted_text};

// ==== Config =================================================================

// opens and closes the panel: the action, and the key it starts on
const KEYS_PANEL: ActionName = ActionName("keys_panel");
const KEYS_PANEL_KEY: KeyCode = KeyCode::F1;

// ---- words: English, as every dev tool's, since release builds leave them out --
const KEYS_PANEL_LABEL: &str = "Keys Panel";
const KEYS_PANEL_TITLE: &str = "Dev Tools";

// ---- place: the top left corner, under Bevy's FPS overlay -------------------
// fixed, so the panel does not move up when F2 hides the FPS. Bevy's overlay is its 32 px
// text over a graph twice that tall, about 104 px in all (`bevy_dev_tools`' fps_overlay.rs),
// whose sizes it keeps private
const KEYS_PANEL_TOP_PX: f32 = 116.0;
const KEYS_PANEL_LEFT_PX: f32 = 10.0;
const KEYS_PANEL_MIN_WIDTH_PX: f32 = 220.0;

// ---- lines ------------------------------------------------------------------
// between the title's icon and its words
const TITLE_GAP_PX: f32 = 6.0;
// at least this much between a tool's name and its key
const KEY_LINE_GAP_PX: f32 = 16.0;
// between the hint's keycap and its words
const HINT_GAP_PX: f32 = 8.0;

// ==== Plugin =================================================================

/// A small window on F1 listing every dev key, and while it is closed, F1 itself in its
/// place, so no key has to be remembered.
pub(super) fn plugin(app: &mut App) {
    register_action(
        app,
        Action {
            name: KEYS_PANEL,
            default: Binding::Key(KEYS_PANEL_KEY),
            page: Page::Dev,
            label: Some(KEYS_PANEL_LABEL),
        },
    );
    app.add_systems(PostStartup, spawn_keys_panel_hint);
    app.add_systems(
        Update,
        toggle_keys_panel
            .run_if(action_just_pressed(KEYS_PANEL))
            .run_if(not(in_state(Pause::Photo))),
    );
}

// ==== Components =============================================================

/// Marks the root of the open keys panel.
#[derive(Component)]
struct KeysPanel;

/// Marks the hint that waits in the panel's place while it is closed: the panel's own key,
/// so nobody has to remember even that one.
#[derive(Component)]
struct KeysPanelHint;

// ==== Systems ================================================================

/// Spawns the hint: the panel's key on a keycap, beside the panel's title. After `Startup`,
/// so the key is the one the settings file bound. Drawn by the screen camera, as Bevy's FPS
/// overlay is, not on the dev tools' layer: an open layer blurs every frame, and the hint has
/// no glass to blur for. So the pause menu blurs it, like the FPS.
fn spawn_keys_panel_hint(mut commands: Commands, bindings: Res<Bindings>) {
    let Some(binding) = bindings.binding(KEYS_PANEL) else {
        return;
    };
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(KEYS_PANEL_TOP_PX),
                left: px(KEYS_PANEL_LEFT_PX),
                align_items: AlignItems::Center,
                column_gap: px(HINT_GAP_PX),
                ..default()
            },
            // over every screen root the screen camera draws
            DEV_TOOLS_Z_INDEX,
            KeysPanelHint,
            DevUi,
            children![
                keycap(KEYS_PANEL, binding_legend(binding)),
                (Text::new(KEYS_PANEL_TITLE), muted_text(SMALL_FONT_SIZE_PX)),
            ],
        ))
        .insert_recursive::<Children>(Pickable::IGNORE);
}

/// Opens the keys panel in the hint's place, or closes it and brings the hint back. It lists
/// every dev action but its own, in the order they were registered, each beside the key it is
/// bound to now; a key changed on Settings' Dev page while it is open changes on it too
/// (`paint_slots`).
fn toggle_keys_panel(
    mut commands: Commands,
    open: Option<Single<Entity, With<KeysPanel>>>,
    mut hints: Query<&mut Node, With<KeysPanelHint>>,
    bindings: Res<Bindings>,
) {
    let opening = open.is_none();
    for mut hint in &mut hints {
        hint.display = if opening {
            Display::None
        } else {
            Display::Flex
        };
    }
    if let Some(panel) = open {
        commands.entity(*panel).despawn();
        return;
    }
    // collected, since the lines are spawned after this borrow of the bindings ends
    let lines: Vec<_> = bindings
        .listed(Page::Dev)
        .filter(|(action, _, _)| *action != KEYS_PANEL)
        .map(|(action, words, binding)| key_line(action, words, binding))
        .collect();
    let window = (
        dev_window(Node {
            top: px(KEYS_PANEL_TOP_PX),
            left: px(KEYS_PANEL_LEFT_PX),
            min_width: px(KEYS_PANEL_MIN_WIDTH_PX),
            ..default()
        }),
        Children::spawn((Spawn(title()), SpawnIter(lines.into_iter()))),
    );
    spawn_dev_window(&mut commands, KeysPanel, window);
}

// ==== Helpers ================================================================

/// Returns the panel's title: a keyboard and the words, quieter than the lines under them.
fn title() -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: px(TITLE_GAP_PX),
            ..default()
        },
        children![
            icon(Icon::Keyboard),
            (Text::new(KEYS_PANEL_TITLE), muted_text(SMALL_FONT_SIZE_PX)),
        ],
    )
}

/// Returns one line of the panel: a tool's name on the left, its key's keycap on the right.
fn key_line(action: ActionName, words: &'static str, binding: Binding) -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: px(KEY_LINE_GAP_PX),
            ..default()
        },
        children![
            (Text::new(words), font(SMALL_FONT_SIZE_PX), TextColor(TEXT)),
            keycap(action, binding_legend(binding)),
        ],
    )
}
