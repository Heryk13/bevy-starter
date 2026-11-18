# Photo mode: freeze the game and take a photo of it alone

Status: **built** on `main` on 2026-11-18, both open questions answered as recommended (UTC by
a pure function; E kept).
Date: 2026-11-18
Bevy: 0.20.0
Roadmap: row 3 of `docs/roadmap/roadmap.md`, from its "Photo mode, as agreed" section. The
look is the mockup's block "3" in `docs/roadmap/ui/mockup.html`.

## Goal

A game being made needs pictures of itself: for its page, a trailer's still, a bug report.
A screenshot of the window has the dev tools in it, the FPS, whatever moves while the key is
pressed. Photo mode freezes the game without the pause menu, hides every dev window, and
takes a photo of the game alone on E, into the folder the file explorer (row 4) will browse.

`main` has no camera to move, so here photo mode freezes the game and takes it as it is. The
free cameras come with the branches that have something to look at: the 3D one with row 5
(`3d/player`), the 2D one with row 6 (`2d/player`). This design leaves them a state to hang on.

## Decided

| Question | Decision |
| --- | --- |
| A dev tool | Yes: F3 is on the roadmap's table of dev keys, so the whole of it lives in `src/dev_tools/photo_mode.rs`, in dev builds only, and its words are English |
| Keys | Photo Mode (F3) and Take Photo (E), actions on Settings' Dev page, so the keys panel lists them. Esc leaves too, and cannot change, as everywhere |
| Where it works | Gameplay, while it runs: F3 does nothing over the pause menu, an overlay or a dialog, nor on any other screen |
| Freezing | A third value of `Pause`, `Photo`, behind `#[cfg(feature = "dev")]` as `Page::Dev` is (below) |
| Esc | Leaves photo mode, instead of opening the pause menu: one more arm in `esc_action` |
| What hides | Every dev window, the F1 hint and the FPS, while photo mode is on; they come back as they were when it ends. The other dev keys do nothing meanwhile |
| The hint | A glass bar at the bottom centre: a camera, the E keycap, "Take Photo", a divider, a door, the F3 keycap, "Exit" (the mockup's bar) |
| A photo | E captures the window with the hint and the notice hidden for that frame, back on `ScreenshotCaptured`. PNG, the window's full size |
| Where photos go | `gallery/` in the game's data folder, the root row 4 calls Gallery, written with `write_atomic` |
| A photo's name | Its date and time in UTC, `2026-11-18_14-32-05.png`, as in the mockup (below) |
| "Photo saved" | A glass notification at the top right: a camera on a white tile, "Photo saved" and the file's name, for 2 s of real time, then it fades. Hidden in a photo taken while it shows |
| The free cameras | Not here: rows 5 and 6 run theirs `in_state(Pause::Photo)` and put the camera back on `OnExit(Pause::Photo)` |
| Multiplayer | Not here: row 18 decides how photo mode works while the world keeps running |

## Out of scope

- **The free cameras**, 3D and 2D: rows 5 and 6.
- **Browsing the photos**: the file explorer, row 4.
- **A resolution or format to choose.** One PNG at the window's size; a photo for a trailer
  can be scaled down, never up.
- **Hiding the game's own UI.** `main`'s gameplay has none, and `3d/player`'s first-person view
  draws no HUD. A game that adds one hides it the same way the dev windows hide.

## Structure

### Files

| File | Holds |
| --- | --- |
| `src/dev_tools/photo_mode.rs` | the two actions, entering and leaving, the hint bar, taking a photo, the notice, the photo's name |
| `src/dev_tools.rs` | the `DevUi` marker, which `spawn_dev_window` puts on every dev window's root |
| `src/screens/gameplay/pause.rs` | `Pause::Photo`; time freezes on leaving `Pause::Off`, not on entering `Pause::On` |
| `src/overlays.rs` | Esc in `Pause::Photo` goes back to `Pause::Off` |
| `src/dev_tools/keys_panel.rs`, `fps.rs`, `slow_motion.rs` | their keys do nothing in photo mode; the hint carries `DevUi` |

### `Pause::Photo`

`CONTRIBUTING.md` says `Pause` is whether the game is frozen. Photo mode freezes it, without
the menu, so it is a value of `Pause` rather than a state of its own:

```rust
pub(crate) enum Pause {
    #[default]
    Off,
    On,
    #[cfg(feature = "dev")]
    Photo,
}
```

The `cfg` keeps the variant out of release builds, where nothing would ever build it and
`just check` would refuse it, the way `Page::Dev` is kept out.

Everything already keyed to `Pause::Off` does the right thing for free: the player stops
walking and looking (`3d/player`), play time stops counting, autosaves wait. To freeze time
for both, `pause.rs` pauses `Time<Virtual>` on *leaving* `Pause::Off` and unpauses it on
coming back, where it does so on entering and leaving `Pause::On` today; it moves
`freeze_background` and `unfreeze_background` the same way. Nothing ever goes from `On` to
`Photo`, so the two never overlap. The pause menu still spawns on `OnEnter(Pause::On)`.

Considered and turned down: a `PhotoMode` state of its own. It would freeze time a second
way, and Esc's rule, which already reads `Pause`, would need to read a state that release
builds do not have.

### Entering and leaving

A pure function decides what F3 does from the pause it is pressed in: `Off` goes to `Photo`,
`Photo` to `Off`, `On` stays. Outside gameplay there is no `Pause` and nothing happens. Its
tests cover the three.

`esc_action` in `overlays.rs` gets one arm, `Some(Pause::Photo) => SetPause(Pause::Off)`,
behind the same `cfg`, and a test behind it too.

On `3d/player`, until row 5 brings the free camera, leaving `Pause::Off` unlocks the cursor
and stops the look (`first_person.rs`), as the pause menu does: photo mode there freezes the
view where it is, with the cursor free. Row 5 keeps it locked for the free camera.

### Hiding the dev UI

`DevUi` marks every root of the dev tools' UI: `spawn_dev_window` puts it on each window's root,
and the F1 hint carries it. On `OnEnter(Pause::Photo)` photo mode sets each one's `Visibility`
to `Hidden` and turns the FPS off; on `OnExit` it puts both back as they were, the FPS
included, remembered in a resource for the length of photo mode (an FPS turned off before
stays off).

The toggles of the keys panel, the FPS and slow motion get
`.run_if(not(in_state(Pause::Photo)))`, so nothing hidden can be opened behind the photo.
A tool added later does the same; the code says so beside `DevUi`.

Photo mode's own hint and notice are dev windows too, but they do not carry `DevUi`: they are
what shows in photo mode. They carry `PhotoModeUi`, which hides them for the frame a photo is
taken.

### The hint bar

A dev window, at the bottom centre: `spawn_dev_window` with a `dev_window` placed by `bottom`,
in a row node, as slow motion's capsule is. Each half is an icon, a keycap and its words
(`Icon::Camera`, E, "Take Photo"; `Icon::LogOut`, F3, "Exit"), with a thin divider between
them. Its keycaps follow a rebinding like every keycap.

A root lays its child out in the centre, so a window given only `bottom` should sit centred
along the bottom edge. That is the one layout here not tried yet: if Bevy places it at the
left, it is given `left: 50%` and a translation of half its width back, as the mockup does.

### Taking a photo

On Take Photo:

1. Every `PhotoModeUi` root is set `Hidden`.
2. A `Screenshot::primary_window()` is spawned in the same frame, so the frame it captures is
   one where they are already hidden.
3. Its `ScreenshotCaptured` observer sets them back to `Inherited`, encodes the image as PNG
   and writes it on the `AsyncComputeTaskPool`, as `picture.rs` writes an autosave's
   picture, and spawns the notice once the file is written. A failed write is logged, and the
   notice is not shown.

The PNG goes through `Image::try_into_dynamic` and the `image` crate's `write_to`, which
`picture.rs` already uses for its JPEGs. Bevy's `png` feature, which `image` needs for it, is
on in the feature sets `main` builds with (`bevy-0.20.0/Cargo.toml`); the tasks check it on
`2d/base` too.

Two photos in a row are two files: the name has the second in it, and a photo taken in the
same second as the one before gets `_2`, `_3` after its time, never an overwrite.

### The name, and the clock

A photo is named by its date and time, `YYYY-MM-DD_HH-MM-SS.png`, so the explorer lists them
in the order they were taken by sorting names, as `CONTRIBUTING.md` asks of anything ordered.
The standard library reads the clock (`SystemTime`, as `save.rs` does) but cannot say what
date that is.

**UTC, by a pure function**, chosen by the user over local time. Turning seconds since 1970
into a date is a short, well-known calculation, tested against known dates, with no crate.
The cost, accepted: in Brazil a photo taken at 14:32 is named 17-32. Turned down: local time
with the `chrono` crate, one more crate for every game made from the template, for one file
name.

### E, and the doors' E

Row 12 opens doors with E, a game action on `3d/player`. Row 1's bindings give every key to
one action at most: two defaults on one key log an error at start (`report_shared_keys`), and
rebinding one would swap the other. The two are never heard together (a door only while the
game runs, a photo only while it is frozen), but the bindings do not know that.

**E stays**, chosen by the user: nothing on any branch uses it yet, and row 12's spec settles
it, either with another key for doors or by letting actions that are never heard together
share one. Turned down: another photo key now; Space was already turned down, since the 3D
free camera rises with it.

### "Photo saved"

A dev window at the top right, where slow motion's capsule sits outside photo mode: an
`Icon::Camera` in dark ink on a small white tile, then "Photo saved" over the file's name in
muted text. It holds for `NOTICE_HOLD_SECS` (2 s) of `Time<Real>`, since the game is frozen,
then fades its glass out like an entrance in reverse, and despawns. A new photo replaces it.
It carries `PhotoModeUi`, so a photo taken while it shows does not have it in. It despawns on
leaving photo mode.

## Text

English literals, like every dev tool: "Take Photo", "Exit", "Photo saved", and the actions'
labels "Photo Mode" and "Take Photo".

## Tests

- In `photo_mode.rs`: what F3 does from each `Pause`; the name of a photo for known dates
  (1970-01-01, a leap day, the turn of a year), and the `_2` for a second photo in the same
  second.
- In `overlays.rs`: Esc in photo mode goes back to `Pause::Off`.
- The rest (hiding, the hint, the notice) reads the ECS and spawns nodes; it is tried in the
  game.

## Implementation order

Every step compiles, passes `just check` and runs. One commit per step, written and tried on
`3d/player`, committed on `main` with `just tools/sync main`.

1. **Freeze.** `Pause::Photo`, time frozen on leaving `Pause::Off`, the Photo Mode action on
   F3 and its pure function, Esc's arm, `DevUi` and the hiding, the other dev keys silenced.
   *Try:* F3 freezes the game and hides the dev windows and the FPS; F3 and Esc both bring
   them back as they were; Esc out of photo mode never opens the pause menu; the pause menu
   works as before; F1, F2 and F4 do nothing in photo mode.
2. **The hint bar.** *Try:* it shows at the bottom centre in photo mode only, with the keys
   it is bound to.
3. **Take a photo.** The Take Photo action on E, the name and its tests, the capture with the
   hint hidden, the PNG in `gallery/`. *Try:* E writes a dated PNG of the game alone, no hint
   in it; two quick presses write two files.
4. **"Photo saved".** *Try:* it shows after each photo with the file's name, fades after 2 s,
   and a photo taken while it shows does not have it in.
5. **Docs.** `README.md` (the dev keys), `docs/flow.md` (`Pause::Photo` beside the pause menu,
   the Esc rule, the gallery folder), `docs/template.md` (the file), the roadmap's row and
   this spec's status.

## What changed while building it

- **Time still freezes on entering each frozen value, not on leaving `Pause::Off`** (found
  while building 1.1). Leaving gameplay from the pause menu never enters `Pause::Off`, so an
  unpause there would leave the menu's clock frozen; and leaving gameplay while it runs does
  leave `Pause::Off`, so a pause there would freeze it too. `pause.rs` keeps its systems on
  `OnEnter(Pause::On)` and `OnExit(Pause::On)`, and runs the same ones on `Pause::Photo`,
  behind the `cfg`.
- **The FPS hides itself** (decided while building 1.5): `fps.rs` owns `FpsOverlayConfig`, so
  it turns the overlay off on `OnEnter(Pause::Photo)` and back on `OnExit`, keeping what it
  was in `FpsShownBeforePhoto`. Photo mode hides only the `DevUi` roots.
- **Slow motion's Numpad keys are silenced too** (found while building 1.6): its capsule
  hides, so stepping it would change the speed with nothing on screen to say so.
- **Take Photo is registered in step 2** (settled while building 2.2): the bar's E keycap names
  it, so the action comes with the bar, and step 3 brings what it does. For one commit the Dev
  page lists a key that does nothing yet.
- **Photo mode's windows carry `DevUi` too** (decided while building 2.1): `spawn_dev_window`
  puts it on every root, so `hide_dev_ui` skips the ones that also carry `PhotoModeUi`,
  whatever order the two systems of `OnEnter(Pause::Photo)` run in.
- **A photo's `_2` comes from a count kept in memory, not from the disk** (decided while
  building 3.3). Two quick presses ask for their names before either file is written, so a
  look at the disk would give both the same one. `PhotosThisSecond` keeps the last second and
  how many photos it had; a pure `photo_file_name(stamp, count)` spells the name, and its test
  replaces the disk test. Left out: a photo taken in the very second of another from an
  earlier run, which would need the game restarted within that second.
- **Bevy's `png` is on for `2d/base` too** (checked in 3.5): it is in `common_api`, which both
  the `2d` and the `3d` feature sets bring.
- **A flash and a shutter's click** (asked by the user for step 4): as the capture arrives, the
  edges of the window light up and fade (`FLASH`: four strips, 10 px, faint white at the
  window's edge and clear inward, in `Time<Real>`), and `assets/dev_tools/shutter.ogg`
  plays. A first radial glow toward the corners was turned down by the user: too much, and
  round; then a hard 4 px border, widened and softened at the user's word. The click is the
  user's own pick from Pixabay, "camera click" by stevemotion, converted once from MP3 to mono
  OGG, since Bevy's `audio` feature reads Vorbis and not MP3; the user saw no need to list
  its license. A click generated by Claude was turned down by ear.
- **Taking the photo moved to `photo_mode/photo.rs`** (decided while building 4.2): with the
  notice and the flash, `photo_mode.rs` would have passed 400 lines, and the photo (the file,
  its name, the flash, the click, the notice) reads as one thing on its own. `photo_mode.rs`
  keeps the mode: in and out, hiding, the bar.
- **The notice is written once the file is**: the task hands its file name back as a
  `PhotoWrite(Task<..>)` that `notice_written_photos` polls with Bevy's `check_ready`. Its
  systems run outside photo mode too, so a photo still being written as photo mode ends still
  says so, and its notice still goes.
- **The notice goes without fading** (decided while building 4.3): its glass is a
  `GlassMaterial`, whose alpha only `ui/` can reach, and its texts would fade apart from it.
  It comes in with the entrance every floating root has, and leaves after 2 s.
- **`sized_icon` became `custom_icon(icon, size_px, color)`**: the notice's camera is dark ink
  on a white tile, and a bundle cannot hold a second `TextColor`, so the color comes in where
  the icon is made, as the size does.
- **Steps 1 to 5 and the docs of the design landed as one commit**, the user's call, to sync
  less.
