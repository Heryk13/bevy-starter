use bevy::prelude::*;

use crate::i18n::Localized;

pub(crate) mod blur;
mod button;
mod panel;

pub(crate) use button::{ButtonKind, button, menu_button, row_button};
pub(crate) use panel::{Dialog, card, dialog, floating_root};

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

// ---- type -------------------------------------------------------------------
pub(crate) const TITLE_FONT_SIZE_PX: f32 = 44.0;
pub(crate) const HEADING_FONT_SIZE_PX: f32 = 26.0;
pub(crate) const BODY_FONT_SIZE_PX: f32 = 18.0;
pub(crate) const SMALL_FONT_SIZE_PX: f32 = 14.0;
const HEADING_GAP_PX: f32 = 12.0;

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
