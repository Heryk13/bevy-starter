use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::FileExplorer;
use crate::ui::{ClosesOnEsc, Icon, TEXT_MUTED, custom_icon};

// ==== Config =================================================================

// ---- height: a part of the screen's, which the grip changes -----------------
// a third of the screen and a little more at first: the game stays in view above it, and the
// list still shows a handful of rows. The grip resizes it between the two limits, and the
// explorer keeps that height until the game closes
const DRAWER_START_HEIGHT_PERCENT: f32 = 38.0;
const DRAWER_MIN_HEIGHT_PERCENT: f32 = 20.0;
const DRAWER_MAX_HEIGHT_PERCENT: f32 = 85.0;

// ---- slide: in from under the screen, and back out --------------------------
const SLIDE_SECS: f32 = 0.25;
// what it slides past the screen's edge beyond its own height to be out of view: its margin,
// and its shadow
const DRAWER_HIDDEN_EXTRA_PERCENT: f32 = 6.0;

// ---- grip: the lines over the path that resize the drawer -------------------
// wide and tall enough to catch without aiming
const GRIP_WIDTH_PX: f32 = 120.0;
const GRIP_HEIGHT_PX: f32 = 18.0;
const GRIP_ICON_PX: f32 = 16.0;

// ==== Plugin =================================================================

/// The explorer as a drawer: it slides up from under the screen and back down, and its grip
/// resizes it, the height kept while the game runs.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<DrawerHeight>();
    app.add_systems(
        Update,
        (
            slide_file_explorer,
            resize_drawer.run_if(resource_changed::<DrawerHeight>),
        ),
    );
    // no run_if: the explorer opens over any screen
    app.add_observer(resize_by_grip);
}

// ==== Widgets ================================================================

/// Returns the grip, for the middle of the title bar: a few lines in a strip wide enough to
/// catch, which say it can be dragged. The lines let the pointer through to the grip, so a
/// drag starts on it.
pub(super) fn resize_grip() -> impl Bundle {
    (
        Node {
            width: px(GRIP_WIDTH_PX),
            height: px(GRIP_HEIGHT_PX),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ResizeGrip,
        Hovered::default(),
        children![(
            custom_icon(Icon::GripHorizontal, GRIP_ICON_PX, TEXT_MUTED),
            Pickable::IGNORE,
        )],
    )
}

/// Returns where the explorer sits `shown` of the way up, from 0.0, below the screen, to
/// 1.0, in place, eased so it slows as it arrives. `height_percent` is the drawer's height,
/// which it slides past to be out of view.
pub(super) fn slide_transform(shown: f32, height_percent: f32) -> UiTransform {
    let hidden = 1.0 - EaseFunction::CubicOut.sample_clamped(shown);
    let out_of_view = height_percent + DRAWER_HIDDEN_EXTRA_PERCENT;
    UiTransform {
        // a percentage of the root, which is the whole screen
        translation: Val2::percent(0.0, out_of_view * hidden),
        ..UiTransform::IDENTITY
    }
}

// ==== Components =============================================================

/// Marks the explorer's window, the drawer, whose height the grip sets.
#[derive(Component)]
pub(super) struct ExplorerDrawer;

/// Marks the grip over the path, dragged up or down to resize the drawer.
#[derive(Component)]
struct ResizeGrip;

// ==== Resources ==============================================================

/// How tall the drawer is, as a percentage of the screen's height: where the grip left it,
/// kept while the game runs, so the explorer opens the size it was.
#[derive(Resource)]
pub(super) struct DrawerHeight(pub(super) f32);

impl Default for DrawerHeight {
    fn default() -> Self {
        Self(DRAWER_START_HEIGHT_PERCENT)
    }
}

// ==== Systems ================================================================

/// Slides the explorer up from under the screen as it opens, and back down as it closes,
/// counting real time so it moves over a frozen game too, and despawns it once it is down.
fn slide_file_explorer(
    mut commands: Commands,
    time: Res<Time<Real>>,
    height: Res<DrawerHeight>,
    mut explorers: Query<(
        Entity,
        &mut FileExplorer,
        &mut UiTransform,
        Has<ClosesOnEsc>,
    )>,
) {
    for (entity, mut explorer, mut transform, is_modal) in &mut explorers {
        // once closing it is no longer a modal: the next Esc pauses, and the cursor comes back
        // to the game, without waiting for the slide to end
        if explorer.closing && is_modal {
            commands.entity(entity).remove::<ClosesOnEsc>();
        }
        let step = time.delta_secs() / SLIDE_SECS;
        explorer.shown = if explorer.closing {
            (explorer.shown - step).max(0.0)
        } else {
            (explorer.shown + step).min(1.0)
        };
        if explorer.closing && explorer.shown <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let wanted = slide_transform(explorer.shown, height.0);
        if *transform != wanted {
            *transform = wanted;
        }
    }
}

/// Sets the drawer's height when the grip has moved it.
fn resize_drawer(height: Res<DrawerHeight>, mut drawers: Query<&mut Node, With<ExplorerDrawer>>) {
    for mut node in &mut drawers {
        node.height = percent(height.0);
    }
}

/// Resizes the drawer as its grip is dragged: up makes it taller, down shorter, between the
/// limits.
fn resize_by_grip(
    drag: On<PointerDrag>,
    grips: Query<(), With<ResizeGrip>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut height: ResMut<DrawerHeight>,
) {
    let (true, Ok(window)) = (grips.contains(drag.entity), windows.single()) else {
        return;
    };
    let resized = dragged_height(height.0, drag.delta.y, window.height());
    if height.0 != resized {
        height.0 = resized;
    }
}

// ==== Helpers ================================================================

/// Returns the drawer's height after its grip moves `delta_y_px` down a window
/// `window_height_px` tall: dragging up, a negative `delta_y_px`, makes it taller. Held
/// between the limits, so it can neither vanish nor hide the whole game.
fn dragged_height(height_percent: f32, delta_y_px: f32, window_height_px: f32) -> f32 {
    let moved_percent = delta_y_px / window_height_px.max(1.0) * 100.0;
    (height_percent - moved_percent).clamp(DRAWER_MIN_HEIGHT_PERCENT, DRAWER_MAX_HEIGHT_PERCENT)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-4;
    const WINDOW_HEIGHT_PX: f32 = 1000.0;

    #[test]
    fn dragging_the_grip_up_makes_the_drawer_taller() {
        let height = dragged_height(DRAWER_START_HEIGHT_PERCENT, -100.0, WINDOW_HEIGHT_PX);
        assert!((height - (DRAWER_START_HEIGHT_PERCENT + 10.0)).abs() < TOLERANCE);
    }

    #[test]
    fn the_drawer_never_shrinks_past_its_smallest() {
        let height = dragged_height(
            DRAWER_MIN_HEIGHT_PERCENT,
            WINDOW_HEIGHT_PX,
            WINDOW_HEIGHT_PX,
        );
        assert!((height - DRAWER_MIN_HEIGHT_PERCENT).abs() < TOLERANCE);
    }

    #[test]
    fn the_drawer_never_grows_past_its_tallest() {
        let height = dragged_height(
            DRAWER_MAX_HEIGHT_PERCENT,
            -WINDOW_HEIGHT_PX,
            WINDOW_HEIGHT_PX,
        );
        assert!((height - DRAWER_MAX_HEIGHT_PERCENT).abs() < TOLERANCE);
    }
}
