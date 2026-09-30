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
              ^       waits on      |          |
              |       the intro     +-- skip ->+
              |       assets                   |
              +<------ Quit to Menu -----------+  (from the pause menu)
              |
              +--> Quit --> AppExit::Success
```

- `Screen::Splash` is the `#[default]`, so the game starts there.
- The plugins are registered in `screens.rs` in the same order the player meets them:
  `splash`, `menu`, `loading`, `intro`, `gameplay`.
- A skip in `Splash` is not an edge on this diagram: it ends the current logo, and the sequence
  leaves for the menu on its own once the last one is done.
- Quit to Menu out of `Gameplay` and the skip out of `Intro` are the only edges that are not the
  plain next step. There is no way back into `Splash`, `Loading` or `Intro`.

## The overlay flow

```
  Overlay::None  (the #[default] — nothing on top)
        |
        +--[Settings button]--> Overlay::Settings --+
        |                                           +--> Close button / Esc --> None
        +--[Credits button]---> Overlay::Credits ---+
        |                                           |
        +--[Save button]------> Overlay::SaveGame --+   (pause menu only; a save closes it too)
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
        +--[Settings]--------> Overlay::Settings --Close / Esc--> back to Pause::On
        +--[Quit to Menu]----> (dialog, if progress is unsaved) --Yes--> Screen::Menu
        +--[Quit to Desktop]-> (dialog, if progress is unsaved) --Yes--> AppExit::Success
```

A dialog is not a state: it is an entity drawn over whatever asked the question, the pause menu
or the save list, and No or Esc despawns it and leaves what is underneath as it was.

`Pause` is a `SubStates` of `Screen::Gameplay`: it only exists while gameplay does. Leaving
gameplay removes it, so the next game starts unpaused without any code resetting it, and its
`OnExit` still runs, so a game quit while paused never leaves the clock frozen.

## Screen by screen

| Screen | Spawned by | Ends when | Ends by |
| --- | --- | --- | --- |
| `Splash` | `spawn_splash_screen` | the last logo is done, or skipped | `advance_splash_logos` |
| `Menu` | `spawn_menu_screen`, `play_menu_music` | the closing veil finishes | `animate_veils` |
| `Loading` | `spawn_loading_screen`, `load_intro_assets` | the intro's assets are in | `leave_loading_screen` |
| `Intro` | `spawn_intro_screen`, `play_intro_music` | the last frame is done, or skipped | `leave_intro_screen`, `skip_intro_screen` |
| `Gameplay` | `spawn_gameplay_screen` | Quit to Menu on the pause menu | `activate_pause_button` (`pause.rs`) |

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

Four layers under one root, spawned back to front: the background image, a gradient scrim that is
dark on the left and clear on the right, the button column, and a black veil.
`assets/menu/music.ogg` loops beside it with `DespawnOnExit(Screen::Menu)`, so the track's
lifetime is the screen's.

The buttons carry a `MenuAction` component, and the `activate_menu_button` observer reads it off
`activate.entity`:

| Button | What it does |
| --- | --- |
| Play | spawns a second veil, this one fading *in* to black |
| Settings | sets `Overlay::Settings` |
| Credits | sets `Overlay::Credits` |
| Quit | writes `AppExit::Success` |

Play does not set the next screen itself. `animate_veils` does, once the closing veil's timer
finishes — that indirection is what makes the fade visible before the screen changes. Which screen
comes next is `screen_after_veil`'s answer, a pure function with its own tests. The entrance
veil is despawned when it finishes instead of being left at alpha 0, because a transparent
full-window node would still swallow every click.

`update_menu_music_volume` runs `.after(animate_veils)` so the sound never lags the picture by a
frame. It writes the volume every frame on purpose: `GlobalVolume` is only read when a sink starts,
so the settings slider would otherwise do nothing to a track that is already playing.

### Loading

A slideshow over black with a progress bar along the bottom. `load_intro_assets` starts loading
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

A full-window background colour and nothing else yet. `start_session` inserts a fresh `Session` on
entry, and `count_play_time` adds each frame's virtual time to it, so the pause menu never counts
as play. Until `StartGame` exists every game starts as a new one; the resource is also created at
startup, because the pause menu's observer reads it on every button press in the app. Esc lives in
`src/overlays.rs` with the rest of the "go back" rule.

`autosave_game` runs `.after(count_play_time)` and only in `Pause::Off`. Once
`autosave_is_due`, after `AUTOSAVE_INTERVAL_SECS` of unsaved play, it writes a new
`autosave_<millis>.ron` named `Autosave`; every autosave is kept. It first sets
`autosave_tried_secs`, and the interval counts from that or the last save, whichever is later,
so a failed autosave is logged once and tried again a full interval later instead of every
frame. An autosave counts as a save, so quitting right after one asks nothing.

**The pause menu** (`src/screens/gameplay/pause.rs`, registered by `gameplay.rs`'s plugin).
`OnEnter(Pause::On)` spawns the panel with `DespawnOnExit(Pause::On)` over a half-black backdrop,
so the frozen game shows dimmed behind it, and calls `Time<Virtual>::pause()`. `OnExit` calls
`unpause()`. Any gameplay written later that reads `Res<Time>` stops while paused for free, with
no `run_if` of its own.

The buttons carry a `PauseAction` component, read by the `activate_pause_button` observer, the
same shape as the menu's:

| Button | What it does |
| --- | --- |
| Resume | sets `Pause::Off` |
| Save | sets `Overlay::SaveGame`, drawn above the pause menu; how the save went shows under the buttons |
| Settings | sets `Overlay::Settings`, drawn above the pause menu |
| Quit to Menu | sets `Screen::Menu`, or asks first when `has_unsaved_progress`; `Pause` goes with the screen |
| Quit to Desktop | writes `AppExit::Success`, or asks first when `has_unsaved_progress` |

"Quit without saving?" is `dialog()` from `ui.rs`, spawned with `DespawnOnExit(Pause::On)` so it
cannot outlive the pause menu. Its Yes carries `ConfirmQuit(QuitTo)`, and the `confirm_quit`
observer reads where to go from it. Play time within `UNSAVED_GRACE_SECS` of the last save does
not count, so quitting right after starting never asks.

## Overlays

Every panel follows the same three lines: spawn on `OnEnter(Overlay::X)`, build on
`overlay_root()`, and carry `DespawnOnExit(Overlay::X)`. `overlay_root()` in `src/overlays.rs` is
the full-window node every overlay shares: centred column, the row gap, a dark backdrop and
`OVERLAY_Z_INDEX`. The backdrop is there because an overlay can now open over the pause menu, and
without it the pause buttons would show through.

- **Credits** (`src/overlays/credits.rs`): static text and a Close button.
- **Settings** (`src/overlays/settings.rs`): a fullscreen checkbox and a 0–100 volume slider, both
  from `bevy_ui_widgets`, plus Close. Its `update_fullscreen_checkbox` and `update_volume_thumb`
  are gated with `.run_if(in_state(Overlay::Settings))`; the widgets are headless, so those two
  systems are what actually draw the state. Both choices are remembered in `settings.ron` in the
  game's data folder: `load_settings` applies it at `Startup`, and `save_settings` writes it on
  `OnExit(Overlay::Settings)`, once per visit rather than on every slider move. The `Window` and
  `GlobalVolume` stay the only copy of the state; the file is read from them and written back to
  them. A missing, unreadable or other-version file keeps the defaults silently, since
  `parse_settings` returns `None` for all three; a failed write is logged with `error!`.
- **Save Game** (`src/overlays/saves.rs`): a name field, a New Save button, then one row per file
  in the saves folder, then Close. The field is an `EditableText` marked `SaveName`, capped at
  `SAVE_NAME_MAX_CHARS`; the widget is headless, so the field's own `BackgroundColor` and a
  `TextCursorStyle` are what make it visible, and without the latter no cursor is drawn.

  The folder is read once, on `OnEnter(Overlay::SaveGame)`. The rows are spawned in a `for`
  with `ChildOf(list)`, because `children![]` only holds a list known when the code is
  written; Close is spawned after them for the same reason, so it ends up last. `list` is a
  column capped at `SAVE_LIST_MAX_HEIGHT_PX` with `Overflow::scroll_y()` and `ScrollArea`, so
  the mouse wheel scrolls the rows while the title, the field, New Save and Close stay put; it
  reuses `OVERLAY_GAP_PX`, since the panel's own gap only spaces its direct children. A save
  the game can read is a button labelled by `save_label`, a pure function:
  `Castle · 1h 01m played · 3d ago`. An incompatible or unreadable file is plain text, so nothing
  that looks clickable does nothing. Autosaves are left out (`is_autosave` on each entry's
  `path()`): overwriting one would keep its name and turn a player's save into an "Autosave".

  | Press | Observer | What it does |
  | --- | --- | --- |
  | New Save | `save_new_game` | writes the session to a new `save_<millis>.ron`, named by the field, trimmed |
  | a save's row | `ask_to_overwrite_save` | spawns "Overwrite this save?", its Yes carrying `ConfirmOverwrite(path, name)` |
  | that Yes | `overwrite_save` | writes the session over that file, keeping the save's own name |

  Both writes go through `save_status`, which calls `write_save` and returns `SAVED_TEXT` or
  `SAVE_FAILED_TEXT`, logging the reason with `error!` on a failure. Either way the overlay
  closes and the text lands on the pause menu's status line, a `Text` marked `SaveStatus` that
  `pause.rs` spawns empty; no data folder at all counts as a failure too. Writing the line also
  sets its `shown_secs` to zero, and `fade_save_status` in `pause.rs` shows it at once, holds it
  and fades it out by `STATUS_FADE`, counting `Time<Real>` because virtual time is frozen while
  paused. The dialog carries
  `DespawnOnExit(Overlay::SaveGame)`, so it cannot outlive the list.

**Closing.** No panel owns its Close button's behaviour. Each spawns `(button("Close"),
CloseOverlay)`, and the single `close_overlay` observer in `src/overlays.rs` sets `Overlay::None`
when the activated entity carries the marker. `go_back_on_esc` is the second way out, and it holds
the whole "go back" rule in one place, first match wins:

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
| dialog | 200 | `DIALOG_Z_INDEX`, in `ui.rs` |

## Shared code

Four files at the top of `src/`, each named after its domain, each used by more than one feature.

| File | What it holds | Why it is shared |
| --- | --- | --- |
| `src/ui.rs` | `button()`, `dialog()`, the palette, `paint_buttons`, `play_button_sounds`, `close_dialog` | buttons in every screen; saves, deleting and quitting all ask questions |
| `src/fade.rs` | the `Fade` struct and `alpha()` | the splash, the menu veil and the slideshow |
| `src/assets.rs` | `images_in()` | the splash, the loading screen and the intro read folders |
| `src/save.rs` | `Session`, `has_unsaved_progress()`, `autosave_is_due()`, `SaveData`, `SaveEntry`, `list_saves()`, `write_save()`, `is_autosave()`, `game_data_dir()`, `write_atomic()` | gameplay counts play time and autosaves, the pause menu asks before quitting, the save list reads and writes saves, settings are written to disk |

**`src/ui.rs`** hands out the button bundle and paints every button in the app from `Hovered` and
`Has<Pressed>`. Its systems carry no `run_if`: a button lives on any screen and inside any overlay,
so there is no state to gate them on. `load_button_sounds` loads `ui/click.wav` and `ui/hover.wav`
once at `Startup` into a `ButtonSounds` resource, and `play_button_sounds` spawns a one-shot player
per hover and per press with `PlaybackSettings::DESPAWN`, so nothing accumulates.

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

Its `Disk` section is where files live and how they are written. `data_dir(os, var)` picks the
per-user data folder of each OS plus the package name, and takes the environment as a function
so the tests can hand it Windows, Linux and macOS on any machine; `game_data_dir()` calls it
with the real ones. `write_atomic` writes to a `.tmp` beside the target and renames it over, so
a crash mid-write leaves the old file whole, and it is tested against a real temporary folder.

Saves are one RON file each in `saves/` inside that data folder, named `save_<millis>.ron`, or
`autosave_<millis>.ron` for an autosave; the prefix is how `is_autosave` tells them apart.
`SaveData` is what a file holds: a version, the player's name for it, when it was saved and the
play time. `list_saves` returns a `SaveEntry` for every `.ron` file there, `Ready`, `Incompatible`
(another `SAVE_VERSION`) or `Unreadable`, so the list can show a file it cannot load instead of
hiding it; the sorting into those three is `parse_save`, a pure function over the file's text.
`write_save` is the one way a save reaches the disk: it creates the folder when it is missing,
writes through `write_atomic`, and only then sets `saved_play_secs`, so a failed write still
counts as unsaved progress.

**The dialog** in `src/ui.rs` answers "Yes or No?" for any feature without knowing what it asks.
`dialog(question, on_yes)` returns the bundle; both buttons carry `CloseDialog`, and the Yes also
carries the caller's `on_yes` marker. `close_dialog` despawns the `Dialog` root on either answer,
and the caller's own observer sees the same press and does the work. `DIALOG_Z_INDEX` is 200,
above the overlays.

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
| `assets/ui/click.wav`, `assets/ui/hover.wav` | `ui.rs` | loaded once at `Startup` |

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

An overlay is the same shape with `Overlay` in place of `Screen`, with `overlay_root()` as the
root and `(button("Close"), CloseOverlay)` for the way out.
