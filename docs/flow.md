# Screens, overlays and the flow between them

Where the player can be, what moves them on, and where to put the next screen.
For setup and commands see the [README](../README.md), for the code conventions
[CONTRIBUTING.md](../CONTRIBUTING.md).

Three state enums answer three different questions. `Screen` in `src/screens.rs` says *where the
player is*; `Overlay` in `src/overlays.rs` says *what is drawn on top*; `Pause` in
`src/screens/gameplay/pause.rs` says *whether the game is frozen*. They are separate because an
overlay must not tear down the screen underneath it, and a pause menu must still be there when an
overlay opened over it closes.

## The screen flow

```
  Splash --> Menu --> Loading --> Intro --> Gameplay
              ^        |  ^         |         ^  |
              |        |  |         +- skip ->+  |
              |        |  |                   |  |
              |        +--|-- a loaded save --+  |
              |           |                      |
              |           +----- Load Game ------+  (from the pause menu)
              |                                  |
              +<------- Quit to Menu ------------+  (from the pause menu)
              |
              +--> Quit --> AppExit::Success
```

- New Game, Continue and Load Game all leave the menu for `Loading`.
- `Screen::Splash` is the `#[default]`, so the game starts there.
- The plugins are registered in `screens.rs` in the same order the player meets them:
  `splash`, `menu`, `loading`, `intro`, `gameplay`.
- A skip in `Splash` is not an edge on this diagram: it ends the current logo, and the sequence
  leaves for the menu on its own once the last one is done.
- `Loading` has two ways out: a new game goes on to `Intro`, a loaded save straight to
  `Gameplay`. Whoever sends the player there says which, through `StartGame` (see
  [Starting a game](#starting-a-game)).
- Quit to Menu, Load Game out of `Gameplay` and the skip out of `Intro` are the other edges that
  are not the plain next step. There is no way back into `Splash` or `Intro`.

## The overlay flow

```
  Overlay::None  (the #[default] — nothing on top)
        |
        +--[Settings button]--> Overlay::Settings --+
        |                                           +--> Close button / Esc --> None
        +--[Credits button]---> Overlay::Credits ---+
        |                                           |
        +--[Save button]------> Overlay::SaveGame --+   (pause menu only; a save closes it too)
        |                                           |
        +--[Load Game button]-> Overlay::LoadGame --+   (menu and pause menu; a load closes it too)
        |                                           |
        +--[Find Server]------> Overlay::FindServer +   (menu only; a Join closes it too)
```

The overlay is drawn over whatever screen is below, and that screen keeps running: entering
an overlay never changes `Screen`, so no `DespawnOnExit(Screen::X)` fires and nothing is lost.
That is why the menu music keeps playing while Settings is open.

Esc, in these flows, is also Start or B on a gamepad: both are the pause action's buttons
(`overlays.rs`). A gamepad moves through the buttons of the topmost layer only, so a dialog or
an overlay keeps its focus over what is under it (`ui/navigation.rs`).

**Multiplayer** (`src/multiplayer.rs` and its folder; every line outside them that names it ends
with `// multiplayer`) keeps where this game stands on the network in `Network`: `Offline`,
`Hosting`, `Joining`, `Joined`.

```
  Offline --Open to LAN (pause menu)--> Hosting, back in the game, "open to LAN"
  Hosting --Close LAN (pause menu) / leaving gameplay--> Offline
  Offline --Join (Find Server)--> Joining --connected--> Joined --> a new game, into gameplay
                                     +--not in time / full--> Offline, "could not join"
  Joined --host gone, or leaving gameplay--> Offline, the menu, "the host ended the game"
```

The host runs a lightyear server (netcode over UDP, `GAME_PORT`) inside its own game and
broadcasts on `DISCOVERY_PORT` every second; Find Server listens and lists each game heard, and
takes an address typed; a joined game's pause menu has no Open to LAN. Each game sends its
`NetworkedBody`'s pose (the genre puts that on its player); the host relays every pose to
everyone, and each game shows the others as
`RemotePlayer`s (`multiplayer/remote.rs`), which a player branch draws in its
`player/others.rs`, a joiner starting on a ring around the spawn point.
A shared game never freezes: entering `Pause::On` or `Pause::Photo` pauses no time while hosting
or joined (`is_shared_game`), nor stills the view the glass blurs, and slow motion stays off. The host's game is the world, with an
id (`GameData::world_id`); a joiner gets it and its props (`WorldState`) as it connects, keeps
only its own character, per world, in `characters.ron` (`multiplayer/world.rs`), and has no Save,
Load Game nor autosave. Every prop edit goes through the host (`MapEdit` asked, `MapChange` made,
`multiplayer/props.rs`), a prop held is locked for the others, the host's physics moves every
physics prop for all, and in dev builds the props' files go through the host both ways
(`multiplayer/files.rs`).

## The pause flow

```
  Gameplay (Pause::Off, the #[default])
        |
       Esc
        v
  Pause::On --[Resume] / Esc -------------> Pause::Off
        |
        +--[Save]------------> Overlay::SaveGame --Close / Esc / a save--> back to Pause::On
        +--[Load Game]-------> Overlay::LoadGame --Close / Esc--> back to Pause::On
        |                                       \--a save------> Screen::Loading
        +--[Settings]--------> Overlay::Settings --Close / Esc--> back to Pause::On
        +--[Quit to Menu]----> (dialog, if progress is unsaved) --Yes--> Screen::Menu
        +--[Quit to Desktop]-> (dialog, if progress is unsaved) --Yes--> AppExit::Success

  Pause::Off --F3, in dev builds--> Pause::Photo --F3 / Esc--> Pause::Off
```

**Photo mode** (`src/dev_tools/photo_mode.rs`, dev builds only) is the third value of `Pause`,
`Photo`, behind `#[cfg(feature = "dev")]` as the variant nothing in a release build builds. It
freezes the game as the pause menu does, without the menu: `pause.rs` runs the same
`pause_time` and `freeze_background` on `OnEnter(Pause::Photo)` and their pairs on `OnExit`.
F3 enters it only from `Pause::Off` (the pure `photo_key_target`), and F3 or Esc leaves it.
Entering hides every root marked `DevUi` (every dev window, by `spawn_dev_window`, and the F1
hint), and `fps.rs` turns the FPS off, keeping whether it showed; leaving brings both back.
The other dev keys do nothing meanwhile (`.run_if(not(in_state(Pause::Photo)))`). Photo mode
shows its own windows, marked `PhotoModeUi`: a bar of its keys at the bottom, built from the
hints the tools add with `register_photo_hint` (Take Photo on `main`, the free camera's on
`3d/player`), Exit always last. E
(`photo_mode/photo.rs`) hides those for one frame, takes a `Screenshot` of the window in that
frame, and when it arrives shows them again, flashes the window's edges, plays a click, and
writes the PNG on another thread into `gallery/` in the data folder, named by its time in UTC
(`2026-11-18_14-32-05.png`, `_2` for a second photo in the same second). A "Photo saved"
notice comes once the file is written. On `3d/player` the cursor stays held, and the view
leaves the player as a free camera (`dev_tools/free_camera.rs`), coming back where it was as
photo mode ends.

**The file explorer** (`src/dev_tools/file_explorer.rs` and its folder, dev builds only) is no
state either: Numpad 0 opens it over any screen, the running game included, which goes on
under it, and Numpad 0, its X or Esc slide it away. It is a glass drawer along the bottom
(`drawer.rs`), on the dev tools' layer and letting the pointer through around it, so the game
above stays sharp; its grip resizes it, and the height is kept while the game runs. It shows one
of four places, a resource, `Explorer`, saying which, with the folder and the entry selected:
Recents (every photo of the gallery, newest first), Gallery (`gallery/` in the data folder),
Assets (the folder Bevy loads from, the repository's `assets/` under `just run`) and Trash
(`trash/` in the data folder). Every part redraws from `Explorer` when what it shows changes:
the title bar's places and path (`title_bar.rs`), the tree (`tree.rs`), the list (`list.rs`)
and the preview (`preview.rs`). A double click enters a folder or opens a file in the OS's app;
a right click opens a menu (`menu.rs`); a name is renamed in place (`rename.rs`); an entry
dragged onto a folder, a place or a part of the path moves there, or is copied into the other
place; a file dropped from the OS is copied into the folder shown (`actions.rs`). Delete moves
an entry into Trash, which `trash.rs` lists with where each came from and can put back. Before
Assets has something renamed, moved or deleted, `usage.rs` reads `src/` and the text files of
`assets/` for its path, and when the code names it, asks first, listing each line. Its
questions and its menu are a layer of their own over it, 310 (`dialog.rs`).

**Screen recording** (`src/dev_tools/recording.rs` and its folder, dev builds only) is a
state of its own, `RecordingState` (`Off`, `Bar`, `Countdown`, `On`), beside `Pause` and not
inside it, since the pause menu may open while recording and is recorded as the game's own UI.

```
  Off --F9, in Pause::Off--> Bar --E / Record--> Countdown --zero--> On --E / F9--> Off
                              |  (None timer) ------------------->  ^
                              +--F9 / Esc--> Off      Countdown --F9--> Off
```

The bar (`bar.rs`) sits at the bottom, letting the pointer through around it, with
`ClosesOnEsc`, so `3d/player` frees the cursor under it; its choices (`RecordingChoices`) last
until the game closes. The countdown and the REC pill count `Time<Real>`. E is the `capture`
action, shared with photo mode, which reads it only in `Pause::Photo`; F3 does nothing outside
`RecordingState::Off`, and leaving gameplay goes back to `Off`, writing the file. While
recording, the dev tools' layer's camera copies the frame under it, before it draws the dev
UI, into an image read back at each frame of the video (`capture.rs`), so the video holds the
game alone; for as long as it lasts, the FPS and the F1 hint, drawn by the screen camera
otherwise, are lent to that layer (`lend_dev_layer`). `encoder.rs` writes H.264 and the sound's
Opus into `gallery/<time>.mp4.part` on a thread of its own and renames it once whole, on stop
and on `AppExit` alike; `sound.rs` hears the game's own process through Windows' process
loopback.

**The prop editor** (`src/dev_tools/prop_editor.rs` and its folder, dev builds only) is a
state of its own, `PropEditor` (`Off`, `On`), a `SubStates` of `Pause::Off`: the game keeps
running under it, and the pause menu, photo mode or leaving gameplay each close it, since its
state stops existing with `Pause::Off`. F10 opens and closes it; Esc puts a prop held back where
it was, and with none held closes it. F3 does nothing while it is open, as both move the camera.

```
  Off --F10, in Pause::Off--> On --F10 / Esc with nothing held--> Off
                               +--Esc / pause / photo mode / leaving gameplay--> gone
```

Its window is a drawer along the bottom, `ClosesOnEsc`, letting the pointer through around it,
so `3d/player` frees the cursor while it is open and holds it again only while the right button
turns the camera. The props are the files in `assets/props/` the genre takes (`PropKind`), read
again every second while the window is open. Every change goes through one `MapEdit` (`Place`,
`Update`, `Delete`), which `apply_map_edit` applies to the game in progress, `Session::game.map`,
and to the world; the next save keeps it, an autosave included. Save as Default Map writes that
map, each prop where it was placed, to `assets/map.ron`. `screens/gameplay/map.rs`, in every
build, spawns the game's props on entering gameplay, a game with no map yet (a new one, or one
saved before props were) getting a copy of `assets/map.ron` first; each is a `Prop`, a
`PropFile`, a `PropMode` and a `Transform`, and a genre observes `PropFile` to put the model or
the image on it, and on `3d/player` the body of its mode. There a physics prop's resting place
(`resting`) goes into the session in `Pause::Off`, as the player's state does, so a save keeps a
pushed box where it stopped.

A dialog is not a state: it is an entity drawn over whatever asked the question, the pause menu
or the save list, and No or Esc despawns it and leaves what is underneath as it was.

`Pause` is a `SubStates` of `Screen::Gameplay`: it only exists while gameplay does. Leaving
gameplay removes it, so the next game starts unpaused without any code resetting it, and its
`OnExit` still runs, so a game quit while paused never leaves the clock frozen.

## Starting a game

New Game, Continue and every row of the Load Game list start a game, from two screens that leave
differently: the menu fades out behind its veil, gameplay cuts. So whoever starts a game triggers
one event, `StartGame` in `src/screens.rs`, carrying the `Session` to play and the
`LoadingDestination`, and the screen that is up decides how to leave:

| Observer | Acts in | What it does |
| --- | --- | --- |
| `start_game_from_menu` (`menu.rs`) | `Screen::Menu` | inserts both resources, spawns the closing veil |
| `start_game_from_gameplay` (`gameplay.rs`) | `Screen::Gameplay` | inserts both resources, sets `Screen::Loading` |

Each carries `.run_if(in_state(...))` for its screen, so only the one whose screen is up runs. `overlays/saves.rs` triggers the event and never learns what a veil is.

## Screen by screen

| Screen | Spawned by | Ends when | Ends by |
| --- | --- | --- | --- |
| `Splash` | `spawn_splash_screen` | the last logo is done, or skipped | `advance_splash_logos` |
| `Menu` | `spawn_menu_screen`, `play_menu_music` | the closing veil finishes | `animate_veils` |
| `Loading` | `spawn_loading_screen`, `load_intro_assets` (new games only) | the intro's assets are in, or at once for a loaded save | `leave_loading_screen` |
| `Intro` | `spawn_intro_screen`, `play_intro_music` | the last frame is done, or skipped | `leave_intro_screen`, `skip_intro_screen` |
| `Gameplay` | `spawn_gameplay_screen` | Quit to Menu, or a save picked in Load Game | `activate_pause_button` and `confirm_quit` (`pause.rs`), `start_game_from_gameplay` |

### Splash

Shows the Bevy logo first, then every image in `assets/splash/logos/` in file name order. One
`SplashSequence` component holds the folder handle until it has been read, the list, the index and
the timer, all on the logo entity.

Each logo runs the same four-phase `Fade`: wait, fade in, hold, fade out. The four systems are
`.chain()`ed because each one feeds the next: `skip_splash_logo` finishes the current timer,
`fill_splash_logos` appends the folder's images once it has loaded, `advance_splash_logos` ticks
the timer and swaps the image, and `fade_splash_logo` reads the timer the tick just advanced.

A skip ends the *current* logo only, never the whole sequence, so `advance_splash_logos` stays
the one place that decides between "next logo" and "leave for the menu". The skip set is
`SKIP_KEYS` (Escape, Space, Enter, E, F) and `SKIP_MOUSE_BUTTONS` (left, right), both
`pub(super)` so `intro.rs` skips the same way.

A `ponytail:` comment marks the known ceiling: a folder still loading when the Bevy logo ends is
skipped rather than waited on.

### Menu

Five layers under one root, spawned back to front: the background image, a gradient scrim that is
dark on the left and clear on the right, the column (`GAME_TITLE` from `main.rs` over the
menu's words), the version in the bottom-right corner from `CARGO_PKG_VERSION`, and a black veil.
`assets/menu/music.ogg` loops beside it with `DespawnOnExit(Screen::Menu)`, so the track's
lifetime is the screen's.

The choices are words alone, with no card and no capsule: `menu_word(key)`, the menu's own
widget, since nothing else looks like it. Each is a `Button` with `Hovered` and a `MenuWord`,
and no `ButtonKind`: it is pressed and it sounds like any button, while `paint_buttons` leaves
it alone. `paint_menu_words` eases `MenuWord::hover` as `paint_buttons` eases a button, over the
same `BUTTON_EASE_SECS`, and paints from the pure `menu_word_paint(hover)`: a white bar behind
the word, solid for `BAR_SOLID_PERCENT` of its width and fading out to the right
(`bar_gradient`), the word from greyish to `INK`, and its shadow fading out as it darkens. The
words touch, so a pointer going down the column is always over one and the bar never blinks,
and they line up under the title: the column starts `WORD_PADDING_PX` to its left
(`COLUMN_LEFT_PX`), where the bars begin.

The words carry a `MenuAction` component, and the `activate_menu_word` observer reads it off
`activate.entity`:

| Word | What it does |
| --- | --- |
| New Game | triggers `StartGame` with a fresh `Session`, headed for the intro |
| Continue | triggers `StartGame` with the newest save `newest_ready` finds, headed for gameplay |
| Load Game | sets `Overlay::LoadGame` |
| Settings | sets `Overlay::Settings` |
| Credits | sets `Overlay::Credits` |
| Quit | writes `AppExit::Success` |

Continue is only there when a save can be loaded: `update_continue_word`, chained after
`spawn_menu_screen`, sets its `Node::display` to `Display::None` otherwise, which takes it out of
the column so the words below close the gap. Its click lists the saves again rather than trust
what the menu saw when it opened.

`StartGame` reaches `start_game_from_menu`, which keeps the session and the destination as
resources and spawns a second veil, this one fading *in* to black. Neither sets the next screen
itself. `animate_veils` does, once the closing veil's timer
finishes — that indirection is what makes the fade visible before the screen changes. Which screen
comes next is `screen_after_veil`'s answer, a pure function with its own tests. The entrance
veil is despawned when it finishes instead of being left at alpha 0, because a transparent
full-window node would still swallow every click.

`update_menu_music_volume` runs `.after(animate_veils)` so the sound never lags the picture by a
frame. It writes the volume every frame on purpose: `GlobalVolume` is only read when a sink starts,
so the settings slider would otherwise do nothing to a track that is already playing.

`update_frozen_background` also runs after `animate_veils`, and tells the glass the menu is still
while no veil is left: an overlay opened under the opening veil would otherwise keep, for as
long as it stays open, a blur of the menu half covered (see "The cameras and the blur").

### Loading

A slideshow over black with a progress bar along the bottom. Where it goes next is the
`LoadingDestination` resource the caller left, `Intro` for a new game and `Gameplay` for a loaded
save, and `screen_after_loading` turns it into a screen, a pure function with a test per
destination. A loaded save has nothing to load yet, so `load_intro_assets` and
`update_loading_bar` are skipped and `leave_loading_screen` leaves on the next frame; this is
where the gameplay's own assets will be loaded later.

For a new game, `load_intro_assets` starts loading
`assets/intro/frames/` and `assets/intro/music.ogg` into the `IntroAssets` resource — a resource
and not an entity, because an entity with `DespawnOnExit(Screen::Loading)` would drop the handles
exactly when the intro needs them.

`fill_slideshow` takes every image in `assets/loading/`, shuffles it and shows the first;
`advance_slideshow` wraps around at the end, so the screen can run as long as the load does. A
`ponytail:` comment notes that the shuffle happens once, so a second lap repeats the same order.
`update_loading_bar` counts the frame handles that are loaded with their dependencies, plus the
music as one more item.

`leave_loading_screen` counts a failed asset as done. Bevy already logs the failure, and a missing
frame is cosmetic where a screen that never ends is not.

### Intro

Bevy plays no video, so the intro is the frames a video would hold, swapped on a timer with the
soundtrack alongside. `spawn_intro_screen` takes the handles from `IntroAssets`, sorts them with
`images_in`, and sizes one `Timer` to `frame_count / INTRO_FPS`.

`INTRO_FPS` is set by the soundtrack, not by the frames: the constant carries a comment saying to
recalibrate it whenever the intro is re-exported, so picture and sound keep ending together.

Unlike the splash, a skip here leaves the screen outright — `skip_intro_screen` sets
`Screen::Gameplay` on the same keys and buttons. `unload_intro_assets` runs on
`OnExit(Screen::Intro)` and removes the resource, dropping the last handles so Bevy can give the
frames' memory back.

### Gameplay

A full-window background colour and nothing else yet. The `Session` arrives with `StartGame`,
fresh or loaded, and `count_play_time` adds each frame's virtual time to it, so the pause menu
never counts as play. The resource is also created at startup: a default `Session` is a valid
new game, so gameplay can be entered without the menu while building the game. Esc lives in `src/overlays.rs` with the rest of the
Esc rule.

**Where the game's own state goes.** `Session::game` is a `GameData`, the struct in `save.rs` that
holds what the game keeps in a save: the world, the player, their progress. It is empty so far,
and already wired end to end: New Game starts it from `default()`, every save writes it, and
Continue and Load Game hand it back inside the `Session`. So a gameplay feature that must survive
a save adds a field to `GameData`, builds its entities from `Session::game` on
`OnEnter(Screen::Gameplay)`, and keeps the field current while playing, so whatever saves next
writes it. `GameData` is `#[serde(default)]`, so a save written before a field existed still
loads, with that field at its default.

`start_game_from_gameplay` answers `StartGame` while gameplay is up, which is a save picked in the
pause menu's Load Game list: it keeps the session and the destination as resources and sets
`Screen::Loading` on a cut, since gameplay has no veil.

`autosave_game` runs `.after(count_play_time)` and only in `Pause::Off`. Once
`autosave_is_due`, after `AUTOSAVE_INTERVAL_SECS` of unsaved play, it writes a new
`autosave_<millis>.ron` with an empty name, which the save lists show as Autosave from the
file name's prefix (`is_autosave`); every autosave is kept. It first sets
`autosave_tried_secs`, and the interval counts from that or the last save, whichever is later,
so a failed autosave is logged once and tried again a full interval later instead of every
frame. An autosave counts as a save, so quitting right after one asks nothing. Once the file is
written, `take_save_picture` asks for a picture of the window, which is written beside the
autosave when it arrives, off the main thread.

**The pause menu** (`src/screens/gameplay/pause.rs`, registered by `gameplay.rs`'s plugin).
`OnEnter(Pause::On)` spawns the panel with `DespawnOnExit(Pause::On)`: a `floating_root` holding a
`card`, so the frozen game shows blurred and dimmed behind it (see "The cameras and the blur").
It also calls `Time<Virtual>::pause()`, and `OnExit` calls `unpause()`. Any gameplay written later
that reads `Res<Time>` stops while paused for free, with no `run_if` of its own. The same two
schedules run `freeze_background` and `unfreeze_background`, which tell the glass that the game
behind it stands still, so it is blurred once instead of every frame.

The buttons carry a `PauseAction` component, read by the `activate_pause_button` observer, the
same shape as the menu's:

| Button | What it does |
| --- | --- |
| Resume | sets `Pause::Off` |
| Save | sets `Overlay::SaveGame`, drawn above the pause menu; how the save went shows under the buttons |
| Load Game | sets `Overlay::LoadGame`, drawn above the pause menu; loading asks nothing, even with unsaved play |
| Settings | sets `Overlay::Settings`, drawn above the pause menu |
| Quit to Menu | sets `Screen::Menu`, or asks first when `has_unsaved_progress`; `Pause` goes with the screen |
| Quit to Desktop | writes `AppExit::Success`, or asks first when `has_unsaved_progress` |

"Quit without saving?" is `dialog()` from `ui.rs`, spawned with `DespawnOnExit(Pause::On)` so it
cannot outlive the pause menu. Both quit buttons are one action, `PauseAction::Quit(QuitTo)`, and
the dialog's Yes carries the same `QuitTo` in `ConfirmQuit`. Whether the quit asked first or not,
the leaving itself is `quit_to`, called by `activate_pause_button` and `confirm_quit` alike. Play time within `UNSAVED_GRACE_SECS` of the last save does
not count, so quitting right after starting never asks.

## Overlays

Every panel follows the same lines: spawn on `OnEnter(Overlay::X)`, build an `overlay_root()` holding
one `overlay_card()`, and carry `DespawnOnExit(Overlay::X)`. The card starts with `heading(key)`
and ends with `close_footer()`, Close on the right. `overlay_root()` in `src/overlays.rs` is
`floating_root(OVERLAY_Z_INDEX)` from `ui.rs`: the whole window, `Floating`, and glass that comes
in with the card. The glass's dimming tint matters when an overlay opens over the pause menu: the
pause card darkens under it instead of competing with it.

- **Credits** (`src/overlays/credits.rs`): a translated line, the names as plain text, and a Close
  button.
- **Settings** (`src/overlays/settings.rs`) has pages. `SettingsPage` is a sub-state of
  `Overlay::Settings`, as `Pause` is of `Screen::Gameplay`: General, Controls and, in dev builds,
  Dev, starting at General each time the panel opens. `spawn_settings_overlay` spawns the card
  with its title, the row of pages, an empty `SettingsPageBody` and Close; each page fills the
  body on entering its state and loses its rows on leaving it (`DespawnOnExit`). The row of
  pages is a segmented control, a `RadioGroup` of `segment`s carrying a `PageOption`:
  `choose_page` sets the next page and `update_page_segments` fills the one shown, through the
  `set_segment` the language's control shares. `offered_pages` leaves out a page of controls
  that has no action to list, and with General alone there is no row of pages: on `main` a
  release build shows Settings as it was, a dev build General and Dev.

  The Controls and Dev pages are `settings/controls.rs`. Each is a scrolling column
  (`spawn_action_list`) of one `setting_row` per action its page lists (`Bindings::listed`): the label,
  a translation key for a game action and English words for a dev tool, then a `keycap`, a grey
  skirt holding a white face holding the key's legend. `binding_legend` gives the legend, what
  is printed on the key, never translated (`W`, `Esc`, `Mouse 1`). `press_held_keycaps` sets a
  `KeycapFace` down on its skirt while its action's key is held.

  A keycap sits in a `binding_slot`, a `Button` carrying `BindingSlot(action)`. Pressing it
  (`listen_for_binding`) makes its action the one `Bindings` is listening for: the slot turns
  white and shows "Press a key…" in place of its keycap, a line under the list says Esc
  cancels (`paint_slots`), and a `ListeningShield` is laid over the whole window. While
  `Bindings` listens, `Controls` reports no action as pressed. `take_pressed_binding` runs in
  `PostUpdate`, after every reader of the frame: the next key or mouse button becomes the
  binding through `Bindings::bind`, the swap, and Esc gives up (the pure `heard`). So the key
  being bound does nothing else, the Esc that cancels does not close the panel, and a click
  that becomes a binding lands on the shield, not on the button under the pointer. Esc itself
  can never be bound: it cancels, and Pause, which holds it, is not listed.

  Each page of controls puts a Reset to Defaults in the card's `SettingsFooter`, before Close.
  `ask_to_reset_controls` asks with `dialog()`, and on Yes `reset_controls` calls
  `Bindings::reset`, which puts every action back, those of the other page included: a swap
  can cross the two pages, so one page reset alone could leave two actions on one key.

  The General page (`spawn_general_page`) is a grid of `setting_row`s, each a label in a column of
  its own beside its control: a fullscreen switch, a Reduce Transparency switch and a 0–100
  volume slider, all from `bevy_ui_widgets`, and the language segments. The two switches are
  the same `switch()`, a `Checkbox` drawn as a capsule track with a `SwitchKnob`, turned on
  and painted by `set_switch`: the accent with a dark knob at the right when on. The slider is
  as tall as its thumb, with a thin track through it and a `VolumeFill` up to the thumb, in
  the accent. Its `update_fullscreen_switch`, `update_reduce_transparency_switch`,
  `update_volume_slider` and `update_language_segments` are gated with
  `.run_if(in_state(SettingsPage::General))`, since their controls exist only on that page;
  the widgets are headless, so those systems are what
  actually draw the state, and a control already showing its state is left alone.
  `toggle_reduce_transparency` flips `ReduceTransparency`, a resource of `ui/blur.rs`, and the
  glass follows at once, the Settings panel's own included (see "The cameras and the blur").
  All four choices are remembered in `settings.ron` in the
  game's data folder: `load_settings` applies it at `Startup`, and `save_settings` writes it on
  `OnExit(Overlay::Settings)`, once per visit rather than on every slider move. The `Window`,
  `GlobalVolume`, `Language` and `ReduceTransparency` stay the only copy of the state; the file
  is read from them and written back to them. A missing, unreadable or other-version file keeps
  the defaults silently, since `parse_settings` returns `None` for all of them; a failed write
  is logged with `error!`. A file from before Reduce Transparency existed loads with it off.
  The file also holds the key bindings the player changed, by action name (`bindings`, empty
  in an older file): `load_settings` hands them to `Bindings::load` and `save_settings` writes
  `Bindings::saved()` (see `src/input.rs` below).
  The last row picks the language with a segmented control: a `RadioGroup` drawn as one
  capsule track (`segment_track`), holding a `segment` (a `RadioButton`) per `Language::ALL`, each named
  in its own language. `choose_language` observes the group's `ValueChange<Entity>` and sets
  the `Language` resource; `update_language_segments` checks the one that matches it and fills
  it with the accent. Side by side, the segments fit three or four names: a game with more
  languages needs a list there. `OVERLAY_WIDTH_PX` is what the label column and the three
  names need together.
  The language goes into `settings.ron` with the rest, as `Some(language)`; a file without one,
  or no file, follows the OS through `sys_locale` and `language_from_locale`.
- **Save Game** (`src/overlays/saves.rs`): a name field, a New Save button, then one row per file
  in the saves folder, then Close. The field is an `EditableText` marked `SaveName`, capped at
  `SAVE_NAME_MAX_CHARS`; the widget is headless, so the field's own fill, border and a
  `TextCursorStyle` are what make it visible, and without the latter no cursor is drawn.
  `EditableText` only holds the text: `TextInput` is what types into it, and `TabIndex` is what
  lets a click give it the keyboard, since Bevy hands focus on a click only to an entity with
  one. `paint_save_name_field` outlines it in the accent while `InputFocus` is on it. Each save is a
  `row_button` holding `save_row`: the save's picture (`save_picture`), then `save_name` over
  `save_details` in small muted text. The picture is read from disk as the list spawns
  (`read_save_picture`) and added to `Assets<Image>`; the `ImageNode` holding its handle is what
  keeps it alive, so it goes with the overlay. A save that has no picture, one from before
  pictures existed or one whose file cannot be read, shows a grey frame holding
  `Icon::ImageOff` in its place, marked `OwnColor` so it stays light on the grey when the row
  turns white, and loads all the same. The frame is `SAVE_PICTURE_WIDTH_PX`
  wide and as tall as the file's own shape makes it (`PICTURE_ASPECT_RATIO`).

  The folder is read once, on `OnEnter(Overlay::SaveGame)`. The rows are spawned in a `for`
  with `ChildOf(list)`, because `children![]` only holds a list known when the code is
  written; Close is spawned after them for the same reason, so it ends up last. `list` is a
  column capped at `SAVE_LIST_MAX_HEIGHT_PX`, the height of `SAVE_ROWS_SHOWN` rows and the gaps
  between them, with `Overflow::scroll_y()` and `ScrollArea`, so
  the mouse wheel scrolls the rows while the title, the field, New Save and Close stay put; it
  reuses `OVERLAY_GAP_PX`, since the panel's own gap only spaces its direct children. A save
  the game can read is a row button showing `save_name` over `save_details`, both pure functions:
  `Castle` over `1h 01m played · 3d ago`. An incompatible or unreadable file is muted text, so nothing
  that looks clickable does nothing. Autosaves are left out (`is_autosave` on each entry's
  `path()`): overwriting one would keep its name and turn a player's save into an "Autosave".

  | Press | Observer | What it does |
  | --- | --- | --- |
  | New Save | `write_new_save` | writes the session to a new `save_<millis>.ron`, named by the field, trimmed |
  | a save's row | `ask_to_overwrite_save` | spawns "Overwrite this save?", its Yes carrying `ConfirmOverwrite(path, name)` |
  | that Yes | `overwrite_save` | writes the session over that file, keeping the save's own name |

  Both writes go through `try_write_save`, which calls `write_save` and returns whether it
  worked, logging the reason with `error!` on a failure; no data folder at all counts as a
  failure too. A save that was written then gets its picture, `save_pause_picture`: the one
  taken as this pause was asked for (see `src/save.rs` below). Either way the overlay triggers `SaveFinished { saved }` (in `save.rs`) and
  closes, and never touches the pause menu's widgets: `show_save_status` in `pause.rs` answers
  the event, writing `SAVED_TEXT` in the accent or `SAVE_FAILED_TEXT` in red on the status line,
  a `Text` marked `SaveStatus` that `pause.rs` spawns empty. It also sets the line's
  `shown_secs` to zero, and `fade_save_status` shows it at once, holds it and fades it out by
  `SAVE_STATUS_FADE`, counting `Time<Real>` because virtual time is frozen while paused. The dialog
  carries `DespawnOnExit(Overlay::SaveGame)`, so it cannot outlive the list.
- **Load Game** (`src/overlays/saves.rs`, beside Save Game): the same scrolling `save_list()`
  with the same labels, autosaves included, and "No saves yet" when the folder has none. It opens
  from the menu and from the pause menu. A readable save's row carries `LoadSave(path)`, and the
  `load_save` observer reads that file again with `read_save`, since the list may be stale, then
  triggers `StartGame` with `loaded_session` headed for gameplay and sets `Overlay::None`, so the
  panel does not outlive the screen it was opened over. A file that no longer reads is logged
  with `error!` and the player stays on the list.

  Every row also has a Delete carrying `DeleteSave(path)`, a round `icon_button` holding
  `Icon::Trash2` in `ButtonKind::Danger` with an `AccessibleLabel`, the files the game cannot load
  included, since this is the only way they leave the list. `ask_to_delete_save` spawns "Delete
  this save?" with `DespawnOnExit(Overlay::LoadGame)`, its Yes carrying `ConfirmDelete(path)`, and
  `delete_save` removes the save and its picture (`delete_save_files`) and sets
  `Overlay::LoadGame` again. `set` re-enters a state it is
  already in, so `OnExit` despawns the panel and `OnEnter` reads the folder anew: the list loses
  the row with no code to rebuild it. A failed delete is logged with `error!` and the row stays.
  That same `OnExit` runs the menu's `update_continue_word` again, so deleting the last save hides
  Continue without leaving the menu.

**Closing.** No panel owns its Close button's behaviour. Each ends its card with
`close_footer()`, which holds `(button("overlay-close", ButtonKind::Quiet), CloseOverlay)`, and
the single `close_overlay` observer in `src/overlays.rs` sets `Overlay::None` when the activated
entity carries the marker. `apply_esc_action` is the second way out, run only on the frame Esc is
pressed (`action_just_pressed(PAUSE)`, the Pause action of `src/input.rs`, which is always on
Esc); with nothing open during running gameplay, the same press pauses
instead, and calls `take_pause_picture` on that frame, the last one that shows the game with no
menu over it. The whole Esc rule lives in one pure function beside it,
`esc_action`, which returns an `EscAction` and has a test per line, first match wins:

1. a modal is open, a dialog or a dev tool's drawer (`ClosesOnEsc`) → send the topmost a
   `Dismiss`, which each answers its own way: a dialog despawns, the file explorer slides away.
   Over the explorer the layers decide the order: its question or menu (310), then its rename
   field (301), which gives the rename up, then the explorer (300)
2. an overlay is open → close it (back to the pause menu, if paused)
3. gameplay is paused → resume
4. gameplay is running → pause
5. gameplay is in photo mode, in dev builds → back to the running game

It reads `Option<Res<State<Pause>>>`, because `Pause` does not exist outside gameplay. Esc does
nothing else in `Menu` or `Loading`, and in `Splash` and `Intro` it is a skip key, which moves
*forward*.

**Why a state and not a screen.** Two reasons, both structural:

1. Leaving `Screen::Gameplay` fires `DespawnOnExit(Screen::Gameplay)` and destroys the running
   game. Anything drawn over a live game, like Settings over the pause menu, cannot be a screen.
2. A screen would need a back destination. Settings opened from the menu must return to the menu,
   and from the pause menu to the pause menu. `CloseOverlay` has nothing to compute, remember or
   get wrong, because what is underneath never went away. That is also why the pause menu is a
   state of its own and not an `Overlay` variant: closing an overlay always goes to
   `Overlay::None`, and the pause menu is simply still there.

**Why `GlobalZIndex`.** A screen and an overlay are two separate UI roots, so nothing in the node
tree decides which draws first. Plain `ZIndex` only orders siblings inside one tree, so it cannot
do this. The layers, bottom to top:

| Layer | `GlobalZIndex` | Constant |
| --- | --- | --- |
| screens | none | |
| menu veil | 1 | `VEIL_Z_INDEX` |
| pause menu | 50 | `PAUSE_Z_INDEX` |
| overlays | 100 | `OVERLAY_Z_INDEX` |
| dialog | 200 | `DIALOG_Z_INDEX`, in `ui/panel.rs` |
| dev tools' windows, in dev builds | 300 | `DEV_TOOLS_Z_INDEX`, in `dev_tools.rs` |
| the file explorer's rename field, in dev builds | 301 | `RENAME_FIELD_Z_INDEX`, in `dev_tools/file_explorer/rename.rs`, 1 above the dev tools: a modal only for Esc, inside the explorer |
| the file explorer's questions and right-click menu, in dev builds | 310 | `EXPLORER_DIALOG_Z_INDEX`, in `dev_tools/file_explorer/dialog.rs`, 10 above the dev tools |
| the prop editor's mode and depth menus, on `3d/player` and `2d/player`, in dev builds | 310 | `MENU_Z_INDEX`, in `dev_tools/prop_editor_3d/mode_menu.rs` and `dev_tools/prop_editor_2d.rs`, 10 above the dev tools |

Those numbers order roots *within a camera*. Every floating layer (pause, overlays, dialogs) also
has a camera of its own, ordered by the same number and drawn after the screen camera, so each
layer is above every screen whatever its `GlobalZIndex`, and above the layers under it.

## The cameras and the blur

`main.rs` spawns the **screen camera**: `Camera2d` and `IsDefaultUiCamera`. Every UI root
without a `UiTargetCamera` is drawn by it: the splash, the menu, loading, the intro, gameplay.

Each floating layer gets a **layer camera**. `pause.rs`, `overlays.rs` and `ui/panel.rs` push
their `*_Z_INDEX` onto the `GlassLayers` resource from their plugins, and `spawn_layer_cameras`
(`src/ui/blur.rs`) spawns one camera per entry at `Startup`: a `LayerCamera` holding that
`GlobalZIndex`, which is also its `order`, so the cameras draw in the order the layers stack;
`ClearColorConfig::None`, so each draws over what the cameras below it drew instead of wiping
it; and `RenderLayers::none()`, so none draws a sprite or tilemap of a 2D game a second time,
sharp, over the blurred one; UI ignores render layers, so the panels still reach them. The
observer `target_layer_camera` gives each new `Floating` root a `UiTargetCamera` pointing at the
camera of its `GlobalZIndex`; a root on a layer nobody pushed logs a warning and stays on the
screen camera. A new layer is three things: its constant, the push from its plugin, and its row
in the table above. Clicks need nothing: Bevy's UI picking tests every camera and puts the
later one's hits first. `activate_layer_cameras` turns a layer camera on only while a `Floating`
root of its layer is open, in `PostUpdate` so a root opened or closed during `Update` already
counts: an inactive camera draws nothing, so a closed layer costs nothing.

The blur is a **dual Kawase**, run by each layer camera on what the cameras below it drew. At
`Startup`, and again on every `WindowResized`, `generate_level_images` gives each layer camera a
`GlassCopies`: the descent, `Rgba16Float` images each half the one before, as many halvings as
`levels_for_height` takes to bring the screen to `GLASS_TARGET_HEIGHT_PX` or below, plus one level
more for the cards; and two ascents, the backdrop's and the cards', each back up to half the
screen. `init_kawase_pipelines` queues
the two Kawase pipelines at `RenderStartup`, so they compile while the game loads, not on the
first pause; the glass material's own pipeline is still compiled by Bevy on its first use.
`blur_layer_backdrops` runs in `Core2d`, once per active layer camera, between
`Core2dSystems::PostProcess` and `ui_pass`, the bounds the UI pass itself has: the shared image
then holds the layers below but not this layer's own UI, and nothing before it (the MSAA
writeback of a camera that does not clear swaps the image) can swap it mid-read. It halves the image
down the descent with `assets/shaders/kawase_down.wesl`, every pass reading the one before, then
doubles it back up twice with `kawase_up.wesl`: the backdrop's ascent from the level above the
deepest, and the cards' from the deepest, so a card reads blurrier than the backdrop around it.
Each pass is timed with a `pass_span`, so `RenderDiagnosticsPlugin` can log it as
`kawase_down_pass` or `kawase_up_pass`.

`target_layer_camera` also gives each new `Floating` root a `MaterialNode` of its own
`GlassMaterial`: the top of its layer's backdrop ascent (or the first level, when the window is so
small that the ascent is empty), the `SCREEN_DIM` tint, and an alpha that `animate_entrances`
brings from 0 to 1 over `ENTRANCE`. `give_card_glass`, an observer on `Add<Card>`, gives each new
card one too, reading the top of the cards' ascent and fading in with its root's. A material
replaces what is under it, the root's dim included, so the card's tint is `SURFACE` laid over
`SCREEN_DIM`, made one colour by the pure `lay_over`. `assets/shaders/glass.wesl` reads the copy
at the fragment's screen position, lays the tint over it, fades by that alpha and cuts the node's
rounded corners, which Bevy leaves to the material, so the glass comes in with its panel. A
card's tint darkens toward its bottom by `CARD_SHADE`, the material's `shade`, which is 0 for a
backdrop; and the card carries a rim, a `BorderGradient` bright at its top left and dark at its
bottom right (`CARD_RIM_*` in `panel.rs`). Bevy draws a material over its node's border, so
`GlassMaterial` overrides `stack_z_offset()` to sit where a fill does, under the rim. Which
image each reads is `GlassCopies::backdrop_copy` and `GlassCopies::card_copy`. A resize gives the
cameras new copies, and the observer `retarget_open_glass`, on `Insert<GlassCopies>`, points
every open root of that layer and its cards at them, so a panel left open through a resize reads
the new blur instead of the one from before it.

A layer over a still background is blurred once. `blur.rs` holds `FrozenBackground`, off unless
a feature turns it on, and hands out `freeze_background` and `unfreeze_background` for that:
`pause.rs` runs them on entering and leaving `Pause::On`, and `menu.rs` sets it every frame from
whether a veil is fading over the menu (`update_frozen_background`), and off on leaving the
screen. So `blur.rs` names no screen, as with `GlassLayers`. In `PostUpdate`,
`freeze_settled_layers` marks each layer camera's `FrozenLayer`: the background is still, and
every `Floating` root at or below the layer has finished its `Entrance` (the pure `is_settled`)
and blocks the pointer: under one that lets it through, something may still be hovered.
The layer's own root counts too: its entrance covers what is still settling under it, like the
button that opened it easing out of its hover. `blur_layer_backdrops` reads the mark and keeps,
for each layer camera, the copy it last finished while frozen; a frozen layer whose copy is
still that one runs no pass. A resize hands out new copies, so it is blurred again without a
signal of its own, and a frame that could not blur (a pipeline still compiling) is not counted.
A background nobody called still is blurred every frame, which is always right, only slower:
that is a panel over live gameplay, such as a dev tool. What changes under a frozen layer is
not seen until the layer closes: a language picked in Settings rewrites the menu behind it,
and the blur keeps the old words.

A floating root that lets the pointer through (`Pickable` that does not block, the pure
`lets_pointer_through`) floats over a game that goes on: `target_layer_camera` sends it to its
layer's camera but gives it no backdrop, so what is under it stays sharp and takes clicks, and
only its cards are glass; and its layer is never frozen. The dev tools' windows are such roots
(`spawn_dev_window` in `dev_tools.rs`, which gives every node inside `Pickable::IGNORE` too,
since picking stops at a node without one), on a layer of their own above the dialog: sharp
over the pause menu, with whatever is open under them blurred in their glass. Their windows
are not `card()`s but carry the same `card_glass()`, so they are glass the same way.

Reduce Transparency draws the glass flat. `ReduceTransparency` is a resource of `blur.rs` that
the Settings overlay writes and saves, so `blur.rs` does not name the overlay. While it is on,
`blur_layer_backdrops` runs no pass, and every `GlassMaterial` carries `flat`: the shader then
draws the tint alone, as opaque as the tint's own alpha, so a backdrop is `SCREEN_DIM` over the
sharp screen and a card is `SURFACE` made opaque (`card_tint`, a pure function). A panel opened
under the setting is built flat (`GlassMaterial::backdrop` and `GlassMaterial::card`), and
`apply_reduce_transparency` redraws the ones already open when it changes. Turning it on forgets the
copy a frozen layer kept, so turning it off blurs that layer again instead of showing a copy
from before.

A layer that shows a blur it already made hides everything under it, so whatever is costly to
draw there may stop. Only the passes know that a copy was really made, so they are the ones to
say it: `FrozenLayer` carries `copy_kept`, one flag shared by the camera's component and its
extracted copy, which `blur_layer_backdrops` writes and the main world reads a frame later.
From it `update_screens_covered` sets `ScreensCovered`: some open layer is frozen and shows a
copy it kept, and the glass is not flat, since a flat backdrop is a dim the screens show
through (the pure `are_screens_covered`). Whoever stops drawing runs in `PostUpdate`, after
`ScreensCoveredSystems` and before `CameraUpdateSystems`: Bevy works out what a camera sees,
and builds its shadows, only for a camera that is on, so one turned back on later in the frame
draws an empty frame, and the glass would copy that. Two more things keep the kept copy whole.
A resize clears `copy_kept` at once, in `generate_level_images`, so what had stopped draws
again before the new copy is made. And the frame the screens are uncovered, no layer keeps the
copy it makes: the one kept is the next frame's, drawn whole. It all counts on every
`Floating` root covering the window, as `floating_root` makes them.

Three settings must stay as they are: `IsDefaultUiCamera` on the screen camera (or every root
moves to the top layer's camera), `ClearColorConfig::None` on the layer cameras (or the screens
vanish), and the same HDR and MSAA on all of them (or they stop sharing the image).

## Shared code

Seven files at the top of `src/`, each named after its domain, each used by more than one feature.

| File | What it holds | Why it is shared |
| --- | --- | --- |
| `src/ui.rs`, with `ui/button.rs` and `ui/panel.rs` | the palette and type sizes, `font()`, `label()`, `muted_text()`, `icon()`, `custom_icon()` and `Icon`, `heading()`; re-exported from its parts: `button()`, `menu_button()`, `row_button()`, `icon_button()`, `ButtonKind` (button.rs, with `paint_buttons` and `play_button_sounds`), `floating_root()`, `card()`, `dialog()`, and in dev builds `card_glass()` (panel.rs, with `animate_entrances` and `close_dialog`) | every screen, overlay and dialog is drawn from these; saves, deleting and quitting all ask questions; the dev tools' windows are glass like a card |
| `src/ui/blur.rs` | `GlassLayers` and a camera per layer, `Floating`, `GlassCopies`, the Kawase passes, `GlassMaterial`, `FrozenBackground` with `freeze_background()` and `unfreeze_background()`, `ReduceTransparency`, `ScreensCovered` with `ScreensCoveredSystems` | the pause menu, every overlay and every dialog float, and in dev builds the dev tools' windows |
| `src/input.rs` | `ActionName`, `Action`, `Binding`, `register_action()`, `Controls`, `action_just_pressed()`, `Bindings`, `SavedBindings` | Esc and the dev tools' keys are read as actions, and the settings file keeps the changed bindings; every feature that reads a key registers one |
| `src/i18n.rs` | `Language`, `Translations`, `Localized`, `translate_texts`, `language_from_locale()`, the UI font | every text in every screen and overlay |
| `src/fade.rs` | the `Fade` struct and `alpha()`, `approach()` | the splash, the menu veil, the slideshow, the save status line, card entrances; `approach` eases the buttons |
| `src/assets.rs` | `images_in()` | the splash, the loading screen and the intro read folders |
| `src/save.rs`, with `save/picture.rs` | `Session`, `GameData`, `SaveFinished`, `has_unsaved_progress()`, `autosave_is_due()`, `loaded_session()`, `SaveData`, `SaveEntry`, `newest_ready()`, `saves_on_disk()`, `read_save()`, `write_save()`, `delete_save_files()`, `is_autosave()`; re-exported from picture.rs: `PausePicture`, `take_pause_picture()`, `take_save_picture()`, `save_pause_picture()`, `read_save_picture()`, `PICTURE_ASPECT_RATIO` | gameplay counts play time and autosaves, the pause menu asks before quitting, the menu continues the newest save, the save lists read, write and delete saves, Esc takes the picture a save carries |
| `src/disk.rs` | `game_data_dir()`, `write_atomic()` | saves, their pictures, settings, photo mode's photos and the file explorer's trash all live in the data folder and are written atomically |

**`src/ui.rs`** is the look of the whole game, split the usual way: `ui.rs` keeps the palette,
the type and the text widgets, `ui/button.rs` the buttons, `ui/panel.rs` the floating roots,
cards and dialogs, and `ui/blur.rs` the blur; callers import from `ui`, or `ui::blur` for the blur. Its `Config` holds the palette (`SURFACE`, `BORDER`,
`ACCENT`, `DANGER`, `TEXT`, `TEXT_MUTED`, ...) and the four type sizes, so a game made from the
template recolors in one place. Muted text that stays light is `muted_text(size_px)`, never a
bare `TextColor(TEXT_MUTED)`: it adds the soft `TextShadow` that keeps the faintest text
readable over whatever the glass shows. An icon is `icon(Icon::Trash2)`: a text of one character
in Lucide's font, which the `lucide-icons` crate brings built into the binary with every icon's
name (`Icon`, re-exported so no other file names the crate). `register_icon_font` puts the font
behind `ICON_FONT`, a handle known at compile time, at `Startup`, so `icon()` asks no asset
server. Being text, an icon takes a button's ink like a label. It hands out four buttons,
capsules with no border:
`button(key, kind)` with its label centered, `menu_button(key)` for a column of choices, label to
the left, `row_button(content)` around any bundle, for a list row, with a fixed corner in
place of the capsule's, and `icon_button(icon, kind)`, round, holding one icon and no word. `ButtonKind` is a component saying what a button is for: `Primary` (the
main action, white already at rest), `Choice` (one among equals, white under the pointer: what
`menu_button` and `row_button` are), `Quiet` (bare until hovered, for Close and No) or `Danger`
(red ink, filled red when hovered). `paint_buttons` eases each button's `ButtonMotion` toward
its `Hovered` and `Has<Pressed>` state with `approach`, on `Time<Real>`, and paints it from the
pure `button_colors(kind, hover, press)`: a fill, and an ink, which it writes to every text
under the button at that text's own alpha, so a label turns dark on a white fill and a row's
muted details stay muted. A text marked `OwnColor` is left out: it lies on a surface of its
own, like the icon of a save's empty frame. A button resting where it was last painted is
skipped. A row's corner is derived, `ROW_BUTTON_PADDING_PX` plus `ROW_CONTENT_RADIUS_PX`, the
corner of what the row starts with, so the two curves stay parallel. `floating_root(z)` and `card(width)` are what every panel is
built from; `animate_entrances` brings each new root in, its glass and its card's fading in and
the card rising 12 px and scaling up, by the `ENTRANCE` fade. Its systems carry no `run_if`: a button lives on any screen and inside any overlay,
so there is no state to gate them on. `load_button_sounds` loads `ui/click.wav` and `ui/hover.wav`
once at `Startup` into a `ButtonSounds` resource, and `play_button_sounds` spawns a one-shot player
per hover and per press with `PlaybackSettings::DESPAWN`, so nothing accumulates.
`point_at_pressables` turns the cursor into a pointing hand while the mouse is over anything
that can be pressed, anywhere in the game: it reads Bevy's `HoverMap` and looks, from each
node under the mouse up through those holding it, for a `Button`, `Checkbox`, `RadioButton`,
`Slider`, or a `Hovered` (what lights up under the pointer without being a button, as the
file explorer's tree rows). `paint_buttons` also repaints a resting button whose `ButtonKind`
changed, as a list's row does when it is selected.

**`src/input.rs`** puts a name between a feature and a key. A feature registers an *action*
from its plugin with `register_action(app, Action { name, default, page, label })`, the way a
floating layer adds itself to `GlassLayers`: its `ActionName`, which it keeps as a constant, and the `Binding`
it starts on. `Bindings` is the resource holding every action in the order they were
registered, each with what it is bound to now. A system reads an action through the `Controls`
system parameter (`controls.just_pressed(PAUSE)`), or runs on its press with
`action_just_pressed(PAUSE)`, and never names a `KeyCode` itself. `report_shared_keys` logs two
actions registered on one key as the game starts, found by the pure `shared_default`. On
`main` the actions are Pause (`overlays.rs`, Esc) and, in dev builds, the keys panel
(`dev_tools/keys_panel.rs`, F1), the FPS overlay (`dev_tools/fps.rs`, F2), photo mode
(`dev_tools/photo_mode.rs`, F3, with E taking a photo), slow motion
(`dev_tools/slow_motion.rs`, F4, with Numpad − and + read only while its capsule is open),
speed (`dev_tools/speed.rs`, F8, stepping its multiplier with the same Numpad − and +),
screen recording (`dev_tools/recording.rs`, F9, with E starting and stopping it) and the file
explorer (`dev_tools/file_explorer.rs`, Numpad 0); a branch adds its own. E is one action,
Capture (`CAPTURE` in `dev_tools.rs`), which photo mode and the recording each read in their
own moment, as a key belongs to one action only. Slow
motion's capsule and speed's share their place, their look (the `Step Capsule` section of
`slow_motion.rs`) and their two keys, so opening either closes the other, and each puts its
value back at 1× in an observer on its capsule's `Remove`, however it closed. The keys panel lists the Dev page's actions, each on the keycap of the Controls page
(`keycap()` and `binding_legend()`, re-exported from `overlays.rs` in dev builds); while it is
closed, its own key waits in its place on the screen camera (`spawn_keys_panel_hint`). It
leaves out a tool's inner keys, the ones that only work once the tool is open and that the tool
shows itself (Capture, Slower, Faster): each tool's plugin names its own with
`register_inner_key` in `dev_tools.rs`, and the Dev page still lists them.
An action also has a `Page`, the game's or the dev tools', and a label that page lists it under.
One with no label, Pause, is not listed and never changes: the `swap` refuses its key and a
saved binding for it is ignored. `Controls::pressed` says whether an action is held.
While a text field has the keyboard (`InputFocus` on an `EditableText`: the save name, the file
explorer's rename), `Controls` hears only the unlisted actions, so a letter typed into a name is
never an action too, and Esc still closes what is open (`Bindings::heard`).

A `Binding` is a key or a mouse button. There is one way to change one: the pure `swap`, which
gives whoever held the new binding the one the action gave up, so no two actions ever share a
key and none is left without. `settings.ron` keeps only the bindings that differ from their
defaults (`changed`), so a default changed in a later version still reaches a player who never
touched it. `Bindings::load` lays the saved ones over the defaults through the same `swap`, in
the order the actions were registered: a file edited by hand, or a default that moved onto a
key the player had given another action, still loads with one key an action. A name this build
does not have is set aside and written back by `Bindings::saved` as it was, since one file
serves every branch and both the dev and the release build: opening Settings on `main` must not
erase what was bound on `3d/player`. Writing a `KeyCode` to the file needs `bevy_input`'s
`serialize` feature, which `Cargo.toml` names on a line of its own, the same on every branch.

**`src/i18n.rs`** is what language the game speaks and the words for it. `Language` is a
resource and the only copy of the current language. `Translations` holds one Fluent bundle per
language, built at startup from the `.ftl` files that `include_str!` compiled in; `text(language,
key)` and `text_with(language, key, &args)` fall back to English, then to the key itself, so a
missing word shows where it is missing. `Localized(key)` on an entity (it requires a `Text`) is how
static text is spawned: `translate_texts` in `PostUpdate`, before the UI lays text out, writes it
the frame it spawns and rewrites every one the frame `Language` changes, which is what switches the
menu or the pause menu behind the Settings panel live. Text built from values, like a save's row,
is formatted once with `text_with` as it spawns. `use_ui_font` puts Noto Sans JP, compiled in with
`include_bytes!`, at the default font handle at `Startup`, so every text without a font of its own
draws Portuguese accents and Japanese.

**`src/fade.rs`** is the timing of a fade, not the thing being faded: `delay_secs`, `fade_in_secs`,
`hold_secs`, `fade_out_secs`, and `alpha(elapsed_secs)` returning 0.0 to 1.0. One `FADE_EASING`
constant — `EaseFunction::SmoothStep` — applies to every fade in the game, so none of them feels
different from the others. Each screen builds its own `Fade` and applies the alpha where it needs
it: the splash to an `ImageNode`'s colour, the menu to the veil's `BackgroundColor` *and* to the
music's volume, the loading screen to the current slide.

**`src/assets.rs`** has no plugin and no system. `images_in` takes a `LoadedFolder` and returns its
images sorted by file path, leaving anything that is not an image out. Zero-padded file names are
what keep a sequence in step, which is why the intro's frames sort correctly.

**`src/save.rs`** is plain functions, and its plugin only adds its picture's. `Session` holds `play_secs` and `saved_play_secs`, and
`has_unsaved_progress` is a pure function over it, tested at the bottom of the file.
`loaded_session` turns a save into a session with all its play time counted as saved, so a game
just loaded neither asks before quitting nor autosaves at once. `newest_ready` picks what
Continue loads by `saved_at_secs`, never by list order, which no OS promises.

Saves are one RON file each in `saves/` inside the data folder (`src/disk.rs`, below), named
`save_<millis>.ron`, or
`autosave_<millis>.ron` for an autosave; the prefix is how `is_autosave` tells them apart.
`SaveData` is what a file holds: a version, the player's name for it, when it was saved and the
play time. `saves_on_disk` returns a `SaveEntry` for every `.ron` file there (through the private
`list_saves`, which takes the folder so its tests can use a temporary one), `Ready`, `Incompatible`
(another `SAVE_VERSION`) or `Unreadable`, so the list can show a file it cannot load instead of
hiding it; the sorting into those three is `parse_save`, a pure function over the file's text.
The folder hands files back in no promised order, so `sort_newest_first` puts the newest save
first and the files without a date, the ones it cannot load, last; both lists show that order.
`read_save` reads one file back, `None` when it is gone, unreadable or from another version.
`write_save` is the one way a save reaches the disk: it writes through `write_atomic`, which
creates the folder when it is missing, and only then sets `saved_play_secs`, so a failed write still
counts as unsaved progress. `delete_save_files` removes a save and its picture, and a save that
has no picture is deleted all the same.

**`src/save/picture.rs`** is the picture a save carries: a 320 × 180 JPEG beside it, under its
name (`save_1.ron`, `save_1.jpg`; `picture_path`), which `list_saves` never takes for a save,
since it reads only `.ron`. It is a screenshot of the window (`Screenshot::primary_window()`,
whose observer gets the image a few frames later), so it shows what the player saw, whatever
the genre draws. The pure `picture_crop` keeps the largest 16:9 part around the window's
center, and `encode_picture` crops, shrinks and encodes it through the `image` crate Bevy
already brings, reached through `Image::try_into_dynamic`, so `Cargo.toml` names no new crate.
That work never runs on the main thread: each observer hands the capture to `encode_capture`,
which takes the image out of the event, without copying it, and encodes it in a task on the
`AsyncComputeTaskPool`. The first screenshot of a run makes Bevy build the pipeline that draws
a captured frame back to the window, and that frame is not drawn until it is built, so
`warm_up_screenshots` takes one nobody keeps as the first loading screen opens, where the
frame lost comes in from black.

A picture is taken at two moments. While the game runs, `take_save_picture(path)` writes it
beside the save at `path` when it is done: the autosave's. Under the pause menu the window
shows the menu, so the picture is taken before: `take_pause_picture`, on the frame Esc asks
for the pause, keeps the encoded bytes in the `PausePicture` resource, and
`save_pause_picture(path, picture)` writes them beside each save made under that pause. Each
pause gets a `PausePicture` of its own, so a capture that arrives late fills one nobody reads
any more, and `forget_pause_picture` empties it on leaving gameplay: a save never carries the
picture of an earlier pause or of another game. A save made before its capture is ready has
no picture, and an overwritten save then loses the one it had, which would show another
moment. The save is always written first, and its picture's failure is only logged: a save
with no picture is a save.

`read_save_picture(path)` reads a picture back for the lists, `None` when the save has none or
its file is not one (`decode_picture`). It runs on the main thread, a file per save, as a list
opens: quick for the saves a player keeps by hand, and marked `ponytail:` for the player with
hundreds of autosaves.

**`src/disk.rs`** is where files live and how they are written. `data_dir(os, var)` picks the
per-user data folder of each OS plus the package name, and takes the environment as a function
so the tests can hand it Windows, Linux and macOS on any machine; `game_data_dir()` calls it
with the real ones. `write_atomic` writes text or bytes to the target's own name plus `.tmp`
and renames it over, so a crash mid-write leaves the old file whole, and a save and its picture
never share a temporary file; it is tested against a real temporary folder.

**The dialog** in `src/ui/panel.rs` answers "Yes or No?" for any feature without knowing what it asks.
`dialog(question, yes_kind, on_yes)` returns the bundle: a `floating_root` with a small card,
`question` being a translation key and `yes_kind` the Yes button's kind, `Danger` when Yes destroys
something. Both buttons carry `CloseDialog`, and the Yes also carries the caller's `on_yes` marker. `close_dialog` despawns
the `Dialog` root on either answer, and the caller's own observer sees the same press and does the
work. `DIALOG_Z_INDEX` is 200, above the overlays.

## Assets each screen reads

| Path | Read by | Notes |
| --- | --- | --- |
| `assets/splash/bevy.png` | `splash.rs` | shown first; keep it unmodified, see [template.md](template.md) |
| `assets/splash/logos/` | `splash.rs` | every image, in file name order, after the Bevy logo |
| `assets/menu/background.jpg` | `menu.rs` | stretched to the window |
| `assets/menu/music.ogg` | `menu.rs` | loops for as long as the menu is on screen |
| `assets/loading/` | `loading.rs` | any image in the folder is a slide, shuffled |
| `assets/intro/frames/` | `loading.rs` loads, `intro.rs` plays | sorted by file name |
| `assets/intro/music.ogg` | `loading.rs` loads, `intro.rs` plays | loaded early, starts with frame one |
| `assets/ui/click.wav`, `assets/ui/hover.wav` | `ui/button.rs` | loaded once at `Startup` |
| `assets/locales/<code>/main.ftl` | `i18n.rs` | compiled in with `include_str!`, not loaded |
| `assets/fonts/NotoSansJP-Regular.otf` | `i18n.rs` | compiled in with `include_bytes!`, not loaded |
| `assets/map.ron` | `gameplay/map.rs` | the default map, a new game's props; written by Save as Default Map in dev builds |
| `assets/props/` | `gameplay/map.rs`, the prop editor | the props the editor offers and the map places |

Dropping a file into `assets/splash/logos/`, `assets/loading/` or `assets/intro/frames/` is enough:
those three read the whole folder, so no code changes to add or remove one.

## How to add a screen

1. Add the variant to `Screen` in `src/screens.rs`, **in flow order**. The enum is read as the map
   of the game, so the order it is written in should be the order the player meets it.
2. Create `src/screens/<name>.rs` with `pub(super) fn plugin(app: &mut App)` and the file layout
   from `CONTRIBUTING.md`.
3. Register it in `screens.rs`, again in flow order, next to the screens around it.
4. Spawn on `OnEnter(Screen::X)` and give the root entity `DespawnOnExit(Screen::X)`, so the screen
   tears itself down. Anything that must outlive the screen — like the intro's handles — goes in a
   resource instead, and something has to remove it later.
5. Gate the `Update` systems with `.run_if(in_state(Screen::X))`, and make the order explicit with
   `.chain()` or `.after(...)` wherever one system reads what another writes.
6. Change the screen before it to point at the new one. There is no routing table: each screen sets
   `NextState<Screen>` for its own successor, so adding one in the middle is one line in the
   screen above it.

An overlay is the same shape with `Overlay` in place of `Screen`: an `overlay_root()` holding an
`overlay_card()` that starts with `heading(key)` and ends with `close_footer()`.
