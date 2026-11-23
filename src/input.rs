use bevy::ecs::system::SystemParam;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::EditableText;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Bindings>();
    app.add_systems(Startup, report_shared_keys);
}

// ==== Actions ================================================================

/// The name of an action: what a feature reads it by, and what the settings file calls it,
/// so it never changes once a game has shipped. A feature keeps its own as a constant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ActionName(pub(crate) &'static str);

/// What an action is bound to: a key, by where it is on the keyboard, or a mouse button. It
/// is what the settings file holds for a binding the player changed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub(crate) enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
}

/// The bindings the settings file holds, by the name of their action: the ones the player
/// changed. Sorted by name, so the file reads the same from one save to the next.
pub(crate) type SavedBindings = BTreeMap<String, Binding>;

/// The page of Settings an action belongs on: the game's own, or the dev tools'.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Page {
    Game,
    #[cfg(feature = "dev")]
    Dev,
}

/// Something the player does, as a feature registers it: its name, the binding it starts
/// on, its page, and the label its page lists it under. An action with no label is not
/// listed and never changes: Pause, whose Esc is also the way out of every panel.
#[derive(Clone, Copy)]
pub(crate) struct Action {
    pub(crate) name: ActionName,
    pub(crate) default: Binding,
    pub(crate) page: Page,
    // a translation key for a game action, plain English words for a dev tool
    pub(crate) label: Option<&'static str>,
}

/// Adds an action to the game's list, bound to its default. Called from the plugin of the
/// feature that reads it, so the list is whole before the settings file is loaded.
pub(crate) fn register_action(app: &mut App, action: Action) {
    app.init_resource::<Bindings>();
    app.world_mut()
        .resource_mut::<Bindings>()
        .actions
        .push(BoundAction {
            action,
            binding: action.default,
        });
}

/// Reads the actions: what a feature asks in place of a key written in the code.
#[derive(SystemParam)]
pub(crate) struct Controls<'w, 's> {
    bindings: Res<'w, Bindings>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    // an `Option`: a world without keyboard focus, as a test's, has no field to type in
    focus: Option<Res<'w, InputFocus>>,
    text_fields: Query<'w, 's, (), With<EditableText>>,
    gamepads: Query<'w, 's, &'static Gamepad>,
}

impl Controls<'_, '_> {
    /// Tells whether what the action is bound to went down this frame. An action nobody
    /// registered was never pressed.
    pub(crate) fn just_pressed(&self, action: ActionName) -> bool {
        let bound = match self.heard(action) {
            Some(Binding::Key(key)) => self.keys.just_pressed(key),
            Some(Binding::Mouse(button)) => self.mouse.just_pressed(button),
            None => return false,
        };
        bound || self.on_any_pad(action, Gamepad::just_pressed)
    }

    /// Tells whether what the action is bound to is held down.
    pub(crate) fn pressed(&self, action: ActionName) -> bool {
        let bound = match self.heard(action) {
            Some(Binding::Key(key)) => self.keys.pressed(key),
            Some(Binding::Mouse(button)) => self.mouse.pressed(button),
            None => return false,
        };
        bound || self.on_any_pad(action, Gamepad::pressed)
    }

    /// Tells whether `test` holds for one of the action's gamepad buttons, on any gamepad.
    fn on_any_pad(
        &self,
        action: ActionName,
        test: impl Fn(&Gamepad, GamepadButton) -> bool,
    ) -> bool {
        self.bindings
            .pads(action)
            .any(|button| self.gamepads.iter().any(|pad| test(pad, button)))
    }

    /// Returns where a stick points, right and up, read by `read` (`Gamepad::left_stick`, what
    /// the menus and a genre's walk go by, or `Gamepad::right_stick`, what a genre looks around
    /// by): the one pushed furthest of all the gamepads', or none pushed.
    pub(crate) fn stick(&self, read: impl Fn(&Gamepad) -> Vec2) -> Vec2 {
        if self.bindings.listening.is_some() {
            return Vec2::ZERO;
        }
        self.gamepads
            .iter()
            .map(read)
            .max_by(|a, b| a.length_squared().total_cmp(&b.length_squared()))
            .unwrap_or(Vec2::ZERO)
    }

    /// Returns what the action is bound to, as `Bindings::heard` hears it, a text field
    /// having the keyboard or not.
    fn heard(&self, action: ActionName) -> Option<Binding> {
        let typing = self
            .focus
            .as_ref()
            .and_then(|focus| focus.get())
            .is_some_and(|focused| self.text_fields.contains(focused));
        self.bindings.heard(action, typing)
    }
}

/// Returns a run condition that holds on the frame the action is pressed, for a system
/// that runs once a press.
pub(crate) fn action_just_pressed(action: ActionName) -> impl FnMut(Controls) -> bool + Clone {
    move |controls: Controls| controls.just_pressed(action)
}

/// Lets a gamepad button do what an action does, beside its key: called by the feature that
/// registered the action, for the few a gamepad needs (pause, jump, the camera). An action may
/// take more than one. Unlike a key, it is not changed on the Controls page.
pub(crate) fn register_pad(app: &mut App, action: ActionName, button: GamepadButton) {
    app.init_resource::<Bindings>();
    app.world_mut()
        .resource_mut::<Bindings>()
        .pads
        .push((action, button));
}

// ==== Resources ==============================================================

/// Every action the game has, in the order the features registered them, each with what it
/// is bound to now.
#[derive(Resource, Default)]
pub(crate) struct Bindings {
    actions: Vec<BoundAction>,
    // what the settings file held for actions this build does not have, those of another
    // branch or of a dev tool in a release build: kept, and written back as it was, since
    // one file serves them all
    unknown: SavedBindings,
    // the action waiting for the player to press what it will be bound to, if any
    listening: Option<ActionName>,
    // the gamepad buttons that do what an action does, each beside its action
    pads: Vec<(ActionName, GamepadButton)>,
}

impl Bindings {
    /// Makes `action` the one waiting for a new binding. Until `stop_listening`, no action
    /// reads as pressed.
    pub(crate) fn listen(&mut self, action: ActionName) {
        self.listening = Some(action);
    }

    /// Returns the action waiting for a new binding, if one is.
    pub(crate) fn listening(&self) -> Option<ActionName> {
        self.listening
    }

    /// Ends the wait for a new binding, whether one was chosen or not.
    pub(crate) fn stop_listening(&mut self) {
        self.listening = None;
    }

    /// Puts every action back on the binding it was registered with. The defaults share no
    /// key, so the result shares none either.
    pub(crate) fn reset(&mut self) {
        for bound in &mut self.actions {
            bound.binding = bound.action.default;
        }
    }

    /// Binds `action` to `binding`, by the swap: whoever held it takes what `action` gave up.
    pub(crate) fn bind(&mut self, action: ActionName, binding: Binding) {
        swap(&mut self.actions, action, binding);
    }

    /// Lays the bindings a settings file held over the defaults, each through the swap, in
    /// the order the actions were registered: so a file edited by hand, or a default that
    /// moved onto a key the player had given another action, still loads with one key an
    /// action.
    pub(crate) fn load(&mut self, mut saved: SavedBindings) {
        let names: Vec<ActionName> = self.actions.iter().map(|bound| bound.action.name).collect();
        for name in names {
            if let Some(binding) = saved.remove(name.0) {
                swap(&mut self.actions, name, binding);
            }
        }
        self.unknown = saved;
    }

    /// Returns what the settings file should hold: the bindings that differ from their
    /// defaults, and the ones it held for actions this build does not have.
    pub(crate) fn saved(&self) -> SavedBindings {
        let mut saved = self.unknown.clone();
        for (name, binding) in changed(&self.actions) {
            saved.insert(name.0.to_string(), binding);
        }
        saved
    }

    /// Returns the actions a page lists, in the order they were registered, each with its
    /// label and what it is bound to now.
    pub(crate) fn listed(
        &self,
        page: Page,
    ) -> impl Iterator<Item = (ActionName, &'static str, Binding)> + '_ {
        self.actions
            .iter()
            .filter(move |bound| bound.action.page == page)
            .filter_map(|bound| {
                bound
                    .action
                    .label
                    .map(|label| (bound.action.name, label, bound.binding))
            })
    }

    /// Returns what the action is bound to, as the game hears it: nothing while a binding is
    /// being chosen, so the key pressed to bind does nothing else, the Esc that cancels
    /// included; and while `typing` in a text field, only the actions that are not listed:
    /// Esc, which types nothing and is the way out of every panel. So a letter typed into a
    /// name is never an action too.
    fn heard(&self, action: ActionName, typing: bool) -> Option<Binding> {
        if self.listening.is_some() {
            return None;
        }
        self.actions
            .iter()
            .find(|bound| bound.action.name == action)
            .filter(|bound| !typing || bound.action.label.is_none())
            .map(|bound| bound.binding)
    }

    /// Returns the gamepad buttons that do what the action does.
    fn pads(&self, action: ActionName) -> impl Iterator<Item = GamepadButton> + '_ {
        self.pads
            .iter()
            .filter(move |(name, _)| *name == action)
            .map(|(_, button)| *button)
    }

    /// Returns what the action is bound to, or `None` for one nobody registered.
    pub(crate) fn binding(&self, action: ActionName) -> Option<Binding> {
        self.actions
            .iter()
            .find(|bound| bound.action.name == action)
            .map(|bound| bound.binding)
    }
}

/// An action and what it is bound to now.
struct BoundAction {
    action: Action,
    binding: Binding,
}

// ==== Systems ================================================================

/// Logs two actions registered on the same key, once, as the game starts. It is a mistake
/// in the code, not something a player did: one press would do two things.
fn report_shared_keys(bindings: Res<Bindings>) {
    let defaults: Vec<Action> = bindings.actions.iter().map(|bound| bound.action).collect();
    if let Some((first, second)) = shared_default(&defaults) {
        error!(
            "the actions `{}` and `{}` are registered on the same key",
            first.0, second.0
        );
    }
}

// ==== Helpers ================================================================

/// Returns the first two actions that share a default binding, or `None` when every
/// action has one of its own.
fn shared_default(actions: &[Action]) -> Option<(ActionName, ActionName)> {
    actions.iter().enumerate().find_map(|(index, action)| {
        actions
            .iter()
            .skip(index + 1)
            .find(|other| other.default == action.default)
            .map(|other| (action.name, other.name))
    })
}

/// Binds `action` to `binding`. Whoever held that binding takes the one `action` gave up,
/// so the two swap and no binding is ever shared or lost. Nothing changes for an action
/// nobody registered, for one that is not listed, or when the binding is held by one that
/// is not listed: those never change.
fn swap(actions: &mut [BoundAction], action: ActionName, binding: Binding) {
    let Some(given_up) = actions
        .iter()
        .find(|bound| bound.action.name == action && bound.action.label.is_some())
        .map(|bound| bound.binding)
    else {
        return;
    };
    let held_for_good = actions
        .iter()
        .any(|bound| bound.binding == binding && bound.action.label.is_none());
    if held_for_good {
        return;
    }
    for bound in actions {
        if bound.action.name == action {
            bound.binding = binding;
        } else if bound.binding == binding {
            bound.binding = given_up;
        }
    }
}

/// Returns the actions bound to something other than their default, each with what it is
/// bound to: what a settings file needs to hold.
fn changed(actions: &[BoundAction]) -> Vec<(ActionName, Binding)> {
    actions
        .iter()
        .filter(|bound| bound.binding != bound.action.default)
        .map(|bound| (bound.action.name, bound.binding))
        .collect()
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    const W: Binding = Binding::Key(KeyCode::KeyW);
    const S: Binding = Binding::Key(KeyCode::KeyS);
    const UP: Binding = Binding::Key(KeyCode::ArrowUp);
    const FORWARD: ActionName = ActionName("forward");
    const BACK: ActionName = ActionName("back");

    const ESC: Binding = Binding::Key(KeyCode::Escape);
    const PAUSE: ActionName = ActionName("pause");

    /// Returns a listed action of the given name on the given key.
    fn action(name: &'static str, key: KeyCode) -> Action {
        Action {
            name: ActionName(name),
            default: Binding::Key(key),
            page: Page::Game,
            label: Some("label"),
        }
    }

    /// Returns the bindings of a game with forward on W, back on S, and pause on Esc, which
    /// is not listed.
    fn bindings_with_pause() -> Bindings {
        let mut bindings = bindings();
        let pause = Action {
            label: None,
            ..action(PAUSE.0, KeyCode::Escape)
        };
        bindings.actions.push(BoundAction {
            action: pause,
            binding: pause.default,
        });
        bindings
    }

    /// Returns the bindings of a game with forward on W and back on S, as it starts.
    fn bindings() -> Bindings {
        let actions = [
            action(FORWARD.0, KeyCode::KeyW),
            action(BACK.0, KeyCode::KeyS),
        ];
        Bindings {
            actions: actions
                .iter()
                .map(|&action| BoundAction {
                    action,
                    binding: action.default,
                })
                .collect(),
            ..default()
        }
    }

    /// Returns what each action is bound to, in the order they were registered.
    fn bound(bindings: &Bindings) -> Vec<Binding> {
        bindings.actions.iter().map(|bound| bound.binding).collect()
    }

    /// Returns a settings file's bindings holding the one given.
    fn saved(name: ActionName, binding: Binding) -> SavedBindings {
        SavedBindings::from([(name.0.to_string(), binding)])
    }

    // ---- Controls -----------------------------------------------------------

    #[test]
    fn no_action_is_heard_while_a_binding_is_chosen() {
        let mut world = World::new();
        world.insert_resource(bindings());
        world.init_resource::<ButtonInput<MouseButton>>();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyW);
        world.insert_resource(keys);
        let forward_is_pressed = |controls: Controls| controls.pressed(FORWARD);

        assert!(world.run_system_once(forward_is_pressed).unwrap());
        world.resource_mut::<Bindings>().listen(BACK);
        assert!(!world.run_system_once(forward_is_pressed).unwrap());
    }

    #[test]
    fn a_gamepad_button_does_what_its_action_does() {
        let mut world = World::new();
        let mut bindings = bindings();
        bindings.pads.push((FORWARD, GamepadButton::DPadUp));
        world.insert_resource(bindings);
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        let mut pad = Gamepad::default();
        pad.digital_mut().press(GamepadButton::DPadUp);
        world.spawn(pad);
        let forward_is_pressed = |controls: Controls| controls.pressed(FORWARD);
        let back_is_pressed = |controls: Controls| controls.pressed(BACK);

        assert!(world.run_system_once(forward_is_pressed).unwrap());
        assert!(!world.run_system_once(back_is_pressed).unwrap());
        world.resource_mut::<Bindings>().listen(BACK);
        assert!(!world.run_system_once(forward_is_pressed).unwrap());
    }

    #[test]
    fn only_esc_is_heard_while_a_text_field_has_the_keyboard() {
        let bindings = bindings_with_pause();
        assert_eq!(bindings.heard(FORWARD, false), Some(W));
        assert_eq!(bindings.heard(FORWARD, true), None);
        assert_eq!(bindings.heard(PAUSE, true), Some(ESC));
    }

    #[test]
    fn no_action_is_heard_while_a_focused_text_field_takes_the_keys() {
        let mut world = World::new();
        world.insert_resource(bindings());
        world.init_resource::<ButtonInput<MouseButton>>();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyW);
        world.insert_resource(keys);
        let field = world.spawn(EditableText::default()).id();
        world.insert_resource(InputFocus::from_entity(field));
        let forward_is_pressed = |controls: Controls| controls.pressed(FORWARD);

        assert!(!world.run_system_once(forward_is_pressed).unwrap());
        world.resource_mut::<InputFocus>().clear();
        assert!(world.run_system_once(forward_is_pressed).unwrap());
    }

    // ---- shared_default -----------------------------------------------------

    #[test]
    fn actions_on_keys_of_their_own_share_none() {
        let actions = [
            action("pause", KeyCode::Escape),
            action("jump", KeyCode::Space),
        ];
        assert_eq!(shared_default(&actions), None);
    }

    #[test]
    fn two_actions_on_one_key_are_found() {
        let actions = [
            action("pause", KeyCode::Escape),
            action("jump", KeyCode::Space),
            action("back", KeyCode::Escape),
        ];
        assert_eq!(
            shared_default(&actions),
            Some((ActionName("pause"), ActionName("back")))
        );
    }

    // ---- swap ---------------------------------------------------------------

    #[test]
    fn binding_a_free_key_changes_one_action() {
        let mut bindings = bindings();
        swap(&mut bindings.actions, FORWARD, UP);
        assert_eq!(bound(&bindings), [UP, S]);
    }

    #[test]
    fn binding_a_taken_key_swaps_the_two_actions() {
        let mut bindings = bindings();
        swap(&mut bindings.actions, FORWARD, S);
        assert_eq!(bound(&bindings), [S, W]);
    }

    #[test]
    fn binding_an_actions_own_key_changes_nothing() {
        let mut bindings = bindings();
        swap(&mut bindings.actions, FORWARD, W);
        assert_eq!(bound(&bindings), [W, S]);
    }

    #[test]
    fn a_key_held_by_an_unlisted_action_is_refused() {
        let mut bindings = bindings_with_pause();
        swap(&mut bindings.actions, FORWARD, ESC);
        assert_eq!(bound(&bindings), [W, S, ESC]);
    }

    #[test]
    fn an_unlisted_action_never_changes() {
        let mut bindings = bindings_with_pause();
        swap(&mut bindings.actions, PAUSE, UP);
        assert_eq!(bound(&bindings), [W, S, ESC]);
    }

    #[test]
    fn a_saved_binding_for_an_unlisted_action_is_ignored() {
        let mut bindings = bindings_with_pause();
        bindings.load(saved(PAUSE, UP));
        assert_eq!(bound(&bindings), [W, S, ESC]);
    }

    // ---- Bindings::listed ---------------------------------------------------

    #[test]
    fn a_page_lists_its_labelled_actions_in_order() {
        let bindings = bindings_with_pause();
        let listed: Vec<ActionName> = bindings
            .listed(Page::Game)
            .map(|(name, _, _)| name)
            .collect();
        assert_eq!(listed, [FORWARD, BACK]);
    }

    // ---- Bindings::reset ----------------------------------------------------

    #[test]
    fn resetting_puts_every_action_back_on_its_default() {
        let mut bindings = bindings();
        bindings.bind(FORWARD, S);
        bindings.reset();
        assert_eq!(bound(&bindings), [W, S]);
        assert_eq!(changed(&bindings.actions), []);
    }

    // ---- changed ------------------------------------------------------------

    #[test]
    fn default_bindings_have_nothing_to_save() {
        assert_eq!(changed(&bindings().actions), []);
    }

    #[test]
    fn only_changed_bindings_are_saved() {
        let mut bindings = bindings();
        swap(&mut bindings.actions, FORWARD, UP);
        assert_eq!(changed(&bindings.actions), [(FORWARD, UP)]);
    }

    // ---- Bindings::load and Bindings::saved ---------------------------------

    #[test]
    fn a_saved_binding_wins_over_its_default() {
        let mut bindings = bindings();
        bindings.load(saved(FORWARD, UP));
        assert_eq!(bound(&bindings), [UP, S]);
    }

    #[test]
    fn two_saved_bindings_on_one_key_load_as_one_key_each() {
        let mut bindings = bindings();
        let mut both_on_up = saved(FORWARD, UP);
        both_on_up.extend(saved(BACK, UP));
        bindings.load(both_on_up);
        let loaded = bound(&bindings);
        assert_ne!(loaded.first(), loaded.last());
    }

    #[test]
    fn a_binding_for_an_unknown_action_is_kept() {
        let mut bindings = bindings();
        let of_another_build = saved(ActionName("fly"), UP);
        bindings.load(of_another_build.clone());
        assert_eq!(bound(&bindings), [W, S]);
        assert_eq!(bindings.saved(), of_another_build);
    }

    #[test]
    fn what_is_saved_loads_back_the_same() {
        let mut first_run = bindings();
        first_run.load(saved(FORWARD, S));

        let mut second_run = bindings();
        second_run.load(first_run.saved());
        assert_eq!(bound(&second_run), bound(&first_run));
    }
}
