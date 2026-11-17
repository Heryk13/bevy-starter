use bevy::prelude::*;

use crate::i18n::Localized;

pub(crate) mod blur;
mod button;
mod panel;

pub(crate) use button::{ButtonKind, button, menu_button, row_button};
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
const MUTED_SHADOW_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
const MUTED_SHADOW_OFFSET_PX: f32 = 1.0;

// ---- borders and gaps, shared by text, buttons and panels -------------------
// between buttons stacked in a column or set side by side: the menu, the pause menu, a dialog
pub(crate) const BUTTON_GAP_PX: f32 = 10.0;
pub(crate) const BORDER_PX: f32 = 1.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((blur::plugin, button::plugin, panel::plugin));
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
