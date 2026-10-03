use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use super::blur::Floating;
use super::{BORDER, BORDER_PX, BUTTON_GAP_PX, ButtonKind, SURFACE, button, label};
use crate::fade::Fade;

// ==== Config =================================================================

// laid over the blur, under a card, so the card stands out from what it floats over
const SCREEN_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

// ---- cards ------------------------------------------------------------------
const CARD_PADDING_PX: f32 = 32.0;
const CARD_GAP_PX: f32 = 16.0;
const CARD_RADIUS_PX: f32 = 16.0;
const CARD_SHADOW_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
const CARD_SHADOW_OFFSET_PX: f32 = 12.0;
const CARD_SHADOW_BLUR_PX: f32 = 40.0;

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
    // no run_if: a floating root comes in, and a dialog closes, over any screen or overlay
    app.add_systems(Update, animate_entrances);
    app.add_observer(close_dialog);
}

// ==== Widgets ================================================================

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

// ==== Systems ================================================================

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

/// Returns where a card is `progress` of the way into its entrance: lowered and a little
/// small at 0.0, exactly in place at 1.0.
fn entrance_transform(progress: f32) -> UiTransform {
    UiTransform {
        translation: Val2::px(0.0, ENTRANCE_RISE_PX * (1.0 - progress)),
        scale: Vec2::splat(ENTRANCE_START_SCALE + (1.0 - ENTRANCE_START_SCALE) * progress),
        ..UiTransform::IDENTITY
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

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
