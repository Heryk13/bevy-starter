use bevy::asset::uuid_handle;
use bevy::prelude::*;
use lucide_icons::LUCIDE_FONT_BYTES;

use crate::i18n::Localized;

pub(crate) mod blur;
mod button;
mod panel;

pub(crate) use button::{
    BUTTON_EASE_SECS, BUTTON_HEIGHT_PX, ButtonKind, ROW_BUTTON_PADDING_PX, button, icon_button,
    menu_button, row_button,
};
// the icons by name, re-exported so this file is the only one that names the crate
pub(crate) use lucide_icons::Icon;
pub(crate) use panel::{Dialog, card, dialog, floating_root};

// ==== Config =================================================================

// ---- palette: the one place to recolor the whole game -----------------------
// thin enough for the blur behind a card to show through it: under Reduce Transparency the
// same color is drawn solid
pub(crate) const SURFACE: Color = Color::srgba_u8(0x2C, 0x2C, 0x32, 143);
// the hairline under a heading and around a field
pub(crate) const BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.16);
// white, since a hue is lost over glass, whose color is whatever lies behind it
pub(crate) const ACCENT: Color = Color::WHITE;
// text on a fill of the accent, where white text would vanish
pub(crate) const INK: Color = Color::srgb_u8(0x11, 0x11, 0x14);
pub(crate) const DANGER: Color = Color::srgb_u8(0xFF, 0x5F, 0x57);
// both texts are white let through in part, never a grey: a grey reads over one background
// and vanishes over another, and a card's background is the game
pub(crate) const TEXT: Color = Color::srgba(1.0, 1.0, 1.0, 0.92);
pub(crate) const TEXT_MUTED: Color = Color::srgba(1.0, 1.0, 1.0, 0.74);
// a field the player types in, and a control that is off
pub(crate) const FIELD_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);

// ---- type -------------------------------------------------------------------
pub(crate) const TITLE_FONT_SIZE_PX: f32 = 44.0;
pub(crate) const HEADING_FONT_SIZE_PX: f32 = 26.0;
pub(crate) const BODY_FONT_SIZE_PX: f32 = 18.0;
pub(crate) const SMALL_FONT_SIZE_PX: f32 = 14.0;
const HEADING_GAP_PX: f32 = 12.0;

// ---- muted text -------------------------------------------------------------
// a shadow under it: it is the faintest text there is, and what it sits on is whatever the
// game shows behind the glass. `TextShadow` has no blur, so it stays a pixel down and soft
pub(crate) const MUTED_SHADOW_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
const MUTED_SHADOW_OFFSET_PX: f32 = 1.0;

// ---- icons: Lucide's, drawn as text in its own font -------------------------
// the font comes built into the binary with the `lucide-icons` crate, and sits behind a
// handle known at compile time, so `icon()` is a plain function like `label()`, with no
// asset server to ask. here, although Load Game is the only screen with an icon so far:
// the icons are part of the look, and every tool to come draws from them
const ICON_FONT: Handle<Font> = uuid_handle!("62d3686c-8ac2-4ff3-a801-d7263c00dc3e");
// a little over the body text's size, which an icon sits beside: it has no letters'
// ascenders and descenders to fill its line with
const ICON_SIZE_PX: f32 = 22.0;

// ---- borders and gaps, shared by text, buttons and panels -------------------
// between buttons stacked in a column or set side by side: the menu, the pause menu, a dialog
pub(crate) const BUTTON_GAP_PX: f32 = 10.0;
pub(crate) const BORDER_PX: f32 = 1.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((blur::plugin, button::plugin, panel::plugin));
    app.add_systems(Startup, register_icon_font);
}

// ==== Widgets ================================================================

/// Returns the font for text of the given size, in the game's one face.
pub(crate) fn font(size_px: f32) -> TextFont {
    TextFont::from_font_size(size_px)
}

/// Returns body text showing a translation key.
pub(crate) fn label(key: &'static str) -> impl Bundle {
    (Localized(key), font(BODY_FONT_SIZE_PX), TextColor(TEXT))
}

/// Returns what makes a text muted: the font at the given size, the muted color and the
/// shadow that goes with it. The caller adds the `Text` or the `Localized` it shows.
pub(crate) fn muted_text(size_px: f32) -> impl Bundle {
    (
        font(size_px),
        TextColor(TEXT_MUTED),
        TextShadow {
            offset: Vec2::new(0.0, MUTED_SHADOW_OFFSET_PX),
            color: MUTED_SHADOW_COLOR,
        },
    )
}

/// Returns an icon, sized to sit beside body text. It is a text of one character in the
/// icon font, so whatever colors a text colors it: a button's ink reaches it as it reaches
/// a label.
pub(crate) fn icon(icon: Icon) -> impl Bundle {
    (
        Text::new(char::from(icon)),
        TextFont::from_font_size(ICON_SIZE_PX).with_font(ICON_FONT),
        TextColor(TEXT),
    )
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

// ==== Systems ================================================================

/// Puts the Lucide font behind `ICON_FONT`, from the bytes built into the binary, so every
/// icon is there from the first frame.
fn register_icon_font(mut fonts: ResMut<Assets<Font>>) {
    let font = Font::from_bytes(LUCIDE_FONT_BYTES.to_vec());
    if let Err(error) = fonts.insert(ICON_FONT.id(), font) {
        error!("could not register the icon font: {error}");
    }
}
