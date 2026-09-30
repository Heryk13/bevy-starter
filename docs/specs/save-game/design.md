# Saving and loading games, and the pause menu that needs them

Status: **in progress** — steps 1 to 4 built. Anything still bothering us is settled while
building it, and recorded at the bottom under "What changed while building it".
Date: 2026-09-23
Bevy: 0.19.1

## Goal

The menu offers Play and nothing else: every run starts from the intro, and quitting loses
everything. This design adds saved games, the menu buttons that reach them, and the pause
menu that saving from gameplay needs.

`Screen::Gameplay` is still an empty background, so there is almost nothing to save. That is
the point: the *mechanism* (where files live, their format, versioning, corruption, the
lists, the flow) is built and tested now, and the gameplay that arrives later adds its
fields to one struct without touching any of it.

The previous spec already reserved room for this work, and it is followed here: the New
Game / Continue / Load Game trio, Continue greyed out with `InteractionDisabled`, a loading
screen that gets its destination from its caller ("Load Game will not play the intro"), and
Esc in gameplay opening a pause menu instead of leaving.

## Decided

| Question | Decision |
| --- | --- |
| What a save holds | Metadata only: format version, when it was saved, time played |
| How many saves | Unlimited: one file per save, listed newest first |
| Save button | Opens the list: "New save" on top, or overwrite an existing one after a confirmation |
| Load Game | The same list, from the menu and from the pause menu |
| Delete a save | Yes, from the Load Game list, after a confirmation |
| Continue | Loads the newest readable save; greyed out when there is none |
| Intro | Plays on New Game only |
| Pause menu | Resume, Save, Load Game, Settings, Quit to Menu, Quit to Desktop |
| Quitting with unsaved progress | Asks "Quit without saving?" first |
| Settings | Fullscreen and volume are remembered between runs |
| Where files live | The OS's per-user data folder, found with `std` only |
| File format | RON through `serde` |
| Pause and overlays | Pause is its own state under `Screen::Gameplay`; overlays draw above it |
| Autosave | One `autosave.ron`, rewritten after every 5 minutes of unsaved play |

## Out of scope

Each is its own design cycle, not a "while we are here":

- **Quicksave and quickload** (F5 / F9). Common on PC, not universal; it is one more slot
  and two keys on top of what is built here.
- **Autosave at gameplay moments** (a level cleared, a checkpoint). Only the timed autosave
  is built; the gameplay decides its own moments once it has any.
- **Naming saves, thumbnails, screenshots.** A save is labelled by play time and age.
- **Migrating old saves.** `version` is written and checked from day one; a save of a
  version the game does not know is listed as incompatible. Actual migration code waits
  for a version 2 to exist.
- **Loading and saving off the main thread.** A save is a few dozen bytes; see the
  `ponytail:` note in "Storage".
- **Cloud saves, Steam, WASM storage.** Desktop only, like the rest of the template.
- **Keyboard and gamepad navigation**, still out of scope as in the previous spec. The long
  list scrolls with the mouse wheel.
- **Anything inside `Screen::Gameplay`** beyond counting play time.

## Structure

Three states, each answering one question:

| State | Kind | Answers | Values |
| --- | --- | --- | --- |
| `Screen` | `States` | where the player is | unchanged: Splash, Menu, Loading, Intro, Gameplay |
| `Pause` | `SubStates` of `Screen::Gameplay` | is the game frozen | `Off`, `On` |
| `Overlay` | `States` | what is drawn on top | None, Credits, Settings, **LoadGame**, **SaveGame** |

### Why `Pause` is not an overlay

Every overlay closes to `Overlay::None`. With a pause menu underneath, Pause → Settings →
Close must land back on the pause menu, which `docs/flow.md` already called out. Three ways
were weighed:

- **A pause state under the overlays** (chosen). Closing an overlay still goes to
  `Overlay::None`, the pause menu is still there underneath, and it shows again by itself.
  `CloseOverlay` keeps having "nothing to compute, remember or get wrong".
- **`Overlay::Pause`, with Close computing where to go back to.** One state fewer, but Close
  gains a rule, exactly what the previous spec argued against, and each new overlay makes
  that rule longer.
- **A stack of overlays in a `Vec`.** The most general, but it is no longer a `States`:
  `OnEnter`, `DespawnOnExit` and `in_state` stop working and would be rebuilt by hand.

The one cost of the choice: overlays now draw over other panels, so every overlay gets a
dark backdrop (`overlay_root()`, below). It also reads better over the menu artwork.

### Why `SubStates`

`Pause` only exists while `Screen::Gameplay` does
(`bevy_state-0.19.1/src/state/sub_states.rs:148`, registered with `app.add_sub_state`).
Leaving gameplay removes it, and the next game starts unpaused without any code resetting
it. `DespawnOnExit(Pause::On)` still fires when the whole screen goes, because removing the
state runs its `OnExit`.

### Pausing freezes time

`OnEnter(Pause::On)` calls `Time<Virtual>::pause()` and `OnExit` calls `unpause()`
(`bevy_time-0.19.1/src/virt.rs:215`). `Res<Time>` in `Update` follows virtual time, so any
gameplay written later that reads `Res<Time>` stops while paused for free, and so does the
play time counter.

Because `OnExit(Pause::On)` also runs when gameplay is left while paused, the menu never
starts with time frozen, which would freeze its veil.

### A confirmation is not a state

"Overwrite this save?", "Delete this save?" and "Quit without saving?" appear *over* a list
or over the pause menu. As an `Overlay` variant, answering one would go to `Overlay::None`
and throw away the list it was asked from. So a dialog is an entity:

```rust
/// Returns a modal question with Yes and No, ready to spawn over whatever asked it.
pub(crate) fn dialog(question: &str, on_yes: impl Bundle) -> impl Bundle
```

- Both buttons carry `CloseDialog`, and one observer in `ui.rs` despawns the `Dialog` root
  on either answer: the same shape as `CloseOverlay`.
- The Yes button also carries `on_yes`, a marker from the caller (`ConfirmDelete(path)`,
  `ConfirmOverwrite(path)`, `ConfirmQuit(QuitTo::Menu)`), and the caller's own observer does
  the work. `ui.rs` never learns what a save is.
- The caller adds the `DespawnOnExit` of wherever it was asked, so a dialog cannot outlive
  its list.
- It draws above overlays: `DIALOG_Z_INDEX` is `GlobalZIndex(200)`, using the room that
  `OVERLAY_Z_INDEX`'s comment left "for anything that must draw over an overlay".

Three features use it (saves, deleting, quitting), so it belongs in the shared `ui.rs`.

### Z order

| Layer | `GlobalZIndex` | Where |
| --- | --- | --- |
| Gameplay, menu | none | the screens |
| Menu closing veil | 1 | `VEIL_Z_INDEX`, unchanged |
| Pause menu | 50 | `PAUSE_Z_INDEX`, `screens/gameplay/pause.rs` |
| Overlays | 100 | `OVERLAY_Z_INDEX`, unchanged |
| Dialog | 200 | `DIALOG_Z_INDEX`, `ui.rs` |

A full-window node blocks picking to everything under it, so no button can be clicked
through a backdrop or a dialog. The menu veil already relies on this.

## Flow

```
Menu
 ├─ New Game ──────────────────────── veil ─→ Loading ─→ Intro ─→ Gameplay (play time 0)
 ├─ Continue (greyed with no save) ── veil ─→ Loading ──────────→ Gameplay (newest save)
 ├─ Load Game ─→ [LoadGame] ─ pick a save ─ veil ─→ Loading ────→ Gameplay
 │                          └─ delete ─→ [dialog]
 ├─ Settings, Credits (as today)
 └─ Quit

Gameplay ─ Esc ─→ Pause
 ├─ Resume ─────────→ Pause::Off
 ├─ Save ───────────→ [SaveGame]: "New save", or an existing save ─→ [dialog: overwrite?]
 ├─ Load Game ──────→ [LoadGame] ─ pick a save ─→ Loading ─→ Gameplay
 ├─ Settings ───────→ [Settings] (the same overlay as from the menu)
 ├─ Quit to Menu ───→ [dialog, if progress is unsaved] ─→ Menu
 └─ Quit to Desktop ─→ [dialog, if progress is unsaved] ─→ AppExit::Success
```

### Starting a game: `StartGame`

New Game, Continue and every pick in the Load Game list start a game, from two different
screens: the menu has a veil to fade out first, the pause menu does not. So whoever starts
a game triggers one event, and the screen that is up decides how to leave:

```rust
/// Asks the current screen to leave for the given game.
#[derive(Event)]
pub(crate) struct StartGame {
    pub(crate) session: Session,
    pub(crate) destination: LoadingDestination,
}
```

- It lives in `screens.rs`, next to `Screen`, because every screen may answer it.
- `menu.rs` observes it: it inserts the `Session` and the `LoadingDestination` resources and
  spawns the closing veil. The veil then goes to `Loading` exactly as today.
- `screens/gameplay.rs` observes it: it inserts both resources and sets `Screen::Loading`
  on a cut. There is no veil in gameplay.
- Both observers check `State<Screen>` first, so only one of them acts.

`saves.rs` triggers `StartGame` and never learns what a veil is.

### The loading screen's destination

As the previous spec planned, the only structural change in `loading.rs` is a resource set
by the caller:

```rust
/// Where the loading screen goes once its work is done.
#[derive(Resource, Clone, Copy)]
pub(crate) enum LoadingDestination {
    Intro,
    Gameplay,
}
```

- `Intro`: loads the intro's frames and music as today, then goes to `Screen::Intro`.
- `Gameplay`: has nothing to load yet and leaves on the next frame. This is where the
  gameplay's own assets will be loaded later.

`load_intro_assets` runs only for `Intro`. `leave_loading_screen` asks a pure function where
to go, so the choice is tested:

```rust
/// Returns the screen the loading screen leaves for.
fn screen_after_loading(destination: LoadingDestination) -> Screen
```

### The file is read on the click

Picking a save reads and parses it *before* the veil starts. A save that turns out to be
unreadable says so in the list, and the player never leaves the menu to find out on the
other side.

### The Esc rule, one owner, two steps longer

`go_back_on_esc` in `overlays.rs` stays the single place Esc is decided, as the previous spec
chose:

```
1. a dialog is open           → close it
2. an overlay is open         → close it (back to the pause menu, if paused)
3. gameplay is paused         → resume
4. gameplay is running        → pause          (replaces "back to the menu")
```

It reads `Option<Res<State<Pause>>>`, because `Pause` does not exist outside gameplay.

## Storage

### Where

The per-user data folder of each OS, plus the package name:

| OS | Folder |
| --- | --- |
| Windows | `%APPDATA%\bevy-starter\` |
| Linux | `$XDG_DATA_HOME/bevy-starter/`, or `~/.local/share/bevy-starter/` when it is unset |
| macOS | `~/Library/Application Support/bevy-starter/` |

- Found with `std::env::var`, in a pure function with a test per OS:
  ```rust
  /// Returns the folder this game keeps its files in, or `None` when the OS gives no home.
  fn data_dir(os: &str, var: impl Fn(&str) -> Option<String>) -> Option<PathBuf>
  ```
  `os` is `std::env::consts::OS` in the game and a literal in the tests.
- The folder name is `env!("CARGO_PKG_NAME")`, so renaming the game in `Cargo.toml` renames
  its data folder too, and `docs/template.md` does not gain a fourth place to keep in sync.
- Not next to the executable: an install under `Program Files` cannot be written to.
- `directories` would do this, and was turned down: about twenty lines of `std` against a
  third-party crate, under the rule the previous spec left ("every third-party crate is
  friction for everyone who clones it").
- No data folder at all (no `APPDATA` or `HOME`): saving reports "Could not save", and the
  lists show as empty. The game still runs.

### Layout

```
bevy-starter/
  settings.ron              fullscreen and volume
  saves/
    1758620400123.ron       one file per save, named by its creation time in milliseconds
    1758621234567.ron
    autosave.ron            the one autosave, always rewritten in place
```

The file name only has to be unique, so it is set once and never changes. The list is
ordered by `saved_at_secs` inside the file, which an overwrite updates.

### Format

RON through `serde`. Both are already in `Cargo.lock` because Bevy uses them (`serde
1.0.229`, `ron 0.12.2`); they become direct dependencies, so no new crate enters the graph
and `about.toml` has nothing new to review. RON is what Bevy's scene files use, and it is
readable and editable by hand while debugging.

```rust
/// What one save file holds. The gameplay adds its fields here.
#[derive(Serialize, Deserialize)]
pub(crate) struct SaveData {
    pub(crate) version: u32,
    pub(crate) saved_at_secs: u64,
    pub(crate) play_secs: f32,
}
```

```ron
(version: 1, saved_at_secs: 1758620400, play_secs: 754.2)
```

`SAVE_VERSION: u32 = 1` sits in `save.rs`'s Config. The field order is the order a reader of
the file wants: what it is, when, then the content.

### Reading: every file becomes an entry

```rust
/// One file in the saves folder, as far as the game could make sense of it.
pub(crate) enum SaveEntry {
    Ready(PathBuf, SaveData),
    Incompatible(PathBuf),
    Unreadable(PathBuf),
}

/// What a save file's text turned out to be, before it is paired with its path.
enum ParsedSave {
    Ready(SaveData),
    Incompatible,
    Unreadable,
}

/// Returns what a save file's text holds: a save, one from an unknown version, or neither.
fn parse_save(text: &str) -> ParsedSave
```

- An unknown `version` → `Incompatible`: listed, not loadable, deletable.
- Text that is not a save at all → `Unreadable`: listed as "Unreadable save", deletable.
- A file that cannot be opened → `Unreadable` too.

Nothing in the saves folder can break the list, and the player can always get rid of it.

### Writing: never half a file

```rust
/// Writes a file through a temporary one and a rename, so a crash mid-write leaves the
/// old file whole.
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()>
```

This is what makes overwriting safe: without it, a crash at the wrong moment destroys the
save being replaced. `std::fs::rename` replaces the target in one step on the same volume,
and the temporary file sits in the same folder, so it is always the same volume.

A failed write (full disk, no permission) shows "Could not save" in the Save list and logs
the error with `error!`. A save that fails silently is the worst case: the player believes
they saved.

`// ponytail: read and written on the main thread; move to IoTaskPool when saves grow past a few KB`
goes above `write_atomic`.

### Settings

```rust
/// The preferences remembered between runs.
#[derive(Serialize, Deserialize)]
struct SettingsFile {
    version: u32,
    fullscreen: bool,
    volume: f32,
}
```

- Read once at `Startup` and applied to the `Window` and to `GlobalVolume`. Missing,
  unreadable or from another version: the defaults, silently. It is a preference and
  does not deserve an error screen.
- Written on `OnExit(Overlay::Settings)`, from the `Window` and `GlobalVolume` themselves,
  which stay the only copy of the state, as in the previous spec. One write per visit to
  the panel, not one per slider frame.
- It lives in `overlays/settings.rs`, which already owns both controls, and uses
  `save.rs`'s reading and writing.

## The session: the game being played

```rust
/// The game in progress: how long it has been played, and how long at its last save.
#[derive(Resource, Clone, Default)]
pub(crate) struct Session {
    pub(crate) play_secs: f32,
    pub(crate) saved_play_secs: f32,
}
```

- New Game: `Session::default()`.
- A loaded save: both fields from the file's `play_secs`, so it starts with nothing unsaved.
- `screens/gameplay.rs` adds `Time::delta_secs()` to `play_secs` every frame in gameplay.
  Virtual time is paused with the game, so the pause menu does not count as playing.
- Saving, by hand or automatically, writes `play_secs` and then sets `saved_play_secs` to it.

```rust
/// Returns whether quitting now would lose play time that is in no save.
fn has_unsaved_progress(session: &Session) -> bool
```

**It is never removed.** Removing it on `OnExit(Screen::Gameplay)` looks tidy and is a bug:
loading from the pause menu inserts the new `Session` and changes the screen in the same
frame, and the transition's `OnExit` would then remove the session that was just loaded.
Each `StartGame` replaces it instead. A leftover session in the menu is never read.

## Autosave

Most games keep one autosave next to the player's own saves, and this template does too:

- **One file, `saves/autosave.ron`**, always rewritten in place through `write_atomic`. It
  never multiplies, so the unlimited list does not fill up with autosaves.
- **When:** after `AUTOSAVE_INTERVAL_SECS` (300, in `save.rs`'s Config) of *unsaved* play.
  The clock is the session itself, so no extra timer or field is needed:

  ```rust
  /// Returns whether enough play has gone unsaved that the game should save by itself.
  fn autosave_is_due(session: &Session) -> bool   // play_secs - saved_play_secs >= interval
  ```

  Play time only grows while unpaused, so the pause menu never triggers an autosave, and
  a manual save pushes the next autosave back by a full interval.
- **Not on quitting.** Autosaving on Quit to Menu would mean there is never unsaved
  progress to ask about, and the "Quit without saving?" dialog, which was asked for, would
  never show. Quitting stays the player's decision.
- **Counts as a save:** it sets `saved_play_secs`, so right after an autosave quitting asks
  nothing, which is true: nothing would be lost.
- **In the lists:** labelled "Autosave · 2h 05m played · 3 days ago". Load Game shows it
  and can delete it, and Continue considers it like any other save. Save Game leaves it
  out: writing a manual save over the autosave slot would be lost at the next autosave.
- **A failed autosave** logs with `error!` and shows nothing: the player did not ask for
  it and is in the middle of playing. The next interval tries again.

`autosave_game` runs in `screens/gameplay.rs`, gated on `in_state(Pause::Off)`.

## Lists: `overlays/saves.rs`

`LoadGame` and `SaveGame` are the same list with different actions, so they share one file:

| | `Overlay::LoadGame` | `Overlay::SaveGame` |
| --- | --- | --- |
| Top of the list | — | "New save" |
| The autosave | listed | left out |
| Click a `Ready` save | load it (`StartGame`) | "Overwrite this save?" |
| Click `Incompatible` / `Unreadable` | nothing; shown greyed | nothing; shown greyed |
| Delete button per row | yes, with "Delete this save?" | no |
| Close | back to the menu or the pause menu | back to the pause menu |

- The list is read from disk on `OnEnter` of either overlay, and rebuilt after a delete.
- The rows sit in a node with `Overflow::scroll_y()` and `ScrollArea`
  (`bevy_ui_widgets-0.19.1/src/scrollarea.rs:19`), which turns the mouse wheel into
  scrolling and comes with `DefaultPlugins` (`bevy_internal-0.19.1/src/default_plugins.rs:100`).
  No scrolling code of ours.
- An empty `LoadGame` list says "No saves yet".
- After a successful save, `SaveGame` closes and the pause menu's status line reads
  "Saved". The line is plain text that disappears with the pause menu, not a timed toast:
  virtual time is paused there, so a `Timer` on `Res<Time>` would never finish.

A row's label comes from a pure function:

```rust
/// Returns a save's label: its play time and how long ago it was saved.
fn save_label(save: &SaveData, now_secs: u64) -> String      // "2h 05m played · 3 days ago"
```

"3 days ago" and not a date: `std` has no time zone, and a date in UTC would be off by hours
for most players. A relative age needs only a subtraction, and every unit boundary gets a
test ("just now", minutes, hours, days).

## The menu

| Button | Action |
| --- | --- |
| New Game | `StartGame { Session::default(), Intro }` |
| Continue | `StartGame` with the newest `Ready` save, `Gameplay` |
| Load Game | `Overlay::LoadGame` |
| Settings | `Overlay::Settings`, as today |
| Credits | `Overlay::Credits`, as today |
| Quit | `AppExit::Success`, as today |

- Continue spawns with `InteractionDisabled` (`bevy_ui-0.19.1/src/interaction_states.rs:23`)
  when no save is `Ready`. The `Button` widget already ignores clicks and keys on a disabled
  entity.
- Deleting the last save from the Load Game list must grey Continue out without leaving
  the menu: the menu re-checks on `OnExit(Overlay::LoadGame)`.
- "Play" becomes "New Game", and `MenuAction::Play` becomes `MenuAction::NewGame`.
- The newest save is chosen by a tested pure function, `newest_ready(&[SaveEntry])`.

### The veil moves out first

`menu.rs` is at 316 lines and is about to gain two buttons, a disabled state and a
`StartGame` observer. The veil (`Veil`, `MenuTransition`, `veil()`, `veil_alpha`,
`screen_after_veil`, `animate_veils` and their tests) already reads as a thing of its own,
so it moves to `src/screens/menu/veil.rs`, in a commit that only moves code, before any of
this lands.

## The pause menu

`src/screens/gameplay/pause.rs`, registered by `gameplay.rs`'s plugin:

- `Pause` as a `SubStates` with `#[source(Screen = Screen::Gameplay)]`, `Off` by default.
- The panel spawns on `OnEnter(Pause::On)` with `DespawnOnExit(Pause::On)` and
  `PAUSE_Z_INDEX`, over a backdrop, so the frozen game shows dimmed behind it.
- Buttons, through a `PauseAction` component, as `MenuAction` does in the menu:

| Button | Action |
| --- | --- |
| Resume | `Pause::Off` |
| Save | `Overlay::SaveGame` |
| Load Game | `Overlay::LoadGame` |
| Settings | `Overlay::Settings` |
| Quit to Menu | `Screen::Menu`, or the dialog first when `has_unsaved_progress` |
| Quit to Desktop | `AppExit::Success`, or the dialog first when `has_unsaved_progress` |

The two quits share one confirmation marker, `ConfirmQuit(QuitTo)`, where `QuitTo` is
`Menu` or `Desktop`.

## Shared code

| File | Gains | Why here |
| --- | --- | --- |
| `src/save.rs` (new) | `SaveData`, `SaveEntry`, `Session`, the data folder, reading, `write_atomic`, listing, deleting, the pure functions above | saves, settings, the menu, the pause menu and gameplay all use it; `CONTRIBUTING.md` already names `save.rs` as the example of a shared file |
| `src/ui.rs` | `dialog()`, `Dialog`, `CloseDialog`, `DIALOG_Z_INDEX`; a disabled button colour | three features ask questions; every button can be disabled |
| `src/overlays.rs` | `overlay_root()` with the backdrop, used by all four overlays | all four overlays |

`save.rs` starts as one file. When it passes the 300–400 lines where `CONTRIBUTING.md` says
to split, the disk half (`data_dir`, `write_atomic`, listing) is the part that reads on its
own: `save.rs` next to `save/disk.rs`.

**The disabled button.** `paint_buttons` gains a fourth colour, `BUTTON_DISABLED`, chosen
first by `button_color`: a disabled button looks disabled whether or not it is hovered.
`play_button_sounds` skips entities with `InteractionDisabled`, because `Hovered` still
changes on them, and a greyed button that ticks reads as broken.

## Files

| File | Change |
| --- | --- |
| `Cargo.toml` | `serde = { version = "1", features = ["derive"] }`, `ron = "0.12"` |
| `src/main.rs` | `mod save;` |
| `src/save.rs` | new, see "Shared code" |
| `src/ui.rs` | dialog, disabled colour, no hover sound when disabled |
| `src/screens.rs` | `StartGame` |
| `src/screens/menu.rs` | the six buttons, Continue's disabled state, the `StartGame` observer |
| `src/screens/menu/veil.rs` | new, moved out of `menu.rs` |
| `src/screens/loading.rs` | `LoadingDestination`, `screen_after_loading`, intro assets only for `Intro` |
| `src/screens/gameplay.rs` | counts play time, autosaves, observes `StartGame`, registers `pause` |
| `src/screens/gameplay/pause.rs` | new, see "The pause menu" |
| `src/overlays.rs` | `LoadGame`, `SaveGame`, `overlay_root()`, the Esc rule |
| `src/overlays/saves.rs` | new, both lists |
| `src/overlays/credits.rs` | uses `overlay_root()` |
| `src/overlays/settings.rs` | uses `overlay_root()`, reads and writes `settings.ron` |
| unchanged | `splash.rs`, `intro.rs`, `fade.rs`, `assets.rs` |

## Tests

Logic in pure functions, as `CONTRIBUTING.md` asks, with names that say what they guarantee:

| Function | File | Guarantees |
| --- | --- | --- |
| `data_dir` | `save.rs` | the right folder on each OS; `~/.local/share` without `XDG_DATA_HOME`; `None` without a home |
| `parse_save` | `save.rs` | a valid save reads back; an unknown version is incompatible; garbage is unreadable |
| `newest_ready` | `save.rs` | the newest `Ready` save wins; incompatible and unreadable ones are never chosen; an empty list gives `None` |
| `save_label` | `save.rs` | play time as hours and minutes; "just now", minutes, hours and days, each away from its boundary |
| `has_unsaved_progress` | `save.rs` | a fresh game has none; playing adds some; saving clears it |
| `autosave_is_due` | `save.rs` | not before the interval of unsaved play; due after it; a save starts the interval again |
| `screen_after_loading` | `loading.rs` | `Intro` goes to the intro, `Gameplay` straight to gameplay |
| `button_color` | `ui.rs` | disabled wins over hovered and pressed |

Two functions touch the disk and are tested against a folder under `std::env::temp_dir()`,
named after the test, so tests never share one:

- `write_atomic` then reading back gives the same text, and no `.tmp` file is left behind.
- Listing a folder with one valid, one incompatible and one garbage file gives one entry of
  each kind.

No test crate is added for this.

`just check` says nothing about how any of it looks or feels. Each step below ends with
`just run` and the things to try in it.

## Implementation order

Every step compiles, passes `just check` with `-D warnings` and runs. Nothing is written
before the code that calls it, since a function nobody calls fails the build as dead code.
One commit per step.

1. **Move the veil** to `src/screens/menu/veil.rs`. Only a move.
   *Try:* the menu fades in and Play fades out, as before.
2. **Pause.** `Pause` sub-state, the panel with Resume, Settings, Quit to Menu and Quit to
   Desktop (no Save or Load yet), `Time<Virtual>` paused, `PAUSE_Z_INDEX`, the Esc rule's
   steps 2–4, and `overlay_root()` with the backdrop on every overlay.
   *Try:* Esc pauses and resumes; Settings over the pause menu closes back to it; Quit to
   Menu fades the menu in normally.
3. **Session and the dialog.** `save.rs` born with `Session` and `has_unsaved_progress`;
   gameplay counts play time; `dialog()` in `ui.rs` and Esc step 1; both quits ask when
   progress is unsaved.
   *Try:* quitting right after starting asks nothing; after a few seconds it asks; No keeps
   the pause menu, Yes quits; Esc closes the dialog.
4. **Settings remembered.** `data_dir`, `write_atomic`, `serde` and `ron` arrive with their
   first user, `settings.ron`.
   *Try:* go fullscreen and lower the volume, quit, start again: both are kept.
5. **Saving.** `SaveData`, `parse_save`, listing, `save_label`, `Overlay::SaveGame` with
   "New save" and overwrite behind a dialog, "Saved" on the pause menu, "Could not save".
   *Try:* save twice, overwrite one; open the saves folder and read the files.
6. **Autosave.** `AUTOSAVE_INTERVAL_SECS`, `autosave_is_due`, `autosave_game`, and the
   autosave left out of the Save Game list.
   *Try:* with the interval lowered to a few seconds for the run only (not committed),
   `autosave.ron` appears and is rewritten, never while paused, and quitting right after
   one asks nothing.
7. **Starting and loading games.** `StartGame`, `LoadingDestination`, New Game / Continue /
   Load Game in the menu, `Overlay::LoadGame`, Continue's disabled state and colour, and
   Load Game in the pause menu.
   *Try:* New Game plays the intro, Continue and Load Game do not; the play time carries
   on from the save; the autosave is listed and Continue picks it when it is the newest;
   with no saves, Continue is grey, silent and does nothing.
8. **Deleting.** The delete button and its dialog, the list rebuilt, Continue re-checked.
   *Try:* delete every save from the menu's Load Game: Continue greys out on closing.
9. **Docs.** `README.md` features, `docs/flow.md` (states, flow diagrams, Esc, the new
   files), `docs/template.md` (project structure, where saves live, and that the data
   folder follows the package name).

## What changed while building it

1. **Step 2 landed as several commits, not one.** The pause state, the frozen clock, the panel,
   its buttons and `overlay_root()` each went in on their own, so each could be run and checked
   before the next was written. Each commit still passed `just check`.
2. **`Pause` is reached as `crate::screens::gameplay::pause::Pause`.** `overlays.rs` needs it for
   the Esc rule, so `screens::gameplay` and `gameplay::pause` are `pub(crate) mod` instead of
   private.
3. **`overlay_root()` also carries the row gap.** Settings had a `row_gap` on its root, and a
   bundle cannot take a second `Node`, so the gap moved into `overlay_root()` as
   `OVERLAY_GAP_PX`. Credits gains the same spacing between its text and Close button.
4. **The backdrops are both half black.** `BACKDROP_COLOR` in `pause.rs` and
   `OVERLAY_BACKDROP_COLOR` in `overlays.rs` are separate constants, since they are tuned
   separately, and Settings over the pause menu stacks them to about 75% dark.
5. **Unsaved progress has a 30-second grace.** `has_unsaved_progress` as written above is true
   after a single frame, which contradicts step 3's *Try* ("quitting right after starting asks
   nothing"). It is `play_secs - saved_play_secs > UNSAVED_GRACE_SECS` instead, with the
   constant at 30 in `save.rs`, and a fourth test for play inside the grace.
6. **Gameplay inserts the `Session` until step 7.** `StartGame` does not exist yet, so
   `start_session` on `OnEnter(Screen::Gameplay)` inserts a fresh one; step 7 moves that into
   the `StartGame` observer.
7. **`Session` also exists from startup.** `activate_pause_button` is a global observer, so it
   runs on every button press in the app, the menu's included. With `Res<Session>` it panicked
   before the first game (a missing resource is `Severity::Panic` in Bevy 0.19), so
   `gameplay.rs` calls `init_resource::<Session>()`. This fits "a leftover session in the menu
   is never read".
8. **The dialog's Yes and No sit in a row**, spaced by `DIALOG_BUTTON_GAP_PX`, and the dialog
   has its own `DIALOG_BACKDROP_COLOR`.
9. **`game_data_dir()` wraps `data_dir`.** It passes the real `std::env::consts::OS` and
   `std::env::var`, so callers never build the environment function and `data_dir` stays
   private to `save.rs` and its tests.
10. **Reading the settings is a pure `parse_settings(text) -> Option<SettingsFile>`**, with
    three tests (reads back, other version ignored, garbage ignored), so the system around it
    only touches the disk and the ECS.
11. **`FULLSCREEN_MODE` became a constant** in `settings.rs`: `toggled_window_mode` and
    `load_settings` both need the same borderless mode, and it was about to be spelled twice.
    The file stores `fullscreen` as `window.mode != WindowMode::Windowed`, the same test the
    checkbox uses, so the two can never disagree.
12. **Saving the settings creates the data folder first.** On the first run it does not exist,
    and `write_atomic` only writes files. Each of the two steps logs its own failure.
13. **Players name their saves, and the name lives inside the file.** `SaveData` gains
    `name: String`, typed in a text field (`EditableText`, which Bevy 0.19 ships in
    `bevy_ui_widgets`) when making a new save; an empty name shows a default in the list.
    The file name stays a unique id the player never sees: a typed name in the file name
    would fail on characters Windows forbids (`\ / : * ? " < > |`) and on reserved names
    (`CON`, `NUL`), and two saves with the same name would silently become one file.
14. **Save files are named `save_<millis>.ron`**, not just the number, so they read apart
    from `autosave.ron` in the folder. There is still no date in it: `std` has no time zone,
    and the list already shows the age.
