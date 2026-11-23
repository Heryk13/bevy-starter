use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::ui_widgets::{Activate, Button};

use crate::input::Controls;

// ==== Config =================================================================

// ---- the left stick: one push is one step -----------------------------------
// pushed this far it moves the focus once, and must come back this near the middle before it
// moves it again
const STICK_PUSH: f32 = 0.6;
const STICK_RELEASE: f32 = 0.3;

// ---- choosing the next button -----------------------------------------------
// a button off to the side costs this many times its distance straight ahead, so the
// d-pad goes to the button in line before a nearer one beside it
const SIDEWAYS_WEIGHT: f32 = 2.0;

// ==== Plugin =================================================================

/// Moving through the menus with a gamepad: the d-pad or the left stick moves the focus from
/// button to button, A presses the focused one, and B goes back, as Esc does (the pause action,
/// `overlays.rs`). Only the buttons of the topmost layer are reached, the highest
/// `GlobalZIndex`, so a dialog never lets the focus slip to the menu under it. The focused
/// button is lit as under the pointer (`button.rs`).
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        // piped, since `Controls` reads the focus this one writes, and a pipe runs them in turn
        (
            read_left_stick.pipe(move_focus_by_pad),
            press_focused_by_pad,
        )
            .chain(),
    );
}

// ==== Systems ================================================================

/// Returns where the left sticks point, the one pushed furthest: what moves the focus beside
/// the d-pad.
fn read_left_stick(controls: Controls) -> Vec2 {
    controls.stick(Gamepad::left_stick)
}

/// Moves the focus to the next button the way the d-pad or the left stick is pushed. With the
/// focus shown and on no button of the topmost layer, as when a menu opens, it goes to that
/// layer's first button.
fn move_focus_by_pad(
    In(stick): In<Vec2>,
    gamepads: Query<&Gamepad>,
    mut stick_pushed: Local<bool>,
    buttons: Query<
        (
            Entity,
            &UiGlobalTransform,
            &ComputedNode,
            &InheritedVisibility,
        ),
        With<Button>,
    >,
    parents: Query<&ChildOf>,
    z_indices: Query<&GlobalZIndex>,
    mut focus: ResMut<InputFocus>,
    mut focus_visible: ResMut<InputFocusVisible>,
) {
    let direction = pad_direction(&gamepads, stick, &mut stick_pushed);
    if direction.is_none() && !focus_visible.0 {
        return;
    }
    let reachable = topmost_buttons(&buttons, &parents, &z_indices);
    let Some(first) = first_button(&reachable) else {
        return;
    };
    let current = focus
        .get()
        .and_then(|focused| reachable.iter().find(|(entity, _)| *entity == focused));
    let next = match (current, direction) {
        (Some(&(_, at)), Some(direction)) => next_in_direction(at, direction, &reachable),
        (Some(_), None) => None,
        (None, _) => Some(first),
    };
    if let Some(next) = next {
        focus.set(next, FocusCause::Navigated);
    }
    if direction.is_some() {
        focus_visible.0 = true;
    }
}

/// Presses the focused button when A is pressed, if it is one the d-pad can reach.
fn press_focused_by_pad(
    mut commands: Commands,
    gamepads: Query<&Gamepad>,
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    buttons: Query<(), With<Button>>,
) {
    let pressed = gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::South));
    let Some(focused) = focus.get() else {
        return;
    };
    if pressed && focus_visible.0 && buttons.contains(focused) {
        commands.trigger(Activate { entity: focused });
    }
}

// ==== Helpers ================================================================

/// Returns the way a d-pad was just pressed, or a left stick just pushed, in the UI's
/// directions (y grows down), or none.
fn pad_direction(gamepads: &Query<&Gamepad>, stick: Vec2, stick_pushed: &mut bool) -> Option<Vec2> {
    const PRESSES: [(GamepadButton, Vec2); 4] = [
        (GamepadButton::DPadUp, Vec2::NEG_Y),
        (GamepadButton::DPadDown, Vec2::Y),
        (GamepadButton::DPadLeft, Vec2::NEG_X),
        (GamepadButton::DPadRight, Vec2::X),
    ];
    for pad in gamepads {
        for (button, direction) in PRESSES {
            if pad.just_pressed(button) {
                return Some(direction);
            }
        }
    }
    let (direction, pushed) = stick_step(stick, *stick_pushed);
    *stick_pushed = pushed;
    direction
}

/// Returns the step a stick at `stick` (y up) takes, and whether it now counts as pushed:
/// one step as it crosses `STICK_PUSH`, along its main axis, and none again until it comes
/// back inside `STICK_RELEASE`.
fn stick_step(stick: Vec2, was_pushed: bool) -> (Option<Vec2>, bool) {
    let length = stick.length();
    if was_pushed {
        return (None, length > STICK_RELEASE);
    }
    if length < STICK_PUSH {
        return (None, false);
    }
    let direction = if stick.x.abs() > stick.y.abs() {
        Vec2::new(stick.x.signum(), 0.0)
    } else {
        // the stick's up is the screen's up, and the UI's y grows down
        Vec2::new(0.0, -stick.y.signum())
    };
    (Some(direction), true)
}

/// Returns the shown buttons of the topmost layer, each with its centre: a layer is the
/// `GlobalZIndex` of a button's nearest ancestor that has one, and the screen's own buttons,
/// under none, are layer 0.
fn topmost_buttons(
    buttons: &Query<
        (
            Entity,
            &UiGlobalTransform,
            &ComputedNode,
            &InheritedVisibility,
        ),
        With<Button>,
    >,
    parents: &Query<&ChildOf>,
    z_indices: &Query<&GlobalZIndex>,
) -> Vec<(Entity, Vec2)> {
    let layer_of = |entity: Entity| {
        parents
            .iter_ancestors(entity)
            .find_map(|ancestor| z_indices.get(ancestor).ok())
            .map_or(0, |z| z.0)
    };
    let shown: Vec<(Entity, Vec2, i32)> = buttons
        .iter()
        // a button under a hidden node has no size, and an invisible one is not seen
        .filter(|(_, _, node, visibility)| visibility.get() && node.size() != Vec2::ZERO)
        .map(|(entity, transform, _, _)| (entity, transform.affine().translation, layer_of(entity)))
        .collect();
    let Some(top) = shown.iter().map(|(_, _, layer)| *layer).max() else {
        return Vec::new();
    };
    shown
        .into_iter()
        .filter(|(_, _, layer)| *layer == top)
        .map(|(entity, centre, _)| (entity, centre))
        .collect()
}

/// Returns the button a menu starts on: the highest, and of those the leftmost.
fn first_button(buttons: &[(Entity, Vec2)]) -> Option<Entity> {
    buttons
        .iter()
        .min_by(|(_, a), (_, b)| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)))
        .map(|(entity, _)| *entity)
}

/// Returns the button nearest to `from` the way of `direction`, a button off to the side
/// counting `SIDEWAYS_WEIGHT` times its distance, or none when no button lies that way.
fn next_in_direction(from: Vec2, direction: Vec2, buttons: &[(Entity, Vec2)]) -> Option<Entity> {
    buttons
        .iter()
        .filter_map(|(entity, centre)| {
            let offset = *centre - from;
            let along = offset.dot(direction);
            // ahead by more than a rounding, so the focused button itself is never next
            if along <= 1.0 {
                return None;
            }
            let sideways = (offset - direction * along).length();
            Some((along + SIDEWAYS_WEIGHT * sideways, *entity))
        })
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .map(|(_, entity)| entity)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn buttons() -> Vec<(Entity, Vec2)> {
        // a column of three, and one to the right of the middle
        vec![
            (Entity::from_raw_u32(1).unwrap(), Vec2::new(100.0, 100.0)),
            (Entity::from_raw_u32(2).unwrap(), Vec2::new(100.0, 200.0)),
            (Entity::from_raw_u32(3).unwrap(), Vec2::new(100.0, 300.0)),
            (Entity::from_raw_u32(4).unwrap(), Vec2::new(300.0, 210.0)),
        ]
    }

    #[test]
    fn down_goes_to_the_button_below_not_the_nearer_one_beside() {
        let buttons = buttons();
        let next = next_in_direction(buttons[0].1, Vec2::Y, &buttons);
        assert_eq!(next, Some(buttons[1].0));
    }

    #[test]
    fn right_goes_to_the_button_on_the_right() {
        let buttons = buttons();
        let next = next_in_direction(buttons[1].1, Vec2::X, &buttons);
        assert_eq!(next, Some(buttons[3].0));
    }

    #[test]
    fn nothing_lies_beyond_the_last_button() {
        let buttons = buttons();
        assert_eq!(next_in_direction(buttons[2].1, Vec2::Y, &buttons), None);
    }

    #[test]
    fn a_menu_starts_on_its_top_button() {
        assert_eq!(first_button(&buttons()), Some(buttons()[0].0));
    }

    #[test]
    fn a_stick_pushed_and_held_takes_one_step() {
        let push = Vec2::new(0.0, (STICK_PUSH + 1.0) / 2.0);
        let (first, pushed) = stick_step(push, false);
        assert_eq!(first, Some(Vec2::NEG_Y));
        let (held, still_pushed) = stick_step(push, pushed);
        assert_eq!(held, None);
        assert!(still_pushed);
        let (_, released) = stick_step(Vec2::ZERO, still_pushed);
        assert!(!released);
    }
}
