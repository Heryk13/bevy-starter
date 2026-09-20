# Menu, Intro, Settings, Credits and Gameplay screens

Status: **approved** — no code written yet.
Date: 2026-09-21
Bevy: 0.19.1

## Goal

Today the splash screen transitions to `Screen::Menu`, which has no plugin: the game
shows a black window and the player can only alt-F4 out of it. This design closes the
flow with a real menu and the screens it reaches, so every path a player can take leads
somewhere and comes back.

This repository is a template, so its structure *is* the product. State topology is
decided once, up front, for growth that is planned rather than hypothetical. Feature
scope stays minimal — see "Out of scope".

## Decided

| Question | Decision |
| --- | --- |
| Buttons | Play, Settings, Credits, Quit |
| Play leads to | `Screen::Intro`, then `Screen::Gameplay` (background colour only) |
| Intro | 91 stills on a timer plus its soundtrack — Bevy plays no video |
| Settings controls | Fullscreen on/off, and a 0–100 volume slider |
| Widgets | `bevy_ui_widgets` for `Button`, `Slider` and `Checkbox` — experimental, accepted |
| Credits content | "Made with Bevy" and Heryk13 |
| Menu and UI assets | Placeholders, meant to be replaced |
| Settings persistence | None. Both reset to the default on every run |
| Navigation | Mouse only, plus Esc to go back |
| Menu layout | Buttons in a column on the left, over a background image |
| Menu title | Part of the background artwork, not a `Text` node |
| Menu entrance | A black veil fades out, on every entry into the menu |
| Button sounds | A click on press and a tick on hover, for every button in the app |
| `src/ui.rs` | Approved. Shared code, named after its domain — see below |

## Out of scope

Each of these is a separate design cycle, not a "while we are here":

- Saving preferences to disk (serialization, per-OS paths, corrupt-file handling)
- Saved games, and the menu buttons that need them. The menu here is Play, Settings,
  Credits and Quit; the eventual New Game / Continue / Load Game set waits for a save
  system, and `InteractionDisabled` is how Continue will grey out with no save present
- Keyboard and gamepad navigation (focus state, selected-item highlight)
- Any actual gameplay inside `Screen::Gameplay`
- The pause menu itself. Its *shape* is accounted for below; its code is not written
- Looping music and ambience. The sounds in this cycle are all one-shots: two UI clicks
  and the intro's soundtrack, which starts and ends with the intro

## Structure

Two state enums, each answering one question:

```rust
/// Where the player is.
enum Screen {
    #[default]
    Splash,
    Menu,
    Intro,
    Gameplay,
}

/// What is drawn on top of the current screen.
enum Overlay {
    #[default]
    None,
    Settings,
    Credits,
}
```

The first draft of this design made `Settings` and `Credits` plain `Screen` variants
and deferred the problem. It was rejected, because one known fact about this game — it
will have a pause menu — breaks that shape in two places:

1. **Pause cannot be a `Screen`.** Leaving `Screen::Gameplay` fires
   `DespawnOnExit(Screen::Gameplay)` and destroys the running game. Pause has to happen
   *without* leaving the gameplay screen.
2. **Settings opened from a pause menu has to return to the pause menu**, not to the
   main menu. A sub-state does not fix this: in Bevy's own `examples/showcase/game_menu.rs`,
   even with a `MenuState` sub-state, the back buttons are hardcoded destinations
   (`MenuButtonAction::BackToMainMenu`, line 720). Nesting pages does not remember where
   you came from.

An overlay dissolves both problems instead of solving them. Opening Settings never
leaves the screen underneath, so closing it has nowhere to go wrong — there is no
"back destination" to compute, remember, or get wrong.

**The cost today is the same as a `Screen` variant:** `OnEnter(Overlay::Settings)`
instead of `OnEnter(Screen::Settings)`, `DespawnOnExit(Overlay::Settings)` instead of
`DespawnOnExit(Screen::Settings)`. One extra enum, two fewer `Screen` variants.
`DespawnOnExit<S: States>` is generic over any state type, and several `States` in one
app is ordinary Bevy (`game_menu.rs` runs two).

**When the pause menu arrives**, it is a sub-state of gameplay and nothing written now
changes:

```rust
enum GameplayState { Running, Paused }   // sub-state of Screen::Gameplay
```

The pause UI spawns on `OnEnter(GameplayState::Paused)`, gameplay systems get
`.run_if(in_state(GameplayState::Running))`, and the pause menu's Settings button sets
`Overlay::Settings` — the same overlay, over the paused game, closing back to the pause
menu. The settings code is reused, not rewritten.

**Drawing on top.** A screen and an overlay are separate UI roots, so nothing in the
tree decides which one draws first. `GlobalZIndex` is what makes it explicit, and it is
what the engine's own FPS overlay uses
(`bevy_dev_tools-0.19.1/src/fps_overlay.rs:200`):

```rust
const OVERLAY_Z: i32 = 100;   // room left above it on purpose
```

The engine picks `i32::MAX - 32` for the FPS overlay and says why in a comment: *"so you
can render on top of it if you really need to"*. Same idea here, at a saner number.

Plain `ZIndex` does not work for this — it only orders siblings inside one tree. A second
camera would work, but that is a camera, a render target and an order to maintain for one
panel.

**The panel is translucent**, not opaque: black at around 85%, so the screen underneath
stays visible. Same cost as an opaque one, and it is what makes the idea legible to the
player — especially over a paused game, where seeing the frozen frame behind is the point.

**Clicks underneath.** The overlay panel is full-screen, and in `bevy_picking` an entity
without a `Pickable` component blocks the ones below it. That is the documented default,
not an accident (`bevy_picking-0.19.1/src/hover.rs:207`):

```rust
pointer_entity_set.insert(*entity, pick_data.clone()); // Emit events by default
break; // Entities block by default so we break out of the loop
```

So the overlay root carries no `Pickable` at all — the default is already what we want.
(The FPS overlay adds `Pickable::IGNORE` precisely because it wants the opposite.) The
menu systems are still gated with `.run_if(in_state(Overlay::None))` so correctness does
not rest on that subtlety.

## Flow

```
Splash --> Menu --> Play --> Intro --> Gameplay --> Esc --> Menu
             |                  |
             |                  +--> skipped, same keys as the splash
             |
             +--> Quit --> AppExit

Overlay, over whatever screen is below it:

  Menu (or, later, a paused game)
    +--> Settings --+
    |               +--> Close / Esc --> back to the screen underneath
    +--> Credits ---+
```

`Splash` already skips to `Menu` on any key or click; that stays as it is.

## Files

| File | Change |
| --- | --- |
| `src/screens.rs` | `Intro` and `Gameplay` variants, registers the screen plugins |
| `src/screens/menu.rs` | new — background, scrim, the 4 buttons, entrance fade |
| `src/screens/intro.rs` | new — the frame sequence, its audio and the skip |
| `src/screens/gameplay.rs` | new — background colour only |
| `src/overlays.rs` | new — the `Overlay` state, its plugins, and the Esc system |
| `src/overlays/settings.rs` | new — fullscreen checkbox, volume slider, Close |
| `src/overlays/credits.rs` | new — static text, Close |
| `src/ui.rs` | new — button widget, palette, colouring and the two UI sounds |
| `src/main.rs` | `mod ui;`, `mod overlays;`, and the `GAME_TITLE` constant |
| `Cargo.toml` | `features = ["jpeg", "wav"]` on the bevy dependency |
| `assets/menu/background.jpg` | moved from `assets/sample.jpg` — placeholder art |
| `assets/intro/frame_000.png` … `frame_090.png` | 91 frames, moved out of `assets/` |
| `assets/ui/click.wav` | moved from `assets/ui_click.wav` |
| `assets/ui/hover.wav` | moved from `assets/ui_hover.wav` |
| `README.md` | one new bullet under Features (the split itself is already done) |
| `docs/template.md` | exists — gains the new files and the placeholder note |
| `docs/flow.md` | new — screens, overlays, and how the player moves between them |

The two module trees mirror the two state enums: `screens/` for where the player is,
`overlays/` for what is on top. Both follow the existing `screens.rs` pattern — a file
next to a folder, never `mod.rs`.

## `src/ui.rs`

Six buttons across three files (four in the menu, one Close in settings, one in credits)
need the same bundle and the same hover/press colours. Writing that six times is the
real risk in this change, so it lives in one module:

- `Config`: the palette (normal, hovered, pressed) and button sizing constants
- `pub(crate) fn button(text: &str) -> impl Bundle` — a `Button` node with its `Text`
  child. `pub(crate)`, not the `pub(super)` the other plugins use: `menu.rs` and
  `settings.rs` are not children of `ui`
- One system painting `BackgroundColor` from `Hovered` and `Pressed`, for every button
  in the app, with no `run_if`: it is generic over any button, on any screen or overlay
- `pub fn button_color(hovered: bool, pressed: bool) -> Color` — the pure function the
  system calls, and what the tests cover
- A second system playing the click and hover sounds off the same two components

### Which `Button`

`bevy_ui_widgets::Button`, not the classic `bevy_ui::Button` with its `Interaction`
enum. Both are headless as far as colour goes — painting is ours either way — so the
comparison is only about what comes for free:

| | `bevy_ui::Button` | `bevy_ui_widgets::Button` |
| --- | --- | --- |
| State | `Interaction` enum | `Hovered` + `Pressed` components |
| Accessibility | none | `AccessibilityNode(Role::Button)` |
| Click | `Interaction::Pressed` | an `Activate` event |
| Press, drag away, release | counts as a click | cancelled, which is correct |
| Disabled state | hand-rolled | `InteractionDisabled` |

That fourth row is a bug we would otherwise have written: with `Interaction::Pressed`,
pressing Quit, dragging off the button and releasing still quits the game.
`button_on_pointer_cancel` and the `DragEnd` handler in `bevy_ui_widgets-0.19.1/src/button.rs`
already deal with it.

**The crate says it is experimental**, in the first lines of its `lib.rs`:

> This crate is currently experimental and under active development. The API is likely
> to change substantially: be prepared to migrate your code.

Accepted knowingly. The deciding argument is that this project already takes that risk
for the volume slider, which is not worth hand-rolling. Staying on `bevy_ui::Button`
would not avoid the 0.20 migration — it would only leave the app with *two* interaction
models at once, `Interaction` for buttons and `Hovered`/`Pressed` for the slider, and
`ui.rs` having to speak both.

If the crate's churn ever costs more than it saves, the way back is `bevy_ui::Button`
plus `Interaction`, and it touches `ui.rs` and nothing else — that is exactly what
keeping the widget in one shared module buys.

Keyboard activation ships with the widget but needs focus, and `TabNavigationPlugin` is
**not** in `DefaultPlugins` (Bevy's own example adds it by hand). So keyboard navigation
stays out of scope as planned, one line away from working.

### Button sounds

`assets/ui/click.wav` and `assets/ui/hover.wav` (moved from `assets/ui_click.wav` and
`assets/ui_hover.wav`). The folder mirrors the module that owns them — `src/ui.rs` and
`assets/ui/` — the same way `splash.rs` and `assets/splash/` already line up. The folder
names the domain, so the files do not repeat it.

The handles load once into a resource; the system spawns a one-shot player per press:

```rust
commands.spawn((AudioPlayer::new(sounds.click.clone()), PlaybackSettings::DESPAWN));
```

`PlaybackSettings::DESPAWN` (`bevy_audio-0.19.1/src/audio.rs:106`) removes the entity
when the sound ends, so nothing accumulates.

Colour and sound are two systems, not one. Each reads the same query and does one thing
the name says. Neither gets a pure function of its own beyond `button_color`: picking
which of two handles to play is a two-arm match with no logic in it, and a test would
only write the match down a second time.

One thing that falls out for free: during the entrance fade the veil blocks picking, so
`Hovered` never changes and no sound plays over the transition.

`ui.rs` is the first file to follow the shared-code rule in `CONTRIBUTING.md`: named
after its domain, at the top of `src/`, because three features need it and none of them
owns it. Designing this screen is what surfaced the rule, so the rule was written down
in the same cycle rather than left as a one-off exception here.

The colouring system is the part that settles it: it must exist exactly once, over every
button in the app, with no `run_if`. There is no "the menu's version" of it.

## Asset formats

Bevy's default features cover **png** and **ogg vorbis** only — `common_api` pulls in
`png` (`bevy-0.19.1/Cargo.toml:2734`) and `audio` pulls in `vorbis` (line 2645). A `.jpg`
or a `.wav` fails to load with a stock setup, which is a quiet runtime failure, not a
compile error.

So `Cargo.toml` asks for the two extra decoders:

```toml
bevy = { version = "0.19.1", features = ["jpeg", "wav"] }
```

| Asset | Format | Why |
| --- | --- | --- |
| Art with transparency | PNG | Lossless, and the only one of these with an alpha channel |
| Photographic background | JPEG | No alpha to keep, and roughly 10x smaller on disk |
| Frame sequence | PNG | Lossless across 91 stills; JPEG artefacts would crawl frame to frame |
| Short UI sound | WAV | Already small at this length, and no lossy pass over the original |
| Anything longer than a second | OGG | Where compression finally matters, and it needs no extra feature |

On the GPU a PNG and a JPEG both end up as uncompressed RGBA, so the format only decides
the size of what you ship, never the video memory. The placeholder background is 38 KB as
JPEG and 150 KB re-encoded to PNG, because its worn stamp texture is noise, and noise is
what PNG compresses worst.

Licences were checked, not assumed: `hound` (wav) is Apache-2.0 and `zune-jpeg` is
MIT/Apache-2.0/Zlib, all three already in `about.toml`, and `cargo about generate --fail`
passes unchanged.

## Menu look

The menu does not just appear. It fades in from black, over a background image, with the
buttons in a column on the left.

Four layers, all children of one root node — so the order they are spawned is the order
they draw, and no `ZIndex` is needed between them:

| Layer | What |
| --- | --- |
| 1 | the background image, full-screen |
| 2 | a scrim, dark on the left and clear on the right |
| 3 | the button column, on the left |
| 4 | the entrance veil: solid black, fading out, then gone |

**There is no title node.** The game's title is part of the background artwork, drawn
and styled there, so the menu spawns no `Text` for it. The placeholder has no title,
which means the menu looks bare until the real art lands — expected, not a bug.

One thing to watch when that art arrives: the scrim is darkest on the left, so a title
painted on that side gets muddied. Keep the title to the right or the centre, where the
gradient has already cleared.

**The background.** `assets/sample.jpg` moves to `assets/menu/background.jpg`, next to
the existing `assets/splash/`. The folder already says what the image is for, so the file
name does not repeat it.

`NodeImageMode` (`bevy_ui-0.19.1/src/widget/image.rs:158`) offers `Auto`, `Stretch`,
`Sliced` and `Tiled` — there is no "cover". The node uses `Stretch`, which means the
final artwork should be authored at the window's aspect ratio. The placeholder is
520x359 and will look stretched on a wide window, which is what a placeholder is for.
A comment names the ceiling: real cover needs a clipping parent plus a min-width and
min-height on the image, which is not worth writing before there is real art.

**The scrim.** White text over an arbitrary photo is unreadable, and that is not a
detail to trim. Bevy 0.19 draws the gradient itself
(`bevy_ui-0.19.1/src/gradients.rs:290`):

```rust
LinearGradient::to_right(vec![
    ColorStop::new(Color::BLACK.with_alpha(0.8), percent(0)),
    ColorStop::new(Color::NONE, percent(60)),
])
```

Dark where the text is, clear where the artwork is. One component, the same cost as a
flat scrim, and it is what makes the left-hand layout worth choosing.

**The entrance.** A black node covering the screen, alpha going from 1.0 to 0.0, then
despawned. It follows `splash.rs` exactly: a `Timer` on the component, an `EaseFunction`,
and a pure `veil_alpha(elapsed_secs) -> f32` sitting next to its tests.

Nothing is factored out of `splash.rs` to share with it. That fade has four phases and
this one has a single ramp; a common helper would be an abstraction over two things that
only look alike.

It spawns on every `OnEnter(Screen::Menu)`, the return from gameplay included. There is
no check for where the player came from, which is the same principle the overlay follows.

While the veil is alive it also blocks clicks — for free, by the `Pickable` default
above — so no button can be pressed mid-transition. It despawns when the timer finishes,
and the block goes with it.

## Intro

Bevy plays no video, so the intro is what a video would have been taken apart into: a
sequence of stills swapped on a timer, with the soundtrack played alongside. Done right
nobody can tell the difference — it *is* what a video file holds, minus the codec.

**91 frames**, `assets/intro/frame_000.png` to `frame_090.png`, 960x540, 16 MB on disk.

```
Menu --[Play]--> Intro --> Gameplay
                   |
                   +-- any skip key or click, same set splash.rs already defines
```

It is a `Screen`, not part of gameplay: it has a beginning, an end, and it can be
skipped — the same shape as the splash. `src/screens/intro.rs` ends up reading like
`splash.rs` with an index instead of an alpha.

**Memory.** 91 textures at 960x540 come to roughly **177 MB of video memory** —
uncompressed RGBA is what the GPU holds, whatever the file on disk was. That is fine
while the intro runs and pure waste afterwards, so the handles live on an entity carrying
`DespawnOnExit(Screen::Intro)`. Leaving the screen drops them and the memory goes back.
Nothing new to write: it is what `splash.rs` already does with its logo.

**Loading.** 91 files load asynchronously, and starting the timer before they arrive
would flash blank frames. The guard is one line before the clock starts:

```rust
handles.iter().all(|h| asset_server.is_loaded_with_dependencies(h))
```

This is exactly the case `bevy_asset_loader` exists for, and it was re-examined now that
the count went from 3 assets to 94. It still loses: the whole benefit here is that one
line, and a loading-state plugin costs more than it removes.

**Rate.** One constant, `INTRO_FPS`, and a pure function next to its tests:

```rust
/// Returns which frame is showing at the given time into the intro.
fn frame_index(elapsed_secs: f32) -> usize
```

The value cannot be settled from the frames alone — it is the audio that fixes it. 91
frames at 30 fps is 3.03 s, at 24 fps it is 3.79 s, and the right answer is whichever one
matches the soundtrack's length. Calibrate it against the real file and leave the
constant where it can be nudged.

### These frames are a placeholder too

Like the menu background, this countdown is here so the screen has something to show. It
is not the game's real intro, so neither of the two things below is a defect to fix —
they are notes for whoever exports the real one.

**There is no audio.** `assets/intro_audio.ogg` turned out to be a PNG — magic bytes
`89 50 4E 47`, 960x540, and the one frame absent from the sequence. It was renamed to
`frame_027.png`, which completes the run with no gaps. `INTRO_FPS` therefore has nothing
to be calibrated against yet; the soundtrack's length is what will settle it.

**The frames are out of order.** Sampling the sequence, the countdown numbers do not run
in any consistent direction:

| File | Number on screen |
| --- | --- |
| `frame_000.png` | 1 |
| `frame_026.png` | 3 |
| `frame_027.png` | 1 |
| `frame_045.png` | 4 |
| `frame_090.png` | 3 |

A film leader counts down monotonically, so file order is not time order here. The usual
cause is alphabetical sorting during export: `frame1, frame10, frame11, …, frame2, …`,
which padding applied afterwards then locks in.

Known and accepted while it is a placeholder — a scrambled countdown still proves the
screen works. The fix for the real intro is to export straight into zero-padded names
instead of renaming after the fact:

```
ffmpeg -i intro.mp4 assets/intro/frame_%03d.png
```

One pass, correct order, and the same command emits the soundtrack.

Worth knowing while the code is written: `intro.rs` should build its handle list by
index, `frame_{i:03}.png`, not by reading the directory. Directory order is the thing
that went wrong here, and a loop over `0..FRAME_COUNT` cannot go wrong the same way.

## The game's name

```rust
const GAME_TITLE: &str = "Bevy Starter";
```

In `main.rs`, with one user today: the window title. It is not there for reuse — it is
there because renaming the game should be one edit in an obvious place, and this template
exists to be renamed. The menu does not read it, since its title is painted into the art.

## Transitions, and the two cuts left

Every screen change in this design is a hard cut except one: entering the menu, which
fades up from black. Where that leaves things:

| Change | What the player sees |
| --- | --- |
| Gameplay → Menu | covered — the menu's own entrance fade |
| Menu → Intro | a cut into `frame_000` |
| Intro → Gameplay | a cut out of `frame_090` |

The intro took care of the case this section was originally written about. Play used to
drop straight into gameplay; now it opens a film countdown, which is a transition by
nature.

The two cuts that remain stay as they are. Fading *out* of a screen means holding the
state change until the animation finishes — a small state machine, and one that would
have to live in every screen that wants it. Worth building only if the cuts actually
bother you once you watch it run, and the cheap half-measure is available anyway: the
intro's first and last frames can be authored dark, and then there is nothing to fade.

## Button actions

Each button carries a marker component naming what it does, and one small system per
file reads it:

```rust
#[derive(Component)]
enum MenuAction {
    Play,
    Settings,
    Credits,
    Quit,
}
```

Closing an overlay is not one of these. Both overlays have a Close button doing the
exact same thing, so it is one marker with one system, in `overlays.rs`, next to the Esc
system:

```rust
#[derive(Component)]
struct CloseOverlay;
```

Compare with Bevy's own example (`game_menu.rs:313`), whose action enum carries both
`BackToMainMenu` and `BackToSettings` — two hardcoded destinations. `CloseOverlay` has no
destination to get wrong, which is the same reason the overlay exists at all.

With `bevy_ui_widgets::Button` the click arrives as an `Activate` event carrying the
entity that fired it (`bevy_ui_widgets-0.19.1/src/lib.rs:82`), so each file gets **one**
observer that reads the marker off that entity:

```rust
fn activate_menu_button(activate: On<Activate>, actions: Query<&MenuAction>, ...) {
    match actions.get(activate.entity) { ... }
}
```

It sets `NextState<Screen>` for Play, `NextState<Overlay>` for Settings and Credits, or
writes `AppExit` for Quit.

This is an observer rather than a polling system, but it keeps what mattered about the
plain-system version: the behaviour is in one named function per file, readable top to
bottom. Bevy's own example attaches a closure per button at spawn time
(`standard_widgets.rs:157`); reading the marker from `activate.entity` instead keeps the
spawn code declarative and the behaviour in one place.

## Settings behaviour

**Fullscreen.** A `Checkbox`, not a button. Same crate as the other two widgets
(`bevy_ui_widgets-0.19.1/src/checkbox.rs:35`):

```rust
#[require(AccessibilityNode(accesskit::Node::new(Role::CheckBox)), Checkable)]
pub struct Checkbox;
```

Its own doc says to override the role with `Switch` for a toggle switch, which is what
this is.

The practical difference from a button: a button would need a system keeping its label
in sync — "Fullscreen: On" and "Fullscreen: Off" — and a place to hold the state. With a
checkbox the `Checked` component *is* the state, and there is no label to get out of step
with it.

The window is the state that matters though, so nothing is stored twice: the observer
takes `Single<&mut Window>` and flips `window.mode`. Borderless rather than exclusive
fullscreen — no resolution switching and instant alt-tab:

```rust
WindowMode::Windowed  <->  WindowMode::BorderlessFullscreen(MonitorSelection::Current)
```

**Volume.** A 0 to 100 slider, dragged with the mouse, the way most games do it.

Bevy already ships the widget. `bevy_ui_widgets` comes in with the `ui` feature, and
`UiWidgetsPlugins` — `SliderPlugin` included — is part of `DefaultPlugins`
(`bevy_internal-0.19.1/src/default_plugins.rs:100`), so nothing is registered by hand:

```rust
Slider { track_click: TrackClick::Snap, ..default() },
SliderValue(50.0),
SliderRange::new(0.0, 100.0),
```

That gives dragging, clamping to the range, click-to-jump, keyboard control, and
`AccessibilityNode(accesskit::Node::new(Role::Slider))` — a screen reader announces it
as a slider without us doing anything. Writing this by hand would mean redoing pointer
capture and drag maths, and the accessibility part would simply not exist.

The widget is headless: it holds the behaviour and draws nothing. We spawn a track and a
thumb, and one system moves the thumb from the value. The maths is already there too —
`SliderRange::thumb_position(value)` (`bevy_ui_widgets-0.19.1/src/slider.rs:192`) returns
the percentage to write into the thumb's `left`.

Dragging fires `ValueChange<f32>`; an observer writes it to `GlobalVolume`. No new
resource — Bevy already owns this state.

**The slider is not wired straight to the amplitude.** Volume is perceived roughly
logarithmically, and the engine's own doc comment shows why a linear mapping feels
broken (`bevy_audio-0.19.1/src/volume.rs:54`):

```rust
let volume = Volume::Linear(0.5);
assert!(ops::abs(volume.to_decibels() - -6.0206) < EPSILON);
```

Half the slider is only -6 dB — barely quieter to the ear, so the whole audible range
would bunch up near zero. Squaring fixes it for one line, which is what games have always
done:

```rust
/// Returns the linear volume for a slider position in 0..=100.
fn volume_from_slider(percent: f32) -> f32 {
    (percent / 100.0).powi(2)
}
```

Both APIs were verified against the local 0.19.1 source rather than from memory:
`bevy_window-0.19.1/src/window.rs` and `bevy_audio-0.19.1/src/volume.rs`. Note that
`AppExit` is a *message* in 0.19: the writer is `MessageWriter<AppExit>`, not
`EventWriter`.

No font asset is needed. `default_font` arrives through the `ui` feature chain
(`ui -> default_platform -> default_font`), so `Text` renders with Bevy's built-in font.

## Credits

Static text, a Close button, nothing else:

```
Made with Bevy
Heryk13
```

Short on purpose. The credits screen of a template is a slot, not a document — whoever
clones this replaces the name and adds their own.

It does not list dependency licences. `just release` already generates
`dist/licenses.html` from every crate in the tree, which is both more complete and always
current; a hand-written copy in the game would start rotting on the next `cargo update`.

## Assets are placeholders

`assets/menu/background.jpg`, `assets/ui/click.wav`, `assets/ui/hover.wav` and the 91
frames in `assets/intro/` are here so the screens have something to show and something to
play. They are meant to be replaced —
a game's own art and sounds are most of its personality, and none of it should come from
a starter template.

The one asset that stays is `assets/splash/bevy_logo_dark.png`, for the reason already
written in `docs/template.md`. That section gains a line saying which files are
placeholders, so nobody has to guess which ones are load-bearing.

## Esc

Esc means "close what is on top of me", which is now one rule with one owner:

- An overlay is open: one system in `overlays.rs` sets `Overlay::None`. It does not
  care which overlay, or what screen is underneath
- No overlay, and the player is in `Screen::Gameplay`: return to `Screen::Menu`

The second line is the one thing here that changes when the pause menu lands — Esc in
gameplay will open pause instead of leaving. That is a behaviour change, not a
structural one: no state, file or spawn code moves because of it.

Esc does nothing in `Menu`, where quitting is the Quit button's job.

**In `Splash` and `Intro` Esc is not "back" at all** — it is one of the skip keys, and
skipping moves forward. `SKIP_KEYS` in `splash.rs` already includes `KeyCode::Escape`,
so pressing it during the splash goes to the menu and during the intro goes to gameplay.
The same key reading as "back" in one screen and "forward" in another is worth saying out
loud, but it is the contract the splash set first, and one list of skip keys is simpler
than two rules inside the intro.

## Libraries considered and turned down

Written so nobody repeats the search in six months. Everything below was checked against
the local 0.19.1 sources, and one of them was actually compiled.

**What Bevy already gives us**, and what it saves writing:

| Piece | Saves |
| --- | --- |
| `bevy_ui_widgets::Button` | drag-cancel, focus, `Role::Button` |
| `bevy_ui_widgets::Slider` | dragging, clamping, `thumb_position`, `Role::Slider` |
| `bevy_ui_widgets::Checkbox` | the `Checked` state, `Role::CheckBox` |
| `EaseFunction` | the animation curves |
| `GlobalVolume`, `Window` | the state itself, so no resource of ours holds a copy |
| `PlaybackSettings::DESPAWN` | cleaning up finished sounds |
| `bevy_ui` flexbox | all of the layout |
| `DespawnOnExit` | tearing a screen down |

**`bevy_feathers`** — exists, is styled, and has `button.rs`, `slider.rs`,
`toggle_switch.rs` and a theme. Turned down because its own `lib.rs:8` says to:

> While it may be tempting to use this crate for your game's UI, it's deliberately not
> intended for that. [...] Consider copying this code into your own project.

It is built for editors and inspectors. Worth reading as a reference — its
`theme.rs`/`tokens.rs` is a tidy way to organise a palette.

**`bevy_tweening`** — not judged by its version number. It was added, resolved and
compiled: **0.16.0 builds against Bevy 0.19.1** without pulling a second `bevy` into the
graph, then it was reverted. Turned down anyway, because all it would replace is the
entrance veil: a `Timer`, an alpha and an `EaseFunction`, about twenty lines, with
`splash.rs` already holding the same pattern next door. The hard part of a tweening
library is the curves, and those are already in the engine.

**`bevy_asset_loader`** — loading states and declarative handles. For three assets
loaded in one `Startup` system, it is more configuration than it removes.

**`bevy_animation`**, which is native, can drive an arbitrary field through
`AnimatableProperty`. It still needs an `AnimationClip`, an `AnimationGraph`, an
`AnimationPlayer` and a trait impl per property. For one fade that is *more* code than
doing it by hand, not less.

### And the splash screen?

`splash.rs` is the other place with hand-written animation, and it is the strongest case
on this page for a library: four phases — wait, fade in, hold, fade out — is exactly the
shape a tweening `Sequence` expresses well.

It stays as it is regardless. The code is written, covered by six tests and working;
swapping it for a dependency is negative work. And its logic currently lives in
`logo_alpha`, a pure function testable without an `App` — handing the curve to a library
would move that behaviour somewhere a unit test cannot reach.

### The rule this leaves behind

In a template, every third-party crate is friction for everyone who clones it. The Bevy
ecosystem breaks on each release, and a starter carrying three outside crates sits and
waits for all three to migrate before it can move to 0.20. `bevy_ui_widgets` is the one
exception accepted here, and only because it ships from Bevy's own repository and moves
with the engine.

## Tests

The rule stands: logic in pure functions, systems stay thin. What gets covered:

| Function | Cases |
| --- | --- |
| `ui::button_color` | idle, hovered, pressed |
| `screens::menu::veil_alpha` | opaque at the start, half way at the midpoint, clear at the end |
| `screens::intro::frame_index` | first frame at 0, last frame at the end, clamped past it |
| `overlays::settings::toggled_window_mode` | windowed to borderless, borderless to windowed |
| `overlays::settings::volume_from_slider` | silent at 0, unchanged at 100, a quarter at 50 |

Floats compared with a tolerance, as in `splash.rs`.

Five pure functions is all the logic there is. Everything else is spawning nodes and
setting a state, which a unit test would only restate.

## Documentation split

The `README.md` is the front door: what this is, what you need installed, and the
commands to run it. Everything that describes how the game is *built* goes under
`docs/`, so the README stops growing every time a screen is added.

| Where | What | Why |
| --- | --- | --- |
| `README.md` | Features, Requirements, Usage, Cargo features, Releasing | Read on day one and used every day |
| `docs/template.md` | Starting a new game, Project structure, Assets | Read once, when the template is turned into a game |
| `docs/flow.md` | Screen flow, what each screen does, how to add one | Grows with every feature |
| `docs/specs/<name>/` | One folder per implementation: `design.md`, then `plan.md` | History of why, not how-to |

Moving Project structure out of the README is the point of the split: it is the one
section that gains a line for every file the game adds.

Step 6 of the loop in `CONTRIBUTING.md` currently says to update the `README.md` when
the project structure changes. After the split that sentence points at the wrong file,
so it moves in the same commit: features and commands stay with the README, project
structure points at `docs/template.md`.

This split is independent of the menu itself and can land as its own commit before
anything else, which is why it is step 0 below. **Done** in `d1eaa50`.

## Implementation order

Each step ends green (`just check`) and is its own commit:

0. `README.md` and `docs/template.md` — the documentation split above. **Done**
1. `src/ui.rs` — button widget, colour system with its tests, and the click and hover
   sounds
2. `src/screens/gameplay.rs` plus the `Gameplay` variant — the simplest screen, proves
   the pattern end to end
3. `src/screens/menu.rs` — background, scrim, the four buttons with Play and Quit
   working, and the entrance fade with its test
4. `src/overlays.rs` plus `src/overlays/credits.rs` — the `Overlay` state, the Esc
   system and the simplest overlay, together because the state needs one user
5. `src/overlays/settings.rs` — the fullscreen checkbox and the volume slider, with tests
6. `src/screens/intro.rs` plus the `Intro` variant — the frame sequence, the skip and
   `frame_index` with its tests. Last of the code because it is the only step waiting on
   an asset that does not exist yet
7. `docs/flow.md` — the finished flow, plus one new bullet under Features in the
   `README.md`, and in `docs/template.md` the new files in the project structure and the
   line marking which assets are placeholders
