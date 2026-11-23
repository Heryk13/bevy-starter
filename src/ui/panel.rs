use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use super::blur::{Floating, GlassLayers, GlassMaterial};
use super::{BORDER_PX, BUTTON_GAP_PX, ButtonKind, button, label};
use crate::fade::Fade;

// ==== Config =================================================================

// laid over the blur, under a card, so the card stands out from what it floats over
pub(super) const SCREEN_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

// ---- cards ------------------------------------------------------------------
const CARD_PADDING_PX: f32 = 32.0;
pub(crate) const CARD_GAP_PX: f32 = 16.0;
const CARD_RADIUS_PX: f32 = 26.0;
const CARD_SHADOW_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
const CARD_SHADOW_OFFSET_PX: f32 = 12.0;
const CARD_SHADOW_BLUR_PX: f32 = 40.0;

// ---- card rim: light on its top left edge, shade on its bottom right --------
// what makes a flat sheet read as glass with a thickness. Four stops, since the light dies
// out well before the middle and the shade only comes at the far corner
const CARD_RIM_LIGHT: Color = Color::srgba(1.0, 1.0, 1.0, 0.62);
const CARD_RIM_FADING: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);
const CARD_RIM_FAINT: Color = Color::srgba(1.0, 1.0, 1.0, 0.03);
const CARD_RIM_SHADE: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);
const CARD_RIM_FADING_STOP_PERCENT: f32 = 38.0;
const CARD_RIM_FAINT_STOP_PERCENT: f32 = 62.0;

// ---- entrance: a card rises into place as its glass comes in ----------------
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
    app.add_observer(dismiss_dialog);
    app.init_resource::<GlassLayers>();
    app.world_mut()
        .resource_mut::<GlassLayers>()
        .0
        .push(DIALOG_Z_INDEX);
}

// ==== Widgets ================================================================

/// Returns the root of something that floats over the screens: the whole window, its
/// panel centered, glass coming in behind it, and blocking clicks to whatever is under it.
pub(crate) fn floating_root(z_index: GlobalZIndex) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        Entrance::default(),
        Floating,
        z_index,
    )
}

/// Returns a glass card of the given width, the panel every overlay, the pause menu and
/// every dialog is drawn on. It goes straight under a `floating_root`, which brings it in,
/// and its fill is the glass `blur.rs` gives every `Card` from that root's layer: anywhere
/// else it would have none.
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
        card_glass(),
    )
}

/// Returns what makes a node a sheet of glass: the lit rim, the shadow under it, the place
/// its entrance starts from, and the `Card` marker that brings its glass. The node brings
/// its own size, padding, corner and border, which the rim is drawn on: without a border the
/// rim does not show.
pub(crate) fn card_glass() -> impl Bundle {
    (
        BorderGradient::from(LinearGradient::to_bottom_right(vec![
            ColorStop::new(CARD_RIM_LIGHT, percent(0)),
            ColorStop::new(CARD_RIM_FADING, percent(CARD_RIM_FADING_STOP_PERCENT)),
            ColorStop::new(CARD_RIM_FAINT, percent(CARD_RIM_FAINT_STOP_PERCENT)),
            ColorStop::new(CARD_RIM_SHADE, percent(100)),
        ])),
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
        ClosesOnEsc,
        children![(
            card(DIALOG_WIDTH_PX),
            children![
                (label(question), TextLayout::justify(Justify::Center)),
                (
                    Node {
                        justify_content: JustifyContent::Center,
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

/// Returns a modal notice with OK alone, ready to spawn over whatever it tells about.
/// `message` is a translation key.
pub(crate) fn notice(message: &'static str) -> impl Bundle {
    (
        floating_root(DIALOG_Z_INDEX),
        Dialog,
        ClosesOnEsc,
        children![(
            card(DIALOG_WIDTH_PX),
            children![
                (label(message), TextLayout::justify(Justify::Center)),
                (
                    Node {
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    children![(button("dialog-ok", ButtonKind::Primary), CloseDialog)],
                ),
            ],
        )],
    )
}

// ==== Events =================================================================

/// Asks the modal at `entity`, one carrying `ClosesOnEsc`, to close, as Esc does. Each closes
/// its own way: a dialog at once, a drawer by sliding away.
#[derive(EntityEvent)]
pub(crate) struct Dismiss {
    pub(crate) entity: Entity,
}

// ==== Components =============================================================

/// Marks the root of an open dialog, so either of its answers can close it.
#[derive(Component)]
struct Dialog;

/// Marks the root of a modal that Esc closes before anything under it: a dialog, or a dev
/// tool's window over the running game. Esc sends the topmost one, the highest
/// `GlobalZIndex`, a `Dismiss`, one per press, and whatever carries this answers it.
#[derive(Component)]
pub(crate) struct ClosesOnEsc;

/// Marks both dialog buttons: either answer closes the dialog.
#[derive(Component)]
struct CloseDialog;

/// Marks a card, so its root's entrance can find it and the blur can give it its glass.
#[derive(Component)]
pub(super) struct Card;

/// How long a `floating_root` has been coming in, in real seconds.
#[derive(Component, Default)]
pub(super) struct Entrance {
    secs: f32,
}

impl Entrance {
    /// Tells whether the root is fully in, so its entrance no longer changes the picture.
    pub(super) fn is_finished(&self) -> bool {
        self.secs > ENTRANCE.fade_in_secs
    }
}

// ==== Systems ================================================================

/// Brings every new `floating_root` in: its glass and its card's fade in together and the card
/// rises into place. Counts real time, so the pause menu comes in although it freezes virtual
/// time as it opens.
fn animate_entrances(
    time: Res<Time<Real>>,
    mut roots: Query<(
        &mut Entrance,
        Option<&MaterialNode<GlassMaterial>>,
        &Children,
    )>,
    mut cards: Query<(&mut UiTransform, Option<&MaterialNode<GlassMaterial>>), With<Card>>,
    mut materials: ResMut<Assets<GlassMaterial>>,
) {
    for (mut entrance, root_glass, children) in &mut roots {
        if entrance.is_finished() {
            continue;
        }
        entrance.secs += time.delta_secs();
        let progress = ENTRANCE.alpha(entrance.secs);
        if let Some(mut material) = root_glass.and_then(|glass| materials.get_mut(&glass.0)) {
            material.alpha = progress;
        }
        for child in children {
            let Ok((mut transform, card_glass)) = cards.get_mut(*child) else {
                continue;
            };
            *transform = entrance_transform(progress);
            if let Some(mut material) = card_glass.and_then(|glass| materials.get_mut(&glass.0)) {
                material.alpha = progress;
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
) {
    if close_buttons.contains(activate.entity) {
        commands.despawn_all::<With<Dialog>>();
    }
}

/// Despawns a dialog that Esc dismisses, as No would.
fn dismiss_dialog(dismiss: On<Dismiss>, mut commands: Commands, dialogs: Query<(), With<Dialog>>) {
    if dialogs.contains(dismiss.entity) {
        commands.entity(dismiss.entity).despawn();
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
