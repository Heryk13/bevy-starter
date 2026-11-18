# Key bindings: a Controls page that changes the keys

Status: **built** — written, approved and built on 2026-11-18, in the seven steps of
`docs/roadmap/key-bindings-tasks.md`: steps 1 to 5 on `main`, step 6 on `3d/player`. Gamepads
and moving through the menus with the keyboard are left for later (see "Out of scope").
Date: 2026-11-18
Bevy: 0.20.0

Row 1 of `docs/roadmap/roadmap.md`. The roadmap's "Key bindings, as agreed so far (row 1)" holds
what was agreed before this design; it builds on that. The page as proposed is in
`docs/roadmap/ui/controls-screens.html` (open it in a browser: click a key and press another).

## Goal

Today a key is a `KeyCode` written where it is read: `KeyCode::Escape` in `overlays.rs`,
`KeyCode::KeyW` in `player.rs`, `KeyCode::F3` in `main.rs`. A player cannot change one, and
every row of the roadmap after this one would add more of the same.

This design gives each of those keys a name, an *action*, and puts the keys behind the names.
A feature says what it does ("move forward") and what it is bound to by default; it reads the
action, never the key. Settings gains a Controls page that lists the actions and lets the
player bind each to another key or mouse button, and the bindings are remembered.

Done looks like this: `just run` on `3d/player`. Esc, Settings, Controls: four rows, Move
Forward beside a W keycap. Click the keycap, press the up arrow: the keycap says ↑. Close,
resume: the up arrow walks and W does nothing. Quit and start again: the up arrow still
walks.

## Decided

*(Claude's call)* marks what Claude proposed in the mockup and the user accepted with it on
2026-11-18 ("I agree with almost all of it"); the rest was agreed in the roadmap or said by
the user. Three calls were made after that and are marked as such.

| Question | Decision |
| --- | --- |
| Gamepads | **Not in this spec**, by the user on 2026-11-18: left for the future. Keyboard and mouse only. The roadmap had this row bind both |
| Where the actions come from | A list each feature adds its own to from its plugin, the way each floating layer adds itself to `GlassLayers`. `main` adds Pause; a branch adds what it has (walking) and touches no shared code to do it |
| What an action is bound to | One key or mouse button, always: every action is registered with a default, and none is ever left without. Not several *(Claude's call)* |
| Looking | The mouse looks and is not an action: there is nothing about it to rebind *(Claude's call)* |
| Esc | Fixed: it pauses, it closes every panel, and it cancels the page's own listening, so it must never be lost *(Claude's call)*. **It has no row on the page**, by the user on 2026-11-18: what cannot be changed need not be shown. Pause is still an action, one the page does not list |
| The FPS overlay's key | **F2**, where it is F3 today, by the user on 2026-11-18 (the roadmap's row 2 had already planned the move: F3 is photo mode's). It is the action a player can change on `main`, in a dev build, on the Dev page. The user turned down a fullscreen key on F11, which Claude had proposed to give `main` a game action to list |
| A page with nothing on it | Is not offered: on `main` no game action can change, so Settings has no Controls page there, and in a release build, with General alone, no row of pages either. The first branch to register an action with a label brings the page *(Claude's call, made after the mockup: it follows from Esc having no row)* |
| A key another action has | The two actions swap. Nothing is asked, and no action loses its key *(Claude's call)* |
| The pages | Settings gets a row of pages under its title, the segmented control the language uses: General, Controls, Dev. One level, not a Controls page with tabs in it *(Claude's call)* |
| The Dev page | The dev tools' keys, behind `#[cfg(feature = "dev")]`, in English, as the roadmap agreed |
| Reset | A Reset to Defaults button on each of the two pages, asking first with the dialog the game has. It puts every action back, on both pages *(Claude's call, changed after the mockup: resetting one page alone could leave two actions on one key, when a swap had crossed the pages)* |
| Where it is saved | In `settings.ron`, as the roadmap agreed. Only the bindings the player changed, so a default changed in a later version still reaches them. A binding for an action this build does not have is kept as it was *(Claude's call; the second half made after the mockup: every branch, and the dev and the release build, share the one file)* |
| A key's legend | What is printed on a key (`W`, `Esc`, `Shift`) is never translated, as a language's own name is not. A mouse button reads `Mouse 1`, `Mouse 2`, a legend too *(Claude's call)* |
| A held key | A keycap on the page goes down while its key is held *(Claude's call, made after the mockup: see Structure, "A held key")* |
| No crate | The roadmap's decision: Bevy reads keyboards and mice itself |
| Where it is built | `main`, tried on `3d/player`, each step with `just tools/sync main`; the last step but one is `3d/player`'s own |

## Approaches

Three ways to put keys behind names were weighed.

- **A list the features fill** (chosen). An action is a name; its binding is data in a
  resource; reading one is asking that resource. A branch registers actions and changes
  nothing shared, which is the rule of the branch tree. It is the least code.
- **A crate** (`leafwing-input-manager`, `bevy_enhanced_input`). They do more: chords,
  several bindings an action, contexts, gamepads. But the template would wait on one more
  crate at every Bevy release, the roadmap already said no, and their actions are an enum,
  which is the next approach's problem.
- **One `Action` enum in `main`** that each branch adds variants to. The simplest to read
  (`Action::Jump`), but every branch would edit the same shared file, against "a branch only
  adds", and two sibling branches adding a variant conflict on every merge down.

## Must not

- **Lose the way out.** No binding, swap or reset may leave the player unable to pause or to
  close a panel. Esc is fixed for that reason.
- **Two actions on one key**, or an action left with none: not after a swap, a reset, a
  settings file edited by hand, or a default changed in a later version.
- **Anything reacting to the input being bound.** The key pressed to bind is not also a key
  pressed: it does not pause, close the panel or toggle a tool, in that frame or the next.
  This holds for the Esc that cancels, and for a mouse click, which must reach nothing under
  the pointer.
- **A settings file that stops loading**, or one that loses bindings. A `settings.ron` from
  before this spec loads with the defaults; one naming an action this build does not have
  (another branch's, or a dev tool's in a release build) loads, and keeps that binding.
- **The Dev page in a release build**, or any trace of its actions.
- **A branch editing shared code to add an action.**
- **Closing the door on gamepads.** Nothing here may have to be undone when they come: a
  second input beside the key is a field added to a binding and a column added to the page.

## Out of scope

- **Gamepads**: binding a button or a stick, and playing with one. Put off by the user.
- **Moving through the menus with the keyboard** (focus, directional navigation,
  `bevy_input_focus`). The roadmap left it as "this row's or a row of its own": it is its
  own. It touches every screen and every button, where this design touches one page.
- **Mouse sensitivity and inverted look.**
- **Two bindings for one action**, **an action with no key**, **chords** (Shift + X) and
  **the mouse wheel**.
- **Rebinding Esc.**
- **Legends that follow the keyboard layout** (see Structure, key legends).
- **The keys that skip the splash and the intro.** "Any of these skips" is not an action a
  player rebinds; `SKIP_KEYS` stays as it is.

## Facts this rests on (read in the 0.20.0 source and in this repository)

- **Where keys are read today.** `main`: `apply_esc_action` in `src/overlays.rs`
  (`input_just_pressed(KeyCode::Escape)`), and in dev builds `toggle_fps_overlay` in
  `src/main.rs` (`FPS_TOGGLE_KEY`, F3). `3d/player` adds `walk_player` in
  `src/screens/gameplay/player.rs` (W, S, A, D, each read with `pressed`) and `look_around`
  in `player/first_person.rs` (`AccumulatedMouseMotion`). `2d/base` adds none. The splash
  and the intro read `SKIP_KEYS`.
- **Bevy's input.** `ButtonInput<KeyCode>` and `ButtonInput<MouseButton>` are resources with
  `pressed`, `just_pressed` and `get_just_pressed()`.
- **A key press as an event.** `KeyboardInput` is a message with `key_code` (where the key
  is), `logical_key` (what it types on this layout) and `state`.
- **A click reaches buttons by itself.** `bevy_ui_widgets`' `Button` takes `Pressed` on a
  pointer press and fires `Activate` on the click that follows. It does not go through
  `ButtonInput<MouseButton>`, so silencing the actions does not silence a click.
- **Saving a key.** `KeyCode` and `MouseButton` derive `Serialize` and `Deserialize` only
  under the `serialize` feature of `bevy_input`, which no branch turns on today
  (`cargo tree -e features -i bevy_input`). It is a feature of a crate already in the tree,
  not a new crate. `Cargo.toml` already holds one line of that kind: `icu_provider`, "not
  used directly", there for its `logging` feature. The `bevy = ..` line itself differs on
  every branch, so a change to it conflicts on every merge down.
- **Settings.** `SettingsFile` in `src/overlays/settings.rs` is read by `load_settings` at
  `Startup` and written by `save_settings` on `OnExit(Overlay::Settings)`. New fields take
  `#[serde(default)]`, as `language` and `reduce_transparency` did. The file is 647 lines.
  `setting_row(key, control)` lays a label in a column of its own beside its control. One
  `settings.ron` serves every branch and both builds: `game_data_dir()` names its folder
  after the package.
- **Pages already have a shape.** The language's segmented control is a `RadioGroup` of
  `RadioButton`s painted by `update_language_segments`. `Pause` is a `SubStates` of
  `Screen::Gameplay`: a state that exists only while its parent state holds.
- **Settings is never open over a running game.** It opens from the menu or from the pause
  menu, so no gameplay system runs while the page is up.
- **What `just check` refuses.** Code nobody calls on `main`, even when a branch below calls
  it, a field nobody reads and a variant nobody builds, in the dev build as in the release
  one; and `#[allow]`, so it cannot be waved through. Whatever lands on `main` must be used
  there, in the commit that brings it.
- **Keycaps** were designed with the Look (roadmap, "The look": a white face on a grey skirt,
  two nodes) and left out of it for having no screen to stand on. This page is that screen.

## Structure

### Actions and bindings (`src/input.rs`, new)

A shared file at the top of `src/`, named after its domain, as `ui.rs` and `save.rs` are.

- **An action** is registered once, from its feature's plugin: a name that never changes
  (`"move_forward"`, what the settings file calls it), its default key, its page (game or
  dev) and, when the player may change it, its label (a locale key, or English words for a
  dev tool). An action with no label, like Pause, is read like any other and never changes:
  no page shows it, the swap never takes its key, and a saved binding for it is ignored. It
  is kept an action all the same, so that listening silences it with the rest; and being a
  game action on `main`, it is what keeps the game's page from being code nobody reaches
  there. A feature keeps the name as a constant and reads the action by it. Two
  actions registered with the same default key is a mistake in the code, logged with
  `error!` as the game starts.
- **A binding** is a key or a mouse button.
- **Reading.** One system parameter answers two questions about an action: is it held, and
  was it just pressed. And a run condition for systems that run on a press, replacing
  `input_just_pressed(KeyCode::..)`.
- **Listening silences it.** While the page waits for a key, every action reads as not
  pressed, and the page stops listening only after everything that reads an action in that
  frame has run: actions are read in `Update` or before it, and the page takes the key in
  `PostUpdate`. So the Esc that cancels never closes the panel, and a key just bound to a
  tool does not also run it.
- **One way to change a binding**: the swap, a pure function (the bindings, an action and a
  new key in; the bindings out). Whoever held the key takes the one this action gave up, so
  keys stay one to an action by construction; a key held by an action that is not listed is
  refused, and the bindings come out as they went in. The page uses it, and so does loading.
- **The other pure parts**, each tested: the bindings that differ from the defaults, and a
  key's legend.

### A held key

A keycap on the page is drawn pressed while its key is held down. It tells the player the
binding works before they close the panel, and it tells them which physical key a legend
names, which matters on a keyboard that is not a US one (see key legends). It is also what
reads "is it held" on `main`: there nothing is held to play, so without it that question
would be code only the branches call, which `just check` refuses, and each branch would have
to add it to the shared file itself.

### Key legends

`KeyCode` names a key by where it is, which is right for binding (W, A, S, D keep their
shape on any keyboard) and wrong for reading: on an AZERTY keyboard the key at `KeyW` types
Z. The legend shown is the one a US keyboard prints, and a held key lights its keycap, so
the player can find it. That ceiling gets a `ponytail:` comment naming the way up: the page
hears `logical_key` when the player presses a key, and could keep that text beside the
binding.

### Saving (`src/overlays/settings.rs`, `Cargo.toml`)

`SettingsFile` gains the changed bindings, by action name, with `#[serde(default)]`. Loading
starts from the defaults and applies each saved binding through the swap, in the order the
actions were registered, so a file edited by hand, or a default that moved onto a key the
player had given another action, still loads with one key an action. An action the file
does not name keeps its default. A name this build does not have is set aside and written
back untouched: opening Settings on `main` must not erase what the player bound on
`3d/player`, nor a release build the Dev page's keys. Saving writes what differs from the
defaults, and those, when Settings closes, with the rest of the file.

`Cargo.toml` gains a line of its own for `bevy_input` with its `serialize` feature, at
Bevy's version, with a comment saying it is not used directly, as `icu_provider` has. A line
of its own is the same on every branch, so it merges down with no conflict, where Bevy's own
`serialize` would go on the `bevy = ..` line, which is different on each.

### The pages (`src/overlays/settings.rs`)

A sub-state of `Overlay::Settings` names the page shown: General, Controls and, in dev
builds, Dev. It starts at General each time Settings opens. The card keeps its title, the
row of pages and Close; each page spawns its own content on entering its state and loses it
on leaving, the rule every state in the project follows. General is today's four rows, moved
as they are.

A page of controls is offered only when it has an action to list, and the row of pages only
when there is more than General to choose from. So on `main` a release build shows Settings
as it is today, a dev build shows General and Dev, and `3d/player` shows all three.

### The Controls and Dev pages (`src/overlays/settings/controls.rs`, new)

`settings.rs` is past the size where the project splits a file, and the pages of controls
read on their own, so they are a part of it: `settings.rs` next to `settings/`.

- **The list.** A scrolling column, as the save lists have, of one `setting_row` per action
  listed on that page, in the order they were registered: the label in its column, then the
  key's slot. The same grid as the General page.
- **A slot** is a button holding a keycap.
- **Listening.** Activating a slot makes it the one listening: it turns white and says
  "Press a key…", and a line under the list says Esc cancels. The next key or mouse button
  becomes the binding; Esc cancels. Since a mouse click is a valid binding, clicking
  elsewhere while a slot listens binds that click: Esc is the way out. That click must not
  also press what is under it (Close, a page, another slot): while a slot listens, something
  stands between the pointer and the card, such as a node over the whole window that takes
  the press and is gone when listening ends.
- **The swap.** If another action, on either page, had that key, it takes the one this
  action gave up.
- **Reset to Defaults** asks with `dialog()`, its Yes of `ButtonKind::Danger`, and puts every
  listed action back to its default, those of the other page included.
- **The keycap**: a skirt node holding a face node holding the legend in `INK`, by the
  roadmap's measures; held, the face sits down on its skirt. It lives in `controls.rs`: this
  page is its only user, and it moves to `ui.rs` when a third feature shows one, as
  `CONTRIBUTING.md` asks.

### The actions this spec registers

| Action | Where | Key | Listed on |
| --- | --- | --- | --- |
| Pause | `overlays.rs`, on `main` | Esc | nowhere: it does not change |
| FPS Overlay (dev) | `main.rs`, on `main` | F2, where it was F3 | Dev |
| Move Forward, Back, Left, Right | `player.rs`, on `3d/player` | W, S, A, D | Controls |

`apply_esc_action` and `toggle_fps_overlay` run on their action's press. `walk_player` asks
whether each of its four actions is held. `look_around` and `first_person.rs` do not change.

### Text

Every label and word on the game's page is a key in the three `main.ftl`: the two page
names, the listening line, the cancel hint, Reset to Defaults and its question, and one key
per listed action. `3d/player` adds its actions' keys to the same files. The Dev page's words are
English in the code.

### Files

| File | What changes |
| --- | --- |
| `src/input.rs` (new) | actions, bindings, reading them, the pure parts and their tests |
| `src/main.rs` | the input plugin; the FPS overlay as an action |
| `src/overlays.rs` | Pause as an action |
| `src/overlays/settings.rs` | the pages and their sub-state; the bindings in the file |
| `src/overlays/settings/controls.rs` (new) | the two pages of controls and the keycap |
| `Cargo.toml` | `bevy_input` named for its `serialize` feature |
| `assets/locales/*/main.ftl` | the page's words; on `3d/player`, its actions |
| `src/screens/gameplay/player.rs` (`3d/player`) | walking through actions |

## Tests

Pure functions, each with its tests beside it:

- The swap: binding a free key changes one action; binding one another action has swaps the
  two; binding an action's own key changes nothing; a key held by an action that is not
  listed is refused.
- Changed bindings: none when everything is default; exactly the ones that differ.
- Loading: a saved binding wins; a missing action keeps its default; an unknown name is kept
  for the next save; two saved bindings on one key load as one key an action.
- A key's legend: a letter, a digit, a function key, a named key, a mouse button.
- The settings file: one from before this spec loads with no bindings; one with a binding
  reads it back; one naming an unknown action loads.
- What a pressed key does to a listening slot: Esc cancels, any other key or mouse button
  binds.
- The translation test already fails while one `main.ftl` lacks a key another has.

What no test sees, and `just run` must: the page and the keycaps over the sky, the list
scrolling, a keycap going down under a held key, a click that binds and presses nothing.

## Implementation order

One commit per step. Steps 1 to 5 on `main`, each tried on `3d/player` and then
`just tools/sync main "<message>"`; step 6 on `3d/player`. Each step updates the docs it
touches (`README.md`, `docs/flow.md`, `docs/template.md`).

Each piece lands in the step that first uses it, since `main` takes nothing unused: an
action is registered with a name and a key in step 1, and gains its page and its label in
step 3. That is why `3d/player`'s own step comes after them: its four
registrations are written once, in their final shape.

1. `feat: read pause and the fps overlay as actions` — `src/input.rs`: registering, the
   press and its run condition, the defaults only. `overlays.rs` and `main.rs` read their
   actions, and the FPS overlay moves from F3 to F2. Nothing else looks or behaves
   differently.
2. `feat: remember changed bindings in the settings file` — the `bevy_input` line, the
   field, the swap, loading through it, saving what differs and what this build does not
   know. Tried by editing `settings.ron` by hand: the FPS overlay moved to another key.
3. `feat: list the actions on the settings pages` — the sub-state and the row of pages;
   General as it is; `settings/controls.rs`: the Controls page and, in dev builds, the Dev
   page, each a list of keycaps, which go down under a held key, and each offered only when
   it has an action to list. Nothing can be clicked yet. On `main` it is seen in a dev
   build, on the Dev page.
4. `feat: rebind an action from its page` — the slots as buttons, listening, the cancel
   hint, the click that reaches nothing. Tried on the Dev page, the one with a key that can
   change on `main`.
5. `feat: reset the controls to their defaults` — the button and its dialog.
6. (`3d/player`) `feat: walk through actions` — the four actions, `walk_player`, their keys
   in the three `main.ftl`. The first place two actions can swap in the game itself: until
   here the swap is held by its tests.
7. `docs: mark the key bindings spec built` — this spec's status and "What changed while
   building it", the roadmap's row 1, `README.md`.

## What changed while building it

Built in the seven steps of `docs/roadmap/key-bindings-tasks.md`, written by Claude in one
sitting on 2026-11-18 at the user's request, each step checked with `just check` as it was
finished. The task numbers below are that file's.

- **A slot paints itself, at once** (decided while building 4.1). Task 4.1 gave a slot the
  white fill of a `Choice` button, which `paint_buttons` eases; but a slot has three faces
  (at rest, under the pointer, listening) and shows words in place of its keycap, so
  `paint_slots` in `controls.rs` paints it, as `paint_menu_words` paints the menu, and with
  no ease.
- **What listens is a field of `Bindings`, not a resource of its own** (decided while
  building 4.2). `Controls` already asks `Bindings` what an action is bound to, so asking it
  whether one is being chosen needs no second resource in every reader.
- **The press that starts the listening is never taken as the answer** (found while building
  4.4). `take_pressed_binding` waits a frame after a slot starts to listen: a tap
  that presses and lets go within one frame would otherwise bind Mouse 1 on the spot.
- **Reset to Defaults is put in the card's footer by its page** (decided while building 5.1).
  The footer belongs to the panel and holds Close; a page of controls inserts its button
  before Close, with a margin that takes the room between the two, and it goes when the page
  does. `SettingsFooter` marks the row.
- **The language's control and the row of pages share `segment` and `set_segment`** (decided
  while building 3.1), where the language had `language_segment` and its own painting.
- **Two tests open Settings in a real app** (added in 3.1). A page is spawned by another
  state's `OnEnter` than the panel's and asks for the panel's body, an order no pure function
  can hold: `settings_opens_on_the_general_page_inside_the_panel` does.
- **"Press a key..." is translated on the Dev page too** (decided while building 4.1). The
  slot is one widget for both pages, and its words are the game's: only a dev tool's own
  label is English.
- **Right-hand modifier keys say so** (decided while building 3.5): `R Shift`, `R Ctrl`,
  `R Alt`. A keyboard prints the same word on both, but they are two keys to bind.
- **No slot keeps listening after the panel closes** (added in 4.4). Nothing can close the
  panel while a slot listens, since Esc is silenced and the shield takes every click; an
  `OnExit` system stops the listening all the same, so no action could ever stay deaf.
- **A key the OS cannot name is not heard** (found in the review after step 7). A slot keeps
  listening through a `KeyCode::Unidentified`, which has nothing to print on a keycap. The
  same review added a test that opens the Controls page in a real app, and one that no
  action is heard while a slot listens.
- **Two commits, not seven** (decided at the sync). The seven steps were written in one
  sitting and run in the game together, so they landed as one commit on `main`, steps 1 to 5
  and 7, and one on `3d/player`, step 6.
