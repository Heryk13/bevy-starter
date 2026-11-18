use bevy::prelude::*;
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};

// ==== Config =================================================================

// ---- scrollbars: a thin, faint bar beside what scrolls ----------------------
const SCROLLBAR_WIDTH_PX: f32 = 4.0;
const SCROLLBAR_MIN_THUMB_PX: f32 = 24.0;
const SCROLLBAR_THUMB: Color = Color::srgba(1.0, 1.0, 1.0, 0.22);
// the room what scrolls leaves on its right, so the bar never lies over a row
const SCROLLBAR_GUTTER_PX: f32 = 10.0;
// between the rows of what scrolls
const ROW_GAP_PX: f32 = 2.0;

// ==== Plugin =================================================================

/// What scrolls in the explorer, the list and the tree: a frame around it, and a thin, faint
/// scrollbar laid over the frame's right edge, there only while there is more to see.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(add_scrollbar);
    app.add_systems(Update, hide_idle_scrollbars);
}

// ==== Widgets ================================================================

/// Returns the frame a scrolling node of the explorer sits in, which also holds its
/// scrollbar: it grows into the room it is given, and can be shorter than what it holds.
pub(super) fn scroll_frame() -> Node {
    Node {
        flex_grow: 1.0,
        min_height: px(0),
        ..default()
    }
}

/// Returns the node that scrolls inside a `scroll_frame`: a column filling the frame, with
/// room on its right for the scrollbar. It carries `ExplorerScroll` too, to get one.
pub(super) fn scrolling_column() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        row_gap: px(ROW_GAP_PX),
        padding: UiRect::right(px(SCROLLBAR_GUTTER_PX)),
        overflow: Overflow::scroll_y(),
        ..default()
    }
}

// ==== Components =============================================================

/// Marks a node of the explorer that scrolls, which gets a thin scrollbar beside it.
#[derive(Component)]
pub(super) struct ExplorerScroll;

/// Marks a scrollbar the explorer gave to one of its `ExplorerScroll` nodes.
#[derive(Component)]
struct ExplorerScrollbar;

// ==== Systems ================================================================

/// Gives a node of the explorer that scrolls its scrollbar: a thin bar laid over its right
/// edge, beside it rather than inside it, where it would scroll away with the rows. Bevy's
/// `Scrollbar` sizes and moves the thumb, and lets it be dragged.
fn add_scrollbar(add: On<Add<ExplorerScroll>>, mut commands: Commands, parents: Query<&ChildOf>) {
    let Ok(frame) = parents.get(add.entity) else {
        return;
    };
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            bottom: px(0),
            right: px(0),
            width: px(SCROLLBAR_WIDTH_PX),
            ..default()
        },
        Scrollbar::new(
            add.entity,
            ControlOrientation::Vertical,
            SCROLLBAR_MIN_THUMB_PX,
        ),
        ExplorerScrollbar,
        ChildOf(frame.parent()),
        children![(
            ScrollbarThumb {
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(SCROLLBAR_THUMB),
        )],
    ));
}

/// Hides each scrollbar while what it scrolls fits, so a short list shows no bar.
fn hide_idle_scrollbars(
    mut scrollbars: Query<(&Scrollbar, &mut Visibility), With<ExplorerScrollbar>>,
    nodes: Query<&ComputedNode>,
) {
    for (scrollbar, mut visibility) in &mut scrollbars {
        let Ok(target) = nodes.get(scrollbar.target) else {
            continue;
        };
        let wanted = if target.content_size.y > target.size.y {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}
