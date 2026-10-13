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
```

The overlay is drawn over whatever screen is below, and that screen keeps running: entering
an overlay never changes `Screen`, so no `DespawnOnExit(Screen::X)` fires and nothing is lost.
That is why the menu music keeps playing while Settings is open.

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
```

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
`menu_button`s), the version in the bottom-right corner from `CARGO_PKG_VERSION`, and a black veil.
`assets/menu/music.ogg` loops beside it with `DespawnOnExit(Screen::Menu)`, so the track's
lifetime is the screen's.

The buttons carry a `MenuAction` component, and the `activate_menu_button` observer reads it off
`activate.entity`:

| Button | What it does |
| --- | --- |
| New Game | triggers `StartGame` with a fresh `Session`, headed for the intro |
| Continue | triggers `StartGame` with the newest save `newest_ready` finds, headed for gameplay |
| Load Game | sets `Overlay::LoadGame` |
| Settings | sets `Overlay::Settings` |
| Credits | sets `Overlay::Credits` |
| Quit | writes `AppExit::Success` |

Continue is only there when a save can be loaded: `update_continue_button`, chained after
`spawn_menu_screen`, sets its `Node::display` to `Display::None` otherwise, which takes it out of
the column so the buttons below close the gap. Its click lists the saves again rather than trust
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
frame. An autosave counts as a save, so quitting right after one asks nothing.

**The pause menu** (`src/screens/gameplay/pause.rs`, registered by `gameplay.rs`'s plugin).
`OnEnter(Pause::On)` spawns the panel with `DespawnOnExit(Pause::On)`: a `floating_root` holding a
`card`, so the frozen game shows blurred and dimmed behind it (see "Two cameras and the blur").
It also calls `Time<Virtual>::pause()`, and `OnExit` calls `unpause()`. Any gameplay written later
that reads `Res<Time>` stops while paused for free, with no `run_if` of its own.

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
`floating_root(OVERLAY_Z_INDEX)` from `ui.rs`: the whole window, `Floating`, a dim that comes in
with the card. The dim matters when an overlay opens over the pause menu: the pause card darkens
under it instead of competing with it.

- **Credits** (`src/overlays/credits.rs`): a translated line, the names as plain text, and a Close
  button.
- **Settings** (`src/overlays/settings.rs`): a grid of `setting_row`s, each a label in a column of
  its own beside its control: a fullscreen checkbox and a 0–100 volume slider, both from
  `bevy_ui_widgets`, and the language radios. The slider is as tall as its thumb, with a thin
  track through it and a `VolumeFill` up to the thumb, in the accent. Its
  `update_fullscreen_checkbox`, `update_volume_slider` and `update_language_radios` are gated
  with `.run_if(in_state(Overlay::Settings))`; the widgets are headless, so those systems are
  what actually draw the state. All three choices are remembered in `settings.ron` in the
  game's data folder: `load_settings` applies it at `Startup`, and `save_settings` writes it on
  `OnExit(Overlay::Settings)`, once per visit rather than on every slider move. The `Window` and
  `GlobalVolume` stay the only copy of the state; the file is read from them and written back to
  them. A missing, unreadable or other-version file keeps the defaults silently, since
  `parse_settings` returns `None` for all three; a failed write is logged with `error!`.
  A third row picks the language: a `RadioGroup` of one `RadioButton` per `Language::ALL`, each
  named in its own language. `choose_language` observes the group's `ValueChange<Entity>` and sets
  the `Language` resource; `update_language_radios` checks and paints the one that matches it.
  The language goes into `settings.ron` with the rest, as `Some(language)`; a file without one,
  or no file, follows the OS through `sys_locale` and `language_from_locale`.
- **Save Game** (`src/overlays/saves.rs`): a name field, a New Save button, then one row per file
  in the saves folder, then Close. The field is an `EditableText` marked `SaveName`, capped at
  `SAVE_NAME_MAX_CHARS`; the widget is headless, so the field's own fill, border and a
  `TextCursorStyle` are what make it visible, and without the latter no cursor is drawn.
  `EditableText` only holds the text: `TextInput` is what types into it, and `TabIndex` is what
  lets a click give it the keyboard, since Bevy hands focus on a click only to an entity with
  one. `paint_save_name_field` outlines it in the accent while `InputFocus` is on it. Each save is a
  `row_button` holding `save_row`: `save_name` over `save_details` in small muted text.

  The folder is read once, on `OnEnter(Overlay::SaveGame)`. The rows are spawned in a `for`
  with `ChildOf(list)`, because `children![]` only holds a list known when the code is
  written; Close is spawned after them for the same reason, so it ends up last. `list` is a
  column capped at `SAVE_LIST_MAX_HEIGHT_PX` with `Overflow::scroll_y()` and `ScrollArea`, so
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
  failure too. Either way the overlay triggers `SaveFinished { saved }` (in `save.rs`) and
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

  Every row also has a Delete button carrying `DeleteSave(path)`, the files the game cannot load
  included, since this is the only way they leave the list. `ask_to_delete_save` spawns "Delete
  this save?" with `DespawnOnExit(Overlay::LoadGame)`, its Yes carrying `ConfirmDelete(path)`, and
  `delete_save` removes the file and sets `Overlay::LoadGame` again. `set` re-enters a state it is
  already in, so `OnExit` despawns the panel and `OnEnter` reads the folder anew: the list loses
  the row with no code to rebuild it. A failed delete is logged with `error!` and the row stays.
  That same `OnExit` runs the menu's `update_continue_button` again, so deleting the last save hides
  Continue without leaving the menu.

**Closing.** No panel owns its Close button's behaviour. Each ends its card with
`close_footer()`, which holds `(button("overlay-close", ButtonKind::Quiet), CloseOverlay)`, and
the single `close_overlay` observer in `src/overlays.rs` sets `Overlay::None` when the activated
entity carries the marker. `apply_esc_action` is the second way out, run only on the frame Esc is
pressed (`input_just_pressed`); with nothing open during running gameplay, the same press pauses
instead. The whole Esc rule lives in one pure function beside it,
`esc_action`, which returns an `EscAction` and has a test per line, first match wins:

1. a dialog is open → close it
2. an overlay is open → close it (back to the pause menu, if paused)
3. gameplay is paused → resume
4. gameplay is running → pause

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

Those numbers order roots *within a camera*. Every `Floating` root (pause, overlays, dialogs) is
drawn by the floating camera, after the screen camera, so all of them are above every screen
whatever its `GlobalZIndex`.

## Two cameras and the blur

`main.rs` spawns the **screen camera**: `Camera2d`, `IsDefaultUiCamera`, and the `BlurStrength`
that `src/ui/blur.rs` drives. Every UI root without a `UiTargetCamera` is drawn by it: the splash,
the menu, loading, the intro, gameplay.

`src/ui/blur.rs` spawns the **floating camera** at `Startup`: order 1, so it draws after, and
`ClearColorConfig::None`, so it draws over the screen camera's image instead of wiping it. An
observer on `Add<Floating>` gives each new `Floating` root a `UiTargetCamera` pointing at it.
Clicks need nothing: Bevy's UI picking tests both cameras and puts the later one's hits first.

`update_blur` moves `BlurStrength` toward 1 while any `Floating` exists and toward 0 when none
does, over `BLUR_RAMP_SECS`, on `Time<Real>` because the pause menu freezes virtual time. While
it is above 0, the screen camera carries `BlurHorizontal` and `BlurVertical`, two
`FullscreenMaterial`s running `assets/shaders/blur.wesl` across then down. Both are ordered
`.after(ui_pass).before(upscaling)` in `Core2d`: after the UI so the menu's buttons blur too, and
outside Bevy's own post-processing, which runs before the UI. At 0 both components are removed
and no pass runs. Three settings must stay as they are: `IsDefaultUiCamera` on the screen camera
(or every root moves to the floating one), `ClearColorConfig::None` on the floating camera (or the
screens vanish), and the same HDR and MSAA on both (or they stop sharing the image).

## Shared code

Six files at the top of `src/`, each named after its domain, each used by more than one feature.

| File | What it holds | Why it is shared |
| --- | --- | --- |
| `src/ui.rs`, with `ui/button.rs` and `ui/panel.rs` | the palette and type sizes, `font()`, `label()`, `heading()`; re-exported from its parts: `button()`, `menu_button()`, `row_button()`, `ButtonKind` (button.rs, with `paint_buttons` and `play_button_sounds`), `floating_root()`, `card()`, `dialog()` (panel.rs, with `animate_entrances` and `close_dialog`) | every screen, overlay and dialog is drawn from these; saves, deleting and quitting all ask questions |
| `src/ui/blur.rs` | the floating camera, `Floating`, `BlurStrength`, the two blur passes | the pause menu, every overlay and every dialog float |
| `src/i18n.rs` | `Language`, `Translations`, `Localized`, `translate_texts`, `language_from_locale()`, the UI font | every text in every screen and overlay |
| `src/fade.rs` | the `Fade` struct and `alpha()`, `approach()` | the splash, the menu veil, the slideshow, the save status line, card entrances; `approach` eases the blur and the buttons |
| `src/assets.rs` | `images_in()` | the splash, the loading screen and the intro read folders |
| `src/save.rs` | `Session`, `GameData`, `SaveFinished`, `has_unsaved_progress()`, `autosave_is_due()`, `loaded_session()`, `SaveData`, `SaveEntry`, `newest_ready()`, `saves_on_disk()`, `read_save()`, `write_save()`, `is_autosave()` | gameplay counts play time and autosaves, the pause menu asks before quitting, the menu continues the newest save, the save lists read and write saves |
| `src/disk.rs` | `game_data_dir()`, `write_atomic()` | saves and settings both live in the data folder and are written atomically |

**`src/ui.rs`** is the look of the whole game, split the usual way: `ui.rs` keeps the palette,
the type and the text widgets, `ui/button.rs` the buttons, `ui/panel.rs` the floating roots,
cards and dialogs, and `ui/blur.rs` the blur; callers import from `ui`, or `ui::blur` for the blur. Its `Config` holds the palette (`SURFACE`, `BORDER`,
`ACCENT`, `DANGER`, `TEXT`, `TEXT_MUTED`, ...) and the four type sizes, so a game made from the
template recolors in one place. It hands out three buttons, all pills with a thin border:
`button(key, kind)` with its label centered, `menu_button(key)` for a column of choices, label to
the left, and `row_button(content)` around any bundle, for a list row. `ButtonKind` is a component:
`Primary`, `Quiet` (no border until hovered, for Close and No) or `Danger` (red when hovered).
`paint_buttons` eases each button's `ButtonMotion` toward its `Hovered` and `Has<Pressed>` state
with `approach`, on `Time<Real>`, and paints its fill, border and glow from the pure
`button_colors(kind, hover, press)`. `floating_root(z)` and `card(width)` are what every panel is
built from; `animate_entrances` brings each new root in, its dim darkening and its card rising
12 px and scaling up, by the `ENTRANCE` fade. Its systems carry no `run_if`: a button lives on any screen and inside any overlay,
so there is no state to gate them on. `load_button_sounds` loads `ui/click.wav` and `ui/hover.wav`
once at `Startup` into a `ButtonSounds` resource, and `play_button_sounds` spawns a one-shot player
per hover and per press with `PlaybackSettings::DESPAWN`, so nothing accumulates.

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

**`src/save.rs`** has no plugin yet. `Session` holds `play_secs` and `saved_play_secs`, and
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
counts as unsaved progress.

**`src/disk.rs`** is where files live and how they are written. `data_dir(os, var)` picks the
per-user data folder of each OS plus the package name, and takes the environment as a function
so the tests can hand it Windows, Linux and macOS on any machine; `game_data_dir()` calls it
with the real ones. `write_atomic` writes to a `.tmp` beside the target and renames it over, so
a crash mid-write leaves the old file whole, and it is tested against a real temporary folder.

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
