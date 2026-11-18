use bevy::picking::hover::{HoverMap, Hovered};
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::{Button, Checkbox, RadioButton, Slider};
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};

use super::{ACCENT, DANGER, INK, Icon, TEXT, icon, label};
use crate::fade::approach;

// ==== Config =================================================================

// ---- colors: the palette lives in ui.rs, these only paint buttons -----------
// the faint fill of a button that waits for the pointer to light up
const BUTTON_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.10);
const QUIET_FILL_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.18);
// white rather than `Color::NONE`, so easing toward the hover fill never passes through grey
const CLEAR: Color = Color::srgba(1.0, 1.0, 1.0, 0.0);
// a Primary is the accent already at rest, so the pointer can only dim it
const PRIMARY_HOVER_ALPHA: f32 = 0.84;
// the ink on a fill of `DANGER`, where the dark ink of a white fill would not read
const DANGER_INK: Color = Color::WHITE;
// a close button's red under the pointer: half of it, a warning rather than a Delete
#[cfg(feature = "dev")]
const CLOSE_HOVER_ALPHA: f32 = 0.5;
// how far pressing darkens whatever fill a button has, 0.0 to 1.0
const BUTTON_PRESS_SHADE: f32 = 0.18;

// ---- size -------------------------------------------------------------------
pub(crate) const BUTTON_HEIGHT_PX: f32 = 44.0;
const BUTTON_MIN_WIDTH_PX: f32 = 120.0;
pub(crate) const BUTTON_PADDING_PX: f32 = 20.0;
pub(crate) const ROW_BUTTON_PADDING_PX: f32 = 8.0;
// the corner of what a row starts with, such as a save's picture
pub(crate) const ROW_CONTENT_RADIUS_PX: f32 = 10.0;

// ---- easing -----------------------------------------------------------------
// how long a button takes to light up or go back, so a pointer passing over a column
// leaves a short trail instead of flickering
pub(crate) const BUTTON_EASE_SECS: f32 = 0.12;

// ---- derived ----------------------------------------------------------------
// a row's corner runs around its content's at the padding's distance, so the two
// curves stay parallel
const ROW_BUTTON_RADIUS_PX: f32 = ROW_BUTTON_PADDING_PX + ROW_CONTENT_RADIUS_PX;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    // no run_if in this file: a button lives on any screen and inside any overlay, so
    // there is no state these systems could be gated on
    app.add_systems(Startup, load_button_sounds);
    app.add_systems(
        Update,
        (paint_buttons, play_button_sounds, point_at_pressables),
    );
}

// ==== Widgets ================================================================

/// Returns a button labeled with a translation key, its label centered.
pub(crate) fn button(key: &'static str, kind: ButtonKind) -> impl Bundle {
    button_base(
        Node {
            min_width: px(BUTTON_MIN_WIDTH_PX),
            height: px(BUTTON_HEIGHT_PX),
            padding: UiRect::horizontal(px(BUTTON_PADDING_PX)),
            justify_content: JustifyContent::Center,
            ..button_frame()
        },
        kind,
        children![label(key)],
    )
}

/// Returns a button labeled with plain words instead of a translation key, its label
/// centered: for a dev tool, whose words are English and never translated. Only dev builds
/// have dev tools, so only they have this.
#[cfg(feature = "dev")]
pub(crate) fn words_button(words: &'static str, kind: ButtonKind) -> impl Bundle {
    button_base(
        Node {
            min_width: px(BUTTON_MIN_WIDTH_PX),
            height: px(BUTTON_HEIGHT_PX),
            padding: UiRect::horizontal(px(BUTTON_PADDING_PX)),
            justify_content: JustifyContent::Center,
            ..button_frame()
        },
        kind,
        children![(
            Text::new(words),
            super::font(super::BODY_FONT_SIZE_PX),
            TextColor(TEXT),
        )],
    )
}

/// Returns a Choice button for a column of choices, like the pause menu's: its label to the
/// left, and as wide as the column it is in.
pub(crate) fn menu_button(key: &'static str) -> impl Bundle {
    button_base(
        Node {
            height: px(BUTTON_HEIGHT_PX),
            padding: UiRect::horizontal(px(BUTTON_PADDING_PX)),
            justify_content: JustifyContent::Start,
            ..button_frame()
        },
        ButtonKind::Choice,
        children![label(key)],
    )
}

/// Returns a Choice button around `content`, as tall as the content needs: a row of a list,
/// such as a save with its picture beside its name.
pub(crate) fn row_button(content: impl Bundle) -> impl Bundle {
    button_base(
        Node {
            flex_grow: 1.0,
            // as close on the left as above and below, so what a row starts with, a save's
            // picture, sits evenly in its corner; a button's own padding on the right
            padding: UiRect {
                right: px(BUTTON_PADDING_PX),
                ..UiRect::all(px(ROW_BUTTON_PADDING_PX))
            },
            justify_content: JustifyContent::Start,
            // a corner of its own: a row is tall, and a capsule's round ends would eat into
            // what it holds
            border_radius: BorderRadius::all(px(ROW_BUTTON_RADIUS_PX)),
            ..button_frame()
        },
        ButtonKind::Choice,
        children![content],
    )
}

/// Returns a round button holding one icon and no word, as wide as a button is tall: what
/// stands in for a word a row has no room for, like a save's Delete. An icon says nothing to
/// a screen reader, so its caller adds an `AccessibleLabel`.
pub(crate) fn icon_button(symbol: Icon, kind: ButtonKind) -> impl Bundle {
    button_base(
        Node {
            width: px(BUTTON_HEIGHT_PX),
            height: px(BUTTON_HEIGHT_PX),
            justify_content: JustifyContent::Center,
            // beside a row that grows, it would be squeezed into an oval
            flex_shrink: 0.0,
            ..button_frame()
        },
        kind,
        children![icon(symbol)],
    )
}

/// Returns a button laid out by its caller, `node` its size, shape and layout, around
/// `content`: for a button none of the others draws, such as a dev tool's icon with its name
/// under it, both inside the fill. Only dev builds have one.
#[cfg(feature = "dev")]
pub(crate) fn laid_out_button(node: Node, kind: ButtonKind, content: impl Bundle) -> impl Bundle {
    button_base(node, kind, children![content])
}

// ==== Components =============================================================

/// What a button is for, which decides the fill and the ink it is painted in.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ButtonKind {
    /// The main action, white already at rest: New Save, a Yes that destroys nothing.
    Primary,
    /// One among equals, faint at rest and white under the pointer: a row of the pause menu
    /// or of a list.
    Choice,
    /// The way out, bare until the pointer comes: Close, No.
    Quiet,
    /// What destroys, in red ink at rest and filled red under the pointer: Delete, and a Yes
    /// that destroys something.
    Danger,
    /// A window's close button: bare at rest, as Quiet is, and a see-through red under the
    /// pointer. Only a dev tool's window has one, so only dev builds have it: a variant
    /// nothing builds fails `just check`.
    #[cfg(feature = "dev")]
    Close,
}

/// Marks a text under a button that keeps its own color: one that lies on a surface of its
/// own, not on the button's fill, so the ink that reads on the fill would not read on it.
#[derive(Component)]
pub(crate) struct OwnColor;

/// How far a button has eased toward hovered and toward pressed, each 0.0 to 1.0.
#[derive(Component, Default)]
struct ButtonMotion {
    hover: f32,
    press: f32,
}

// ==== Resources ==============================================================

/// Holds the two sounds every button in the app plays.
#[derive(Resource)]
struct ButtonSounds {
    click: Handle<AudioSource>,
    hover: Handle<AudioSource>,
}

// ==== Systems ================================================================

/// Loads the button sounds once, so playing one never waits on the asset server.
fn load_button_sounds(mut commands: Commands, asset_server: Res<AssetServer>) {
    let button_sounds = ButtonSounds {
        click: asset_server.load("ui/click.wav"),
        hover: asset_server.load("ui/hover.wav"),
    };

    commands.insert_resource(button_sounds);
}

/// Eases every button toward its hover and press state and paints it from there: its fill,
/// and its ink on every text it holds. Counts real time, since the pause menu's buttons live
/// while virtual time is frozen.
fn paint_buttons(
    time: Res<Time<Real>>,
    mut buttons: Query<
        (
            Entity,
            Ref<ButtonKind>,
            &Hovered,
            Has<Pressed>,
            &mut ButtonMotion,
            &mut BackgroundColor,
        ),
        With<Button>,
    >,
    children: Query<&Children>,
    mut inks: Query<&mut TextColor, Without<OwnColor>>,
) {
    let step = time.delta_secs() / BUTTON_EASE_SECS;
    for (entity, kind, hovered, pressed, mut motion, mut fill) in &mut buttons {
        let hover_target = f32::from(u8::from(hovered.get()));
        let press_target = f32::from(u8::from(pressed));
        // a button resting where it was last painted is left alone: most buttons, most
        // frames. a new one is painted once, since its texts spawn in a color of their own,
        // whatever the ink of its kind; and one whose kind changed, as a row of a list does
        // when it is selected, is painted in its new kind at once, not at the next hover. so a
        // text added under a button later would wait for the next hover to get its ink:
        // nothing does that, every button spawns whole
        let resting = motion.hover == hover_target && motion.press == press_target;
        if !motion.is_added() && !kind.is_changed() && resting {
            continue;
        }
        motion.hover = approach(motion.hover, hover_target, step);
        motion.press = approach(motion.press, press_target, step);
        let colors = button_colors(*kind, motion.hover, motion.press);
        fill.0 = colors.fill;
        for descendant in children.iter_descendants(entity) {
            if let Ok(mut ink) = inks.get_mut(descendant) {
                // the ink's color at the text's own alpha, so muted text stays muted
                // whichever ink it is in
                ink.0 = colors.ink.with_alpha(ink.0.alpha());
            }
        }
    }
}

/// Shows a pointing hand while the mouse is over anything that can be pressed, anywhere in
/// the game: a button, a switch, a choice among a few, a slider, or what carries `Hovered` to
/// light up under the pointer, such as a row of the file explorer's tree. The window's own
/// cursor comes back once it leaves. Writes only when that changes.
fn point_at_pressables(
    mut commands: Commands,
    hover_map: Res<HoverMap>,
    pressables: Query<
        (),
        Or<(
            With<Button>,
            With<Checkbox>,
            With<RadioButton>,
            With<Slider>,
            With<Hovered>,
        )>,
    >,
    parents: Query<&ChildOf>,
    window: Single<Entity, With<PrimaryWindow>>,
    mut pointing: Local<bool>,
) {
    // what the pointer hit, or any node holding it: a text in a button hits the text
    let over_pressable = hover_map.get(&PointerId::Mouse).is_some_and(|hits| {
        hits.keys().any(|&hit| {
            pressables.contains(hit)
                || parents
                    .iter_ancestors(hit)
                    .any(|node| pressables.contains(node))
        })
    });
    if over_pressable == *pointing {
        return;
    }
    *pointing = over_pressable;
    let cursor = if over_pressable {
        SystemCursorIcon::Pointer
    } else {
        SystemCursorIcon::Default
    };
    commands.entity(*window).insert(CursorIcon::System(cursor));
}

/// Plays the tick when a pointer enters a button, and the click when one is pressed.
fn play_button_sounds(
    hovered_buttons: Query<&Hovered, (Changed<Hovered>, With<Button>)>,
    pressed_buttons: Query<(), (Added<Pressed>, With<Button>)>,
    sounds: Res<ButtonSounds>,
    mut commands: Commands,
) {
    // ---- hover: the filter catches leaving and spawning too, hence the if ---
    for hovered in &hovered_buttons {
        if hovered.get() {
            commands.spawn((
                AudioPlayer::new(sounds.hover.clone()),
                PlaybackSettings::DESPAWN,
            ));
        }
    }

    // ---- press: being in this list already means it happened this frame -----
    for _ in &pressed_buttons {
        commands.spawn((
            AudioPlayer::new(sounds.click.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}

// ==== Helpers ================================================================

/// The colors a button is painted in at one moment: what fills it, and the ink of the texts
/// it holds.
#[derive(Debug, PartialEq)]
struct ButtonColors {
    fill: Color,
    ink: Color,
}

/// Returns the colors of a button of `kind`, `hover` and `press` of the way (0.0 to 1.0)
/// toward hovered and pressed. Pressing darkens the fill, whichever it is, and leaves it as
/// see-through as it was.
fn button_colors(kind: ButtonKind, hover: f32, press: f32) -> ButtonColors {
    let (rest_fill, rest_ink, hover_fill, hover_ink) = match kind {
        ButtonKind::Primary => (ACCENT, INK, ACCENT.with_alpha(PRIMARY_HOVER_ALPHA), INK),
        ButtonKind::Choice => (BUTTON_FILL, TEXT, ACCENT, INK),
        ButtonKind::Quiet => (CLEAR, TEXT, QUIET_FILL_HOVER, TEXT),
        ButtonKind::Danger => (BUTTON_FILL, DANGER, DANGER, DANGER_INK),
        #[cfg(feature = "dev")]
        ButtonKind::Close => (
            CLEAR,
            TEXT,
            DANGER.with_alpha(CLOSE_HOVER_ALPHA),
            DANGER_INK,
        ),
    };
    let fill = rest_fill.mix(&hover_fill, hover);
    let pressed_fill = Color::BLACK.with_alpha(fill.alpha());
    ButtonColors {
        fill: fill.mix(&pressed_fill, BUTTON_PRESS_SHADE * press),
        ink: rest_ink.mix(&hover_ink, hover),
    }
}

/// Returns the layout every button shares: a capsule, its label in the middle row. Each kind
/// of button fills in its own size and alignment.
fn button_frame() -> Node {
    Node {
        align_items: AlignItems::Center,
        border_radius: BorderRadius::MAX,
        ..default()
    }
}

/// Returns a button of `kind` with the given layout and children, its fill painted at rest.
/// `paint_buttons` gives its texts their ink the first time it sees it.
fn button_base(node: Node, kind: ButtonKind, children: impl Bundle) -> impl Bundle {
    (
        node,
        Button,
        kind,
        ButtonMotion::default(),
        BackgroundColor(button_colors(kind, 0.0, 0.0).fill),
        Hovered::default(),
        children,
    )
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;
    // a slice, not an array: dev builds have one kind more
    const KINDS: &[ButtonKind] = &[
        ButtonKind::Primary,
        ButtonKind::Choice,
        ButtonKind::Quiet,
        ButtonKind::Danger,
        #[cfg(feature = "dev")]
        ButtonKind::Close,
    ];

    /// Returns whether two colors match channel by channel, within the tolerance.
    fn same_color(a: Color, b: Color) -> bool {
        let (a, b) = (a.to_srgba(), b.to_srgba());
        [
            a.red - b.red,
            a.green - b.green,
            a.blue - b.blue,
            a.alpha - b.alpha,
        ]
        .iter()
        .all(|difference| difference.abs() < TOLERANCE)
    }

    /// Returns whether a fill is white and solid enough to need the dark ink over it.
    fn is_white_fill(fill: Color) -> bool {
        let fill = fill.to_srgba();
        same_color(fill.with_alpha(1.0).into(), Color::WHITE) && fill.alpha > 0.5
    }

    // ---- button_colors ------------------------------------------------------

    #[test]
    fn primary_button_is_white_already_at_rest() {
        let colors = button_colors(ButtonKind::Primary, 0.0, 0.0);
        assert!(same_color(colors.fill, ACCENT));
        assert!(same_color(colors.ink, INK));
    }

    #[test]
    fn choice_button_hovered_turns_white_with_dark_ink() {
        let colors = button_colors(ButtonKind::Choice, 1.0, 0.0);
        assert!(same_color(colors.fill, ACCENT));
        assert!(same_color(colors.ink, INK));
    }

    #[test]
    fn quiet_button_at_rest_draws_nothing() {
        let colors = button_colors(ButtonKind::Quiet, 0.0, 0.0);
        assert!(colors.fill.alpha() < TOLERANCE);
    }

    #[test]
    fn danger_button_hovered_fills_red_with_white_ink() {
        let colors = button_colors(ButtonKind::Danger, 1.0, 0.0);
        assert!(same_color(colors.fill, DANGER));
        assert!(same_color(colors.ink, DANGER_INK));
    }

    #[cfg(feature = "dev")]
    #[test]
    fn close_button_draws_nothing_at_rest_and_turns_half_red_hovered() {
        assert!(button_colors(ButtonKind::Close, 0.0, 0.0).fill.alpha() < TOLERANCE);
        let hovered = button_colors(ButtonKind::Close, 1.0, 0.0);
        assert!(same_color(
            hovered.fill,
            DANGER.with_alpha(CLOSE_HOVER_ALPHA)
        ));
        assert!(same_color(hovered.ink, DANGER_INK));
    }

    #[test]
    fn pressed_button_darkens_its_fill_and_stays_as_see_through() {
        for &kind in KINDS {
            let hovered = button_colors(kind, 1.0, 0.0).fill.to_srgba();
            let pressed = button_colors(kind, 1.0, 1.0).fill.to_srgba();
            assert!(pressed.red < hovered.red, "{kind:?}");
            assert!(
                (pressed.alpha - hovered.alpha).abs() < TOLERANCE,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn ink_is_dark_exactly_where_the_fill_is_white() {
        for &kind in KINDS {
            for hover in [0.0, 1.0] {
                let colors = button_colors(kind, hover, 0.0);
                assert_eq!(
                    is_white_fill(colors.fill),
                    same_color(colors.ink, INK),
                    "{kind:?} at hover {hover}"
                );
            }
        }
    }
}
