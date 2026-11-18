use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button, ScrollArea};

use super::{SettingsFooter, SettingsPage, SettingsPageBody, SettingsPanel, setting_row};
use crate::i18n::Localized;
use crate::input::{ActionName, Binding, Bindings, Controls, Page};
use crate::overlays::Overlay;
use crate::ui::{
    ACCENT, ButtonKind, INK, SMALL_FONT_SIZE_PX, button, dialog, font, label, muted_text,
};
#[cfg(feature = "dev")]
use crate::ui::{BODY_FONT_SIZE_PX, TEXT};

// ==== Config =================================================================

// ---- translation keys -------------------------------------------------------
const RESET_QUESTION: &str = "dialog-reset-controls";

// ---- the list of actions ----------------------------------------------------
const ACTION_LIST_MAX_HEIGHT_PX: f32 = 260.0;
const ACTION_LIST_GAP_PX: f32 = 8.0;

// ---- slot: the button around a keycap, pressed to choose another key --------
const SLOT_MIN_WIDTH_PX: f32 = 132.0;
const SLOT_HEIGHT_PX: f32 = 42.0;
const SLOT_RADIUS_PX: f32 = 12.0;
// around the keycap; the slot is pulled left by as much, so the keycaps stay in line with
// the controls of the General page
const SLOT_PADDING_PX: f32 = 12.0;
const SLOT_FILL_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.18);

// ---- listening: a slot waiting for the key it will be bound to --------------
// the key that gives up on choosing a binding. Esc, which no action can take: Pause, the
// one bound to it, is not listed
const CANCEL_KEY: KeyCode = KeyCode::Escape;

// ---- keycap: a white face on a grey skirt, as a key seen from the front -----
const KEYCAP_SKIRT: Color = Color::srgb_u8(0xC3, 0xC3, 0xCB);
// how much of the skirt shows under the face: the key's height
const KEYCAP_SKIRT_PX: f32 = 5.0;
const KEYCAP_RADIUS_PX: f32 = 6.0;
// a single letter still makes a key as wide as it is tall
const KEYCAP_MIN_WIDTH_PX: f32 = 28.0;
const KEYCAP_PADDING_X_PX: f32 = 8.0;
const KEYCAP_PADDING_Y_PX: f32 = 5.0;
const KEYCAP_SHADOW_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);
const KEYCAP_SHADOW_OFFSET_PX: f32 = 4.0;
const KEYCAP_SHADOW_BLUR_PX: f32 = 8.0;

// ---- derived ----------------------------------------------------------------
// how far the face goes down while its key is held: the skirt less a sliver, so a held
// key still reads as a key
const KEYCAP_HELD_PX: f32 = KEYCAP_SKIRT_PX - 1.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(SettingsPage::Controls), spawn_controls_page);
    #[cfg(feature = "dev")]
    app.add_systems(OnEnter(SettingsPage::Dev), spawn_dev_page);
    app.add_observer(listen_for_binding.run_if(in_state(Overlay::Settings)));
    app.add_observer(ask_to_reset_controls.run_if(in_state(Overlay::Settings)));
    app.add_observer(reset_controls.run_if(in_state(Overlay::Settings)));
    app.add_systems(
        Update,
        (press_held_keycaps, paint_slots).run_if(in_state(Overlay::Settings)),
    );
    // after everything that reads an action this frame, which is in `Update` or before:
    // the key that binds, or the Esc that cancels, is taken here and so does nothing else
    app.add_systems(
        PostUpdate,
        take_pressed_binding.run_if(in_state(Overlay::Settings)),
    );
    // nothing closes the panel while a slot listens; should something ever, no action
    // stays deaf after it
    app.add_systems(OnExit(Overlay::Settings), stop_listening);
}

// ==== Components =============================================================

/// Marks the Reset to Defaults button of a page of controls.
#[derive(Component)]
struct ResetControls;

/// Marks the Yes of `RESET_QUESTION`.
#[derive(Component)]
struct ConfirmReset;

/// Marks the button around a keycap, with the action it rebinds when pressed.
#[derive(Component)]
struct BindingSlot(ActionName);

/// Marks the keycap in a slot, hidden while the slot listens.
#[derive(Component)]
struct SlotKeycap;

/// Marks the words a slot shows in place of its keycap while it listens.
#[derive(Component)]
struct SlotHint;

/// Marks the face of a keycap, with the action whose key it shows: the part that goes down
/// while that key is held.
#[derive(Component)]
struct KeycapFace(ActionName);

/// Marks the legend on a keycap, with the action whose binding it spells.
#[derive(Component)]
struct KeycapLegend(ActionName);

/// Marks the line under a list that says how to give up, shown while a slot listens.
#[derive(Component)]
struct CancelHint;

/// Marks the node laid over the whole window while a slot listens. It takes every press,
/// so a click that becomes a binding does not also press what is under the pointer.
#[derive(Component)]
struct ListeningShield;

// ==== Systems ================================================================

/// Fills the body with the Controls page: the game's actions, each beside its key.
fn spawn_controls_page(
    mut commands: Commands,
    body: Single<Entity, With<SettingsPageBody>>,
    footer: Single<Entity, With<SettingsFooter>>,
    bindings: Res<Bindings>,
) {
    let list = spawn_action_list(&mut commands, *body, *footer, SettingsPage::Controls);
    for (action, label_key, binding) in bindings.listed(Page::Game) {
        commands.spawn((
            setting_row(label(label_key), binding_slot(action, binding)),
            ChildOf(list),
        ));
    }
}

/// Fills the body with the Dev page: the dev tools' actions, each beside its key. Their
/// labels are English words, as every dev tool's text is.
#[cfg(feature = "dev")]
fn spawn_dev_page(
    mut commands: Commands,
    body: Single<Entity, With<SettingsPageBody>>,
    footer: Single<Entity, With<SettingsFooter>>,
    bindings: Res<Bindings>,
) {
    let list = spawn_action_list(&mut commands, *body, *footer, SettingsPage::Dev);
    for (action, words, binding) in bindings.listed(Page::Dev) {
        let name = (Text::new(words), font(BODY_FONT_SIZE_PX), TextColor(TEXT));
        commands.spawn((
            setting_row(name, binding_slot(action, binding)),
            ChildOf(list),
        ));
    }
}

/// Makes the slot that was pressed the one listening: its action waits for the next key or
/// mouse button, and until then no action hears its own.
fn listen_for_binding(
    activate: On<Activate>,
    mut commands: Commands,
    slots: Query<&BindingSlot>,
    panel: Single<Entity, With<SettingsPanel>>,
    mut bindings: ResMut<Bindings>,
) {
    if let Ok(BindingSlot(action)) = slots.get(activate.entity) {
        bindings.listen(*action);
        // spawned after the card, so it is drawn, and pressed, over it
        commands.spawn((
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                ..default()
            },
            ListeningShield,
            ChildOf(*panel),
        ));
    }
}

/// Sets the face of each keycap down on its skirt while its action's key is held, and back
/// up when it is let go: the player sees a binding work, and which key a legend names,
/// before closing the panel.
fn press_held_keycaps(controls: Controls, mut faces: Query<(&KeycapFace, &mut UiTransform)>) {
    for (KeycapFace(action), mut transform) in &mut faces {
        let pressed = keycap_face_transform(controls.pressed(*action));
        // most frames nothing is held and nothing moves
        if *transform != pressed {
            *transform = pressed;
        }
    }
}

/// Paints each slot: lit under the pointer, and while it listens, white with its words in
/// place of its keycap. Spells each keycap's legend from its action's binding, and shows
/// how to give up while any slot listens. Writes only what differs, which is little.
fn paint_slots(
    bindings: Res<Bindings>,
    mut slots: Query<(&BindingSlot, &Hovered, &mut BackgroundColor, &Children)>,
    mut parts: Query<
        (&mut Node, Has<SlotHint>),
        (Or<(With<SlotKeycap>, With<SlotHint>)>, Without<CancelHint>),
    >,
    mut legends: Query<(&KeycapLegend, &mut Text)>,
    mut cancel_hints: Query<&mut Node, With<CancelHint>>,
) {
    let listening = bindings.listening();
    for (BindingSlot(action), hovered, mut fill, children) in &mut slots {
        let listens = listening == Some(*action);
        let color = if listens {
            ACCENT
        } else if hovered.get() {
            SLOT_FILL_HOVER
        } else {
            Color::NONE
        };
        if fill.0 != color {
            fill.0 = color;
        }
        for child in children {
            if let Ok((mut node, is_hint)) = parts.get_mut(*child) {
                // the hint shows exactly while the slot listens, the keycap the rest of
                // the time
                let display = shown(is_hint == listens);
                if node.display != display {
                    node.display = display;
                }
            }
        }
    }
    for mut node in &mut cancel_hints {
        let display = shown(listening.is_some());
        if node.display != display {
            node.display = display;
        }
    }
    if bindings.is_changed() {
        for (KeycapLegend(action), mut text) in &mut legends {
            if let Some(binding) = bindings.binding(*action) {
                text.0 = binding_legend(binding);
            }
        }
    }
}

/// Gives the listening action the key or mouse button the player just pressed, or gives up
/// on Esc, and stops listening either way.
fn take_pressed_binding(
    mut commands: Commands,
    mut bindings: ResMut<Bindings>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let Some(action) = bindings.listening() else {
        return;
    };
    // not on the frame the slot started listening: whatever pressed it, a click or a key,
    // is the press that asked, not the answer
    if bindings.is_changed() {
        return;
    }
    let pressed = keys
        .get_just_pressed()
        .next()
        .map(|key| Binding::Key(*key))
        .or_else(|| {
            mouse
                .get_just_pressed()
                .next()
                .map(|button| Binding::Mouse(*button))
        });
    let binding = match heard(pressed) {
        Heard::Nothing => return,
        Heard::Cancel => None,
        Heard::Binding(binding) => Some(binding),
    };
    if let Some(binding) = binding {
        bindings.bind(action, binding);
    }
    bindings.stop_listening();
    commands.despawn_all::<With<ListeningShield>>();
}

/// Asks `RESET_QUESTION` when Reset to Defaults is pressed: the player may have spent a
/// while on their keys.
fn ask_to_reset_controls(
    activate: On<Activate>,
    mut commands: Commands,
    reset_buttons: Query<(), With<ResetControls>>,
) {
    if reset_buttons.contains(activate.entity) {
        commands.spawn((
            dialog(RESET_QUESTION, ButtonKind::Danger, ConfirmReset),
            DespawnOnExit(Overlay::Settings),
        ));
    }
}

/// Puts every action back on its default once `RESET_QUESTION` is answered Yes, those of
/// the other page included: a swap may have crossed the two, and one page reset alone
/// could leave two actions on one key.
fn reset_controls(
    activate: On<Activate>,
    confirms: Query<(), With<ConfirmReset>>,
    mut bindings: ResMut<Bindings>,
) {
    if confirms.contains(activate.entity) {
        bindings.reset();
    }
}

/// Stops any slot from listening as the panel closes.
fn stop_listening(mut bindings: ResMut<Bindings>) {
    bindings.stop_listening();
}

// ==== Helpers ================================================================

// ---- the page ---------------------------------------------------------------

/// Spawns what both pages of controls are made of, in the body, for as long as `page` is
/// shown: the scrolling column the rows of actions go in, which it returns, and under it
/// the line that says how to give up, hidden until a slot listens.
fn spawn_action_list(
    commands: &mut Commands,
    body: Entity,
    footer: Entity,
    page: SettingsPage,
) -> Entity {
    // Reset to Defaults goes in the footer, before Close, and takes all the room to its
    // right: it sits at the left of the row and leaves Close at the right
    let reset = commands
        .spawn((
            Node {
                margin: UiRect::right(Val::Auto),
                ..default()
            },
            children![(button("controls-reset", ButtonKind::Quiet), ResetControls)],
            DespawnOnExit(page),
        ))
        .id();
    commands.entity(footer).insert_children(0, &[reset]);
    let list = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(ACTION_LIST_GAP_PX),
                max_height: px(ACTION_LIST_MAX_HEIGHT_PX),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollArea,
            ChildOf(body),
            DespawnOnExit(page),
        ))
        .id();
    commands.spawn((
        Localized("controls-cancel-hint"),
        muted_text(SMALL_FONT_SIZE_PX),
        Node {
            display: Display::None,
            ..default()
        },
        CancelHint,
        ChildOf(body),
        DespawnOnExit(page),
    ));
    list
}

/// Returns the button around the keycap of `action`, pressed to choose another binding.
/// While it listens it shows its words in place of the keycap.
fn binding_slot(action: ActionName, binding: Binding) -> impl Bundle {
    (
        Node {
            min_width: px(SLOT_MIN_WIDTH_PX),
            height: px(SLOT_HEIGHT_PX),
            padding: UiRect::horizontal(px(SLOT_PADDING_PX)),
            margin: UiRect::left(px(-SLOT_PADDING_PX)),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(SLOT_RADIUS_PX)),
            ..default()
        },
        // a `Button` with `Hovered` and no `ButtonKind`, as the menu's words are: it is
        // pressed and it sounds like any button, and `paint_slots` paints it
        Button,
        Hovered::default(),
        BindingSlot(action),
        BackgroundColor(Color::NONE),
        children![
            (keycap(action, binding_legend(binding)), SlotKeycap),
            (
                Localized("controls-listening"),
                font(SMALL_FONT_SIZE_PX),
                TextColor(INK),
                Node {
                    display: Display::None,
                    ..default()
                },
                SlotHint,
            ),
        ],
    )
}

/// Returns `Display::Flex` for something to show and `Display::None` for something to hide,
/// which also takes it out of the layout.
fn shown(visible: bool) -> Display {
    if visible {
        Display::Flex
    } else {
        Display::None
    }
}

// ---- listening --------------------------------------------------------------

/// What a slot that listens makes of this frame's input.
#[derive(Debug, PartialEq)]
enum Heard {
    Nothing,
    Cancel,
    Binding(Binding),
}

/// Returns what a listening slot does with what was just pressed, if anything was: Esc
/// gives up, anything else is the new binding. A key the OS could not tell apart is not
/// heard at all: it has no name to put on a keycap.
fn heard(pressed: Option<Binding>) -> Heard {
    match pressed {
        None | Some(Binding::Key(KeyCode::Unidentified(_))) => Heard::Nothing,
        Some(Binding::Key(CANCEL_KEY)) => Heard::Cancel,
        Some(binding) => Heard::Binding(binding),
    }
}

// ---- the keycap -------------------------------------------------------------

/// Returns a keycap showing `legend`: a grey skirt, and on it the white face that goes down
/// while the key of `action` is held.
fn keycap(action: ActionName, legend: String) -> impl Bundle {
    (
        Node {
            padding: UiRect::bottom(px(KEYCAP_SKIRT_PX)),
            border_radius: BorderRadius::all(px(KEYCAP_RADIUS_PX)),
            ..default()
        },
        BackgroundColor(KEYCAP_SKIRT),
        BoxShadow::new(
            KEYCAP_SHADOW_COLOR,
            px(0),
            px(KEYCAP_SHADOW_OFFSET_PX),
            px(0),
            px(KEYCAP_SHADOW_BLUR_PX),
        ),
        children![(
            Node {
                min_width: px(KEYCAP_MIN_WIDTH_PX),
                padding: UiRect::axes(px(KEYCAP_PADDING_X_PX), px(KEYCAP_PADDING_Y_PX)),
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(KEYCAP_RADIUS_PX)),
                ..default()
            },
            BackgroundColor(ACCENT),
            keycap_face_transform(false),
            KeycapFace(action),
            children![(
                Text::new(legend),
                font(SMALL_FONT_SIZE_PX),
                TextColor(INK),
                KeycapLegend(action),
            )],
        )],
    )
}

/// Returns where a keycap's face sits: down on its skirt while held, in place otherwise.
fn keycap_face_transform(held: bool) -> UiTransform {
    let down_px = if held { KEYCAP_HELD_PX } else { 0.0 };
    UiTransform {
        translation: Val2::px(0.0, down_px),
        ..UiTransform::IDENTITY
    }
}

/// Returns what a keycap shows for a binding: what is printed on the key, which is never
/// translated, as a language's own name is not.
// ponytail: a key is named by where it is, and its legend is the one a US keyboard prints:
// on an AZERTY keyboard the key shown as W types Z. A held key goes down on the page, so
// the player can find it. The way up is to keep, beside a binding, the `logical_key` the
// page hears when the player presses the key
fn binding_legend(binding: Binding) -> String {
    match binding {
        Binding::Key(key) => key_legend(key),
        Binding::Mouse(button) => format!("Mouse {}", mouse_button_number(button)),
    }
}

/// Returns what is printed on a key, or Bevy's name for one not spelled out here
/// (`Semicolon`, `Minus`).
fn key_legend(key: KeyCode) -> String {
    let named = match key {
        KeyCode::Escape => "Esc",
        KeyCode::Space => "Space",
        KeyCode::Enter => "Enter",
        KeyCode::Tab => "Tab",
        KeyCode::Backspace => "Backspace",
        KeyCode::ShiftLeft => "Shift",
        KeyCode::ShiftRight => "R Shift",
        KeyCode::ControlLeft => "Ctrl",
        KeyCode::ControlRight => "R Ctrl",
        KeyCode::AltLeft => "Alt",
        KeyCode::AltRight => "R Alt",
        KeyCode::ArrowUp => "↑",
        KeyCode::ArrowDown => "↓",
        KeyCode::ArrowLeft => "←",
        KeyCode::ArrowRight => "→",
        _ => "",
    };
    if !named.is_empty() {
        return named.to_string();
    }
    // the rest read as Bevy names them, less the word that says what kind of key it is:
    // `KeyW` is W, `Digit1` is 1, `Numpad0` is Num 0, and `F2` is already F2
    let name = format!("{key:?}");
    if let Some(letter) = name.strip_prefix("Key") {
        letter.to_string()
    } else if let Some(digit) = name.strip_prefix("Digit") {
        digit.to_string()
    } else if let Some(numpad) = name.strip_prefix("Numpad") {
        format!("Num {numpad}")
    } else {
        name
    }
}

/// Returns the number a mouse button goes by: 1 for the left one, as players call them.
fn mouse_button_number(button: MouseButton) -> u16 {
    match button {
        MouseButton::Left => 1,
        MouseButton::Right => 2,
        MouseButton::Middle => 3,
        MouseButton::Back => 4,
        MouseButton::Forward => 5,
        MouseButton::Other(number) => number,
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use bevy::input::keyboard::NativeKeyCode;

    use super::super::spawn_settings_overlay;
    use super::*;
    use crate::input::{Action, register_action};

    // ---- heard--------------------------------------------------------------

    #[test]
    fn a_frame_with_no_press_keeps_the_slot_listening() {
        assert_eq!(heard(None), Heard::Nothing);
    }

    #[test]
    fn esc_cancels_the_listening() {
        assert_eq!(heard(Some(Binding::Key(CANCEL_KEY))), Heard::Cancel);
    }

    #[test]
    fn any_other_key_becomes_the_binding() {
        let up = Binding::Key(KeyCode::ArrowUp);
        assert_eq!(heard(Some(up)), Heard::Binding(up));
    }

    #[test]
    fn a_mouse_button_becomes_the_binding() {
        let click = Binding::Mouse(MouseButton::Left);
        assert_eq!(heard(Some(click)), Heard::Binding(click));
    }

    #[test]
    fn a_key_with_no_name_keeps_the_slot_listening() {
        let nameless = KeyCode::Unidentified(NativeKeyCode::Unidentified);
        assert_eq!(heard(Some(Binding::Key(nameless))), Heard::Nothing);
    }

    // ---- binding_legend -----------------------------------------------------

    #[test]
    fn a_letter_key_reads_as_its_letter() {
        assert_eq!(binding_legend(Binding::Key(KeyCode::KeyW)), "W");
    }

    #[test]
    fn a_digit_key_reads_as_its_digit() {
        assert_eq!(binding_legend(Binding::Key(KeyCode::Digit1)), "1");
    }

    #[test]
    fn a_function_key_reads_as_its_name() {
        assert_eq!(binding_legend(Binding::Key(KeyCode::F2)), "F2");
    }

    #[test]
    fn a_named_key_reads_as_printed() {
        assert_eq!(binding_legend(Binding::Key(KeyCode::Escape)), "Esc");
        assert_eq!(binding_legend(Binding::Key(KeyCode::ArrowUp)), "↑");
    }

    #[test]
    fn a_numpad_key_says_so() {
        assert_eq!(binding_legend(Binding::Key(KeyCode::Numpad0)), "Num 0");
    }

    #[test]
    fn a_mouse_button_reads_as_mouse_and_its_number() {
        assert_eq!(binding_legend(Binding::Mouse(MouseButton::Left)), "Mouse 1");
        assert_eq!(
            binding_legend(Binding::Mouse(MouseButton::Right)),
            "Mouse 2"
        );
    }

    // ---- keycap_face_transform ----------------------------------------------

    /// Returns how far down a keycap's face sits, in pixels.
    fn face_down_px(held: bool) -> f32 {
        match keycap_face_transform(held).translation.y {
            Val::Px(down_px) => down_px,
            other => panic!("the face is not placed in pixels: {other:?}"),
        }
    }

    #[test]
    fn a_held_keycap_sits_lower_than_one_at_rest() {
        assert!(face_down_px(true) > face_down_px(false));
    }

    // ---- the page: a real app, with no window and nothing drawn -------------

    #[test]
    fn the_controls_page_lists_its_action_and_puts_reset_before_close() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<Overlay>();
        app.add_sub_state::<SettingsPage>();
        register_action(
            &mut app,
            Action {
                name: ActionName("jump"),
                default: Binding::Key(KeyCode::Space),
                page: Page::Game,
                label: Some("action-jump"),
            },
        );
        app.add_systems(OnEnter(Overlay::Settings), spawn_settings_overlay);
        app.add_systems(OnEnter(SettingsPage::Controls), spawn_controls_page);
        app.world_mut()
            .resource_mut::<NextState<Overlay>>()
            .set(Overlay::Settings);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<SettingsPage>>()
            .set(SettingsPage::Controls);
        app.update();

        let slots = app
            .world_mut()
            .query::<&BindingSlot>()
            .iter(app.world())
            .count();
        assert_eq!(slots, 1);
        // the page asks for the panel's footer and puts its Reset to Defaults first in it
        let footer = app
            .world_mut()
            .query_filtered::<&Children, With<SettingsFooter>>()
            .single(app.world())
            .unwrap();
        assert_eq!(footer.len(), 2);
        let reset = app
            .world()
            .entity(footer[0])
            .get::<Children>()
            .and_then(|children| children.first())
            .is_some_and(|button| app.world().entity(*button).contains::<ResetControls>());
        assert!(reset);
    }
}
