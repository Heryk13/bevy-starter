use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::Button;

use super::{ACCENT, BORDER, BORDER_PX, DANGER, label};
use crate::fade::approach;

// ==== Config =================================================================

// ---- colors: the palette lives in ui.rs, these only paint buttons -----------
const BUTTON_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.04);
const BUTTON_FILL_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.09);
const BUTTON_FILL_PRESSED: Color = Color::srgba(0.0, 0.0, 0.0, 0.2);
// white rather than `Color::NONE`, so easing toward the hover fill never passes through grey
const CLEAR: Color = Color::srgba(1.0, 1.0, 1.0, 0.0);

// ---- size -------------------------------------------------------------------
const BUTTON_HEIGHT_PX: f32 = 44.0;
const BUTTON_MIN_WIDTH_PX: f32 = 120.0;
const BUTTON_PADDING_PX: f32 = 20.0;
const ROW_BUTTON_PADDING_PX: f32 = 8.0;

// ---- glow and easing --------------------------------------------------------
// the glow is a shadow the color of the border, soft enough to read as light
const BUTTON_GLOW_BLUR_PX: f32 = 18.0;
const BUTTON_GLOW_ALPHA: f32 = 0.35;
// how long a button takes to light up or go back, so a pointer passing over a column
// leaves a short trail instead of flickering
const BUTTON_EASE_SECS: f32 = 0.12;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    // no run_if in this file: a button lives on any screen and inside any overlay, so
    // there is no state these systems could be gated on
    app.add_systems(Startup, load_button_sounds);
    app.add_systems(Update, (paint_buttons, play_button_sounds));
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
}
