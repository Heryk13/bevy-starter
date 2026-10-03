use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::{Activate, Button};

use crate::blur::Floating;
use crate::fade::{Fade, approach};
use crate::i18n::Localized;

// ==== Config =================================================================

// ---- palette: the one place to recolor the whole game -----------------------
pub(crate) const SURFACE: Color = Color::srgba_u8(0x16, 0x17, 0x1B, 217);
pub(crate) const BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);
pub(crate) const ACCENT: Color = Color::srgb_u8(0xF2, 0xB5, 0x44);
pub(crate) const DANGER: Color = Color::srgb_u8(0xE0, 0x5C, 0x5C);
pub(crate) const TEXT: Color = Color::srgb_u8(0xED, 0xED, 0xF0);
pub(crate) const TEXT_MUTED: Color = Color::srgb_u8(0x9E, 0xA0, 0xA8);
// a field the player types in, and a control that is off
pub(crate) const FIELD_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.06);
const BUTTON_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.04);
const BUTTON_FILL_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.09);
const BUTTON_FILL_PRESSED: Color = Color::srgba(0.0, 0.0, 0.0, 0.2);
// white rather than `Color::NONE`, so easing toward the hover fill never passes through grey
const CLEAR: Color = Color::srgba(1.0, 1.0, 1.0, 0.0);
// laid over the blur, under a card, so the card stands out from what it floats over
const SCREEN_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

// ---- type -------------------------------------------------------------------
pub(crate) const TITLE_FONT_SIZE_PX: f32 = 44.0;
pub(crate) const HEADING_FONT_SIZE_PX: f32 = 26.0;
pub(crate) const BODY_FONT_SIZE_PX: f32 = 18.0;
pub(crate) const SMALL_FONT_SIZE_PX: f32 = 14.0;

// ---- buttons ----------------------------------------------------------------
const BUTTON_HEIGHT_PX: f32 = 44.0;
const BUTTON_MIN_WIDTH_PX: f32 = 120.0;
const BUTTON_PADDING_PX: f32 = 20.0;
const ROW_BUTTON_PADDING_PX: f32 = 8.0;
// between buttons stacked in a column or set side by side: the menu, the pause menu, a dialog
pub(crate) const BUTTON_GAP_PX: f32 = 10.0;
pub(crate) const BORDER_PX: f32 = 1.0;
// the glow is a shadow the color of the border, soft enough to read as light
const BUTTON_GLOW_BLUR_PX: f32 = 18.0;
const BUTTON_GLOW_ALPHA: f32 = 0.35;
// how long a button takes to light up or go back, so a pointer passing over a column
// leaves a short trail instead of flickering
const BUTTON_EASE_SECS: f32 = 0.12;

// ---- cards ------------------------------------------------------------------
const CARD_PADDING_PX: f32 = 32.0;
const CARD_GAP_PX: f32 = 16.0;
const CARD_RADIUS_PX: f32 = 16.0;
const CARD_SHADOW_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
const CARD_SHADOW_OFFSET_PX: f32 = 12.0;
const CARD_SHADOW_BLUR_PX: f32 = 40.0;
const HEADING_GAP_PX: f32 = 12.0;

// ---- entrance: a card rises into place as its dim comes in ------------------
// held forever once in, so it is a fade that only ever comes in
const ENTRANCE: Fade = Fade {
    delay_secs: 0.0,
    fade_in_secs: 0.18,
    hold_secs: f32::INFINITY,
    fade_out_secs: 0.0,
};
const ENTRANCE_RISE_PX: f32 = 12.0;
const ENTRANCE_START_SCALE: f32 = 0.96;

// ---- dialog: drawn above the overlays, to ask over any of them --------------
const DIALOG_Z_INDEX: GlobalZIndex = GlobalZIndex(200);
const DIALOG_WIDTH_PX: f32 = 400.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    // no run_if in this file: a button lives on any screen and inside any overlay, so
    // there is no state these systems could be gated on
    app.add_systems(Startup, load_button_sounds);
    app.add_systems(
        Update,
        (paint_buttons, play_button_sounds, animate_entrances),
    );
    app.add_observer(close_dialog);
}

// ==== Widgets ================================================================

// ---- text -------------------------------------------------------------------

/// Returns the font for text of the given size, in the game's one face.
pub(crate) fn font(size_px: f32) -> TextFont {
    TextFont::from_font_size(size_px)
}

/// Returns body text showing a translation key.
pub(crate) fn label(key: &'static str) -> impl Bundle {
    (Localized(key), font(BODY_FONT_SIZE_PX), TextColor(TEXT))
}

/// Returns a card's title over a thin divider.
pub(crate) fn heading(key: &'static str) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(HEADING_GAP_PX),
            ..default()
        },
        children![
            (Localized(key), font(HEADING_FONT_SIZE_PX), TextColor(TEXT)),
            (
                Node {
                    height: px(BORDER_PX),
                    ..default()
                },
                BackgroundColor(BORDER),
            ),
        ],
    )
}

// ---- buttons ----------------------------------------------------------------

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

/// Returns a Primary button for a column of choices, like the menu's: its label to the left,
/// and as wide as the column it is in.
pub(crate) fn menu_button(key: &'static str) -> impl Bundle {
    button_base(
        Node {
            height: px(BUTTON_HEIGHT_PX),
            padding: UiRect::horizontal(px(BUTTON_PADDING_PX)),
            justify_content: JustifyContent::Start,
            ..button_frame()
        },
        ButtonKind::Primary,
        children![label(key)],
    )
}

/// Returns a Primary button around `content`, as tall as the content needs: a row of a list,
/// such as a save with its name over its details.
pub(crate) fn row_button(content: impl Bundle) -> impl Bundle {
    button_base(
        Node {
            flex_grow: 1.0,
            padding: UiRect::axes(px(BUTTON_PADDING_PX), px(ROW_BUTTON_PADDING_PX)),
            justify_content: JustifyContent::Start,
            ..button_frame()
        },
        ButtonKind::Primary,
        children![content],
    )
}

// ---- cards ------------------------------------------------------------------

/// Returns the root of something that floats over the screens: the whole window, its
/// panel centered, a dim coming in behind it, and blocking clicks to whatever is under it.
pub(crate) fn floating_root(z_index: GlobalZIndex) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(SCREEN_DIM.with_alpha(0.0)),
        Entrance::default(),
        Floating,
        z_index,
    )
}

/// Returns a glass card of the given width, the panel every overlay, the pause menu and
/// every dialog is drawn on. It goes straight under a `floating_root`, which brings it in.
pub(crate) fn card(width_px: f32) -> impl Bundle {
    (
        Node {
            width: px(width_px),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(CARD_PADDING_PX)),
            row_gap: px(CARD_GAP_PX),
            border: UiRect::all(px(BORDER_PX)),
            border_radius: BorderRadius::all(px(CARD_RADIUS_PX)),
            ..default()
        },
        BackgroundColor(SURFACE),
        BorderColor::all(BORDER),
        BoxShadow::new(
            CARD_SHADOW_COLOR,
            px(0),
            px(CARD_SHADOW_OFFSET_PX),
            px(0),
            px(CARD_SHADOW_BLUR_PX),
        ),
        entrance_transform(0.0),
        Card,
    )
}

/// Returns a modal question with Yes and No, ready to spawn over whatever asked it.
/// `question` is a translation key, and `yes_kind` the kind of its Yes: `Danger` when
/// answering Yes destroys something. The Yes button also carries `on_yes`, the caller's
/// marker for what Yes does.
pub(crate) fn dialog(
    question: &'static str,
    yes_kind: ButtonKind,
    on_yes: impl Bundle,
) -> impl Bundle {
    (
        floating_root(DIALOG_Z_INDEX),
        Dialog,
        children![(
            card(DIALOG_WIDTH_PX),
            children![
                (label(question), TextLayout::justify(Justify::Center)),
                (
                    Node {
                        justify_content: JustifyContent::End,
                        column_gap: px(BUTTON_GAP_PX),
                        ..default()
                    },
                    children![
                        (button("dialog-no", ButtonKind::Quiet), CloseDialog),
                        (button("dialog-yes", yes_kind), CloseDialog, on_yes),
                    ],
                ),
            ],
        )],
    )
}

// ==== Components =============================================================

/// How a button looks: which colors it lights up in.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ButtonKind {
    /// Bordered at rest, amber under the pointer: the usual choice.
    Primary,
    /// Borderless until the pointer comes, for the way out: Close, No.
    Quiet,
    /// Like Primary, but red under the pointer: Delete, and a Yes that destroys something.
    Danger,
}

/// How far a button has eased toward hovered and toward pressed, each 0.0 to 1.0.
#[derive(Component, Default)]
struct ButtonMotion {
    hover: f32,
    press: f32,
}

/// Marks the root of an open dialog, so it can be found and closed.
#[derive(Component)]
pub(crate) struct Dialog;

/// Marks both dialog buttons: either answer closes the dialog.
#[derive(Component)]
struct CloseDialog;

/// Marks a card, so its root's entrance can find it.
#[derive(Component)]
struct Card;

/// How long a `floating_root` has been coming in, in real seconds.
#[derive(Component, Default)]
struct Entrance {
    secs: f32,
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

/// Eases every button toward its hover and press state and paints it from there. Counts
/// real time, since the pause menu's buttons live while virtual time is frozen.
fn paint_buttons(
    time: Res<Time<Real>>,
    mut buttons: Query<
        (
            &ButtonKind,
            &Hovered,
            Has<Pressed>,
            &mut ButtonMotion,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut BoxShadow,
        ),
        With<Button>,
    >,
) {
    let step = time.delta_secs() / BUTTON_EASE_SECS;
    for (kind, hovered, pressed, mut motion, mut fill, mut border, mut shadow) in &mut buttons {
        motion.hover = approach(motion.hover, f32::from(u8::from(hovered.get())), step);
        motion.press = approach(motion.press, f32::from(u8::from(pressed)), step);
        let colors = button_colors(*kind, motion.hover, motion.press);
        fill.0 = colors.fill;
        *border = BorderColor::all(colors.border);
        if let Some(glow) = shadow.0.first_mut() {
            glow.color = colors.glow;
        }
    }
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

/// Brings every new `floating_root` in: its dim darkens and its card rises into place. Counts
/// real time, so the pause menu comes in although it freezes virtual time as it opens.
fn animate_entrances(
    time: Res<Time<Real>>,
    mut roots: Query<(&mut Entrance, &mut BackgroundColor, &Children)>,
    mut cards: Query<&mut UiTransform, With<Card>>,
) {
    for (mut entrance, mut dim, children) in &mut roots {
        if entrance.secs > ENTRANCE.fade_in_secs {
            continue;
        }
        entrance.secs += time.delta_secs();
        let progress = ENTRANCE.alpha(entrance.secs);
        dim.0 = SCREEN_DIM.with_alpha(SCREEN_DIM.alpha() * progress);
        for child in children {
            if let Ok(mut transform) = cards.get_mut(*child) {
                *transform = entrance_transform(progress);
            }
        }
    }
}

/// Despawns the open dialog when either of its answers is pressed. What Yes does is up to
/// the caller's own observer, which sees the same press.
fn close_dialog(
    activate: On<Activate>,
    mut commands: Commands,
    close_buttons: Query<(), With<CloseDialog>>,
    dialogs: Query<Entity, With<Dialog>>,
) {
    if close_buttons.contains(activate.entity) {
        for dialog in &dialogs {
            commands.entity(dialog).despawn();
        }
    }
}

// ==== Helpers ================================================================

/// The colors a button is painted in at one moment.
#[derive(Debug, PartialEq)]
struct ButtonColors {
    fill: Color,
    border: Color,
    glow: Color,
}

/// Returns the colors of a button of `kind`, `hover` and `press` of the way (0.0 to 1.0)
/// toward hovered and pressed. Pressing darkens the fill and puts the glow out, hovered or
/// not.
fn button_colors(kind: ButtonKind, hover: f32, press: f32) -> ButtonColors {
    let (rest_fill, rest_border, hover_border) = match kind {
        ButtonKind::Primary => (BUTTON_FILL, BORDER, ACCENT),
        ButtonKind::Quiet => (CLEAR, CLEAR, BORDER),
        ButtonKind::Danger => (BUTTON_FILL, BORDER, DANGER),
    };
    let glow = match kind {
        ButtonKind::Quiet => CLEAR,
        _ => hover_border.with_alpha(BUTTON_GLOW_ALPHA * hover * (1.0 - press)),
    };
    ButtonColors {
        fill: rest_fill
            .mix(&BUTTON_FILL_HOVER, hover)
            .mix(&BUTTON_FILL_PRESSED, press),
        border: rest_border.mix(&hover_border, hover),
        glow,
    }
}

/// Returns where a card is `progress` of the way into its entrance: lowered and a little
/// small at 0.0, exactly in place at 1.0.
fn entrance_transform(progress: f32) -> UiTransform {
    UiTransform {
        translation: Val2::px(0.0, ENTRANCE_RISE_PX * (1.0 - progress)),
        scale: Vec2::splat(ENTRANCE_START_SCALE + (1.0 - ENTRANCE_START_SCALE) * progress),
        ..UiTransform::IDENTITY
    }
}

/// Returns the layout every button shares: a pill with a thin border, its label in the
/// middle row. Each kind of button fills in its own size and alignment.
fn button_frame() -> Node {
    Node {
        align_items: AlignItems::Center,
        border: UiRect::all(px(BORDER_PX)),
        border_radius: BorderRadius::MAX,
        ..default()
    }
}

/// Returns a button of `kind` with the given layout and children, painted at rest. Its glow
/// starts out, and `paint_buttons` brings it up with the hover.
fn button_base(node: Node, kind: ButtonKind, children: impl Bundle) -> impl Bundle {
    let rest = button_colors(kind, 0.0, 0.0);
    (
        node,
        Button,
        kind,
        ButtonMotion::default(),
        BackgroundColor(rest.fill),
        BorderColor::all(rest.border),
        BoxShadow::new(rest.glow, px(0), px(0), px(0), px(BUTTON_GLOW_BLUR_PX)),
        Hovered::default(),
        children,
    )
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

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

    // ---- button_colors ------------------------------------------------------

    #[test]
    fn primary_button_at_rest_has_a_thin_border_and_no_glow() {
        let colors = button_colors(ButtonKind::Primary, 0.0, 0.0);
        assert!(same_color(colors.fill, BUTTON_FILL));
        assert!(same_color(colors.border, BORDER));
        assert!(colors.glow.alpha() < TOLERANCE);
    }

    #[test]
    fn primary_button_hovered_lights_up_in_the_accent() {
        let colors = button_colors(ButtonKind::Primary, 1.0, 0.0);
        assert!(same_color(colors.fill, BUTTON_FILL_HOVER));
        assert!(same_color(colors.border, ACCENT));
        assert!((colors.glow.alpha() - BUTTON_GLOW_ALPHA).abs() < TOLERANCE);
    }

    #[test]
    fn pressed_button_darkens_and_puts_the_glow_out() {
        let colors = button_colors(ButtonKind::Primary, 1.0, 1.0);
        assert!(same_color(colors.fill, BUTTON_FILL_PRESSED));
        assert!(colors.glow.alpha() < TOLERANCE);
    }

    #[test]
    fn quiet_button_at_rest_draws_nothing() {
        let colors = button_colors(ButtonKind::Quiet, 0.0, 0.0);
        assert!(colors.fill.alpha() < TOLERANCE);
        assert!(colors.border.alpha() < TOLERANCE);
    }

    #[test]
    fn quiet_button_never_glows() {
        let colors = button_colors(ButtonKind::Quiet, 1.0, 0.0);
        assert!(colors.glow.alpha() < TOLERANCE);
    }

    #[test]
    fn danger_button_hovered_lights_up_red() {
        let colors = button_colors(ButtonKind::Danger, 1.0, 0.0);
        assert!(same_color(colors.border, DANGER));
    }

    // ---- entrance_transform -------------------------------------------------

    #[test]
    fn card_starts_its_entrance_lowered_and_small() {
        let transform = entrance_transform(0.0);
        assert_eq!(transform.translation, Val2::px(0.0, ENTRANCE_RISE_PX));
        assert!((transform.scale.x - ENTRANCE_START_SCALE).abs() < TOLERANCE);
    }

    #[test]
    fn card_ends_its_entrance_exactly_in_place() {
        let transform = entrance_transform(1.0);
        assert_eq!(transform.translation, Val2::px(0.0, 0.0));
        assert!((transform.scale.x - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn entrance_is_complete_once_its_time_is_up() {
        assert!((ENTRANCE.alpha(ENTRANCE.fade_in_secs * 2.0) - 1.0).abs() < TOLERANCE);
    }
}
