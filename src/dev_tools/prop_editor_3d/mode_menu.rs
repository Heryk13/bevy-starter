use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::PrimaryWindow;

use super::super::prop_editor::PropEditor;
use super::super::prop_editor::edits::{Held, HoveredProp, MapEdit, PropsSheet};
use super::super::{DEV_TOOLS_Z_INDEX, DevUi, dev_window};
use crate::screens::gameplay::map::{Prop, PropMode};
use crate::ui::{ButtonKind, ClosesOnEsc, Dismiss, floating_root, words_button};

// ==== Config =================================================================

// ---- a quick right click, told from holding it to turn the camera -----------
const RIGHT_CLICK_MAX_SECS: f32 = 0.25;
// the mouse barely moved: a turn moves it far more
const RIGHT_CLICK_MAX_MOVE_PX: f32 = 6.0;

// ---- words: English, as every dev tool's, never read in a release build -----
const FIXED_WORDS: &str = "Fixed";
const PHYSICS_WORDS: &str = "Physics";
const NO_COLLISION_WORDS: &str = "No Collision";

// ---- menu: over the props window, as the explorer's menu is over the explorer
const MENU_Z_INDEX: GlobalZIndex = GlobalZIndex(DEV_TOOLS_Z_INDEX.0 + 10);

// ==== Plugin =================================================================

/// A placed prop's mode, chosen from a menu a quick right click on it opens: Fixed, Physics or
/// No Collision, the current one the primary button. Holding the right button turns the
/// camera instead, as in Unreal.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (open_mode_menu, close_mode_menu_on_click_outside).run_if(in_state(PropEditor::On)),
    );
    app.add_observer(choose_mode.run_if(in_state(PropEditor::On)));
    app.add_observer(dismiss_mode_menu.run_if(in_state(PropEditor::On)));
}

// ==== Components =============================================================

/// Marks the root of the mode menu, with the prop it is for.
#[derive(Component)]
struct ModeMenu(Entity);

/// Marks the mode menu's sheet, whose `Hovered` tells a click on it from one outside.
#[derive(Component)]
struct ModeMenuSheet;

/// A mode menu item, with the mode it gives.
#[derive(Component)]
struct ModeItem(PropMode);

// ==== Systems ================================================================

/// Opens the mode menu at the cursor on a quick right click on a placed prop: the right button
/// let go soon after it went down, the mouse barely moved. The prop is the one under the
/// cursor as the button went down, since the cursor is held while it is down.
fn open_mode_menu(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    time: Res<Time<Real>>,
    window: Single<&Window, With<PrimaryWindow>>,
    hovered: Res<HoveredProp>,
    held: Query<(), With<Held>>,
    props: Query<&PropMode, With<Prop>>,
    menus: Query<Entity, With<ModeMenu>>,
    mut press: Local<Option<RightPress>>,
) {
    if mouse.just_pressed(MouseButton::Right) {
        *press = Some(RightPress {
            prop: hovered.0,
            at: window.cursor_position(),
            secs: 0.0,
            moved_px: 0.0,
        });
        return;
    }
    if mouse.pressed(MouseButton::Right) {
        if let Some(press) = press.as_mut() {
            press.secs += time.delta_secs();
            press.moved_px += motion.delta.length();
        }
        return;
    }
    if !mouse.just_released(MouseButton::Right) {
        return;
    }
    let Some(press) = press.take() else {
        return;
    };
    let (Some(prop), Some(at)) = (press.prop, press.at) else {
        return;
    };
    let Ok(mode) = props.get(prop) else {
        return;
    };
    if !is_quick_click(press.secs, press.moved_px) || !held.is_empty() {
        return;
    }
    for menu in &menus {
        commands.entity(menu).despawn();
    }
    let item = |words, item_mode| {
        let kind = if item_mode == *mode {
            ButtonKind::Primary
        } else {
            ButtonKind::Choice
        };
        (words_button(words, kind), ModeItem(item_mode))
    };
    commands.spawn((
        floating_root(MENU_Z_INDEX),
        Pickable::IGNORE,
        DevUi,
        ClosesOnEsc,
        ModeMenu(prop),
        DespawnOnExit(PropEditor::On),
        children![(
            dev_window(Node {
                left: px(at.x),
                top: px(at.y),
                ..default()
            }),
            ModeMenuSheet,
            // a sheet of the editor's, so a click on an item never reaches the prop behind it
            PropsSheet,
            Hovered::default(),
            children![
                item(FIXED_WORDS, PropMode::Fixed),
                item(PHYSICS_WORDS, PropMode::Physics),
                item(NO_COLLISION_WORDS, PropMode::NoCollision),
            ],
        )],
    ));
}

/// Closes the mode menu when a press lands outside it. That press goes on to what is under
/// it, as in the OS's own menus.
fn close_mode_menu_on_click_outside(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    sheets: Query<&Hovered, With<ModeMenuSheet>>,
    menus: Query<Entity, With<ModeMenu>>,
) {
    if !mouse.just_pressed(MouseButton::Left) || sheets.iter().any(Hovered::get) {
        return;
    }
    for menu in &menus {
        commands.entity(menu).despawn();
    }
}

/// Gives the prop the mode pressed, keeping where it stands, and closes the menu.
fn choose_mode(
    activate: On<Activate>,
    mut commands: Commands,
    items: Query<&ModeItem>,
    menus: Query<(Entity, &ModeMenu)>,
    props: Query<(&Prop, &Transform)>,
) {
    let Ok(item) = items.get(activate.entity) else {
        return;
    };
    for (menu, for_prop) in &menus {
        if let Ok((prop, transform)) = props.get(for_prop.0) {
            commands.trigger(MapEdit::Update {
                id: prop.0,
                transform: *transform,
                mode: item.0,
            });
        }
        commands.entity(menu).despawn();
    }
}

/// Closes the mode menu when Esc dismisses it, before the editor's window.
fn dismiss_mode_menu(
    dismiss: On<Dismiss>,
    mut commands: Commands,
    menus: Query<(), With<ModeMenu>>,
) {
    if menus.contains(dismiss.entity) {
        commands.entity(dismiss.entity).despawn();
    }
}

// ==== Helpers ================================================================

/// A right press under way: the prop under the cursor as it went down, where the cursor was,
/// how long it has been down and how far the mouse moved since.
struct RightPress {
    prop: Option<Entity>,
    at: Option<Vec2>,
    secs: f32,
    moved_px: f32,
}

/// Tells a quick right click, which opens the menu, from a hold that turned the camera.
fn is_quick_click(secs: f32, moved_px: f32) -> bool {
    secs < RIGHT_CLICK_MAX_SECS && moved_px < RIGHT_CLICK_MAX_MOVE_PX
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quick_still_press_is_a_click() {
        assert!(is_quick_click(
            RIGHT_CLICK_MAX_SECS / 2.0,
            RIGHT_CLICK_MAX_MOVE_PX / 2.0
        ));
    }

    #[test]
    fn a_long_press_turns_the_camera() {
        assert!(!is_quick_click(RIGHT_CLICK_MAX_SECS * 2.0, 0.0));
    }

    #[test]
    fn a_press_that_moves_the_mouse_turns_the_camera() {
        assert!(!is_quick_click(0.0, RIGHT_CLICK_MAX_MOVE_PX * 2.0));
    }
}
