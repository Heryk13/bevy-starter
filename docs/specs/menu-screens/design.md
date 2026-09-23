# The menu and the screens it reaches

Status: **approved** — built, in `9ad7b1f`..`4dd1ddc`. What the build changed is at the
bottom, under "What changed while building it"; everything above it is the design as approved.
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
| Play leads to | `Screen::Loading`, then `Intro`, then `Gameplay` |
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
| Menu exit | Play fades the menu to black first; the intro then starts on a cut |
| Loading screen | Real work today — it waits for the 91 intro frames, with a progress bar |
| Button sounds | A click on press and a tick on hover, for every button in the app |
| Menu music | `assets/menu/music.ogg`, looping for as long as the menu is on screen |
| Menu music volume | It follows the veil: up on the way in, down on the way out |
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
- Ambience, and music anywhere but the menu. The menu's looping track is in this cycle;
  gameplay music and a soundtrack that survives a screen change are not. Every other
  sound here is a one-shot: two UI clicks and the intro's soundtrack, which starts and
  ends with the intro

## Structure

Two state enums, each answering one question:

```rust
/// Where the player is.
enum Screen {
    #[default]
    Splash,
    Menu,
    Loading,
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
Splash --> Menu --> Play --> Loading --> Intro --> Gameplay --> Esc --> Menu
             |               waits for      |
             |               the frames     +--> skipped, same keys as the splash
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
| `src/screens.rs` | `Loading`, `Intro` and `Gameplay` variants, registers the plugins |
| `src/screens/menu.rs` | new — background, scrim, the 4 buttons, the veil both ways, the music |
| `src/screens/loading.rs` | new — the progress bar and the wait for the intro frames |
| `src/screens/intro.rs` | new — the frame sequence, its audio and the skip |
| `src/screens/gameplay.rs` | new — background colour only |
| `src/overlays.rs` | new — the `Overlay` state, its plugins, and the Esc system |
| `src/overlays/settings.rs` | new — fullscreen checkbox, volume slider, Close |
| `src/overlays/credits.rs` | new — static text, Close |
| `src/ui.rs` | new — button widget, palette, colouring and the two UI sounds |
| `src/fade.rs` | new — the shared fade timing, unplanned: see "What changed while building it" |
| `src/assets.rs` | new — `images_in`, the folder reader the splash, loading and intro share |
| `src/main.rs` | `mod ui;`, `mod overlays;`, `mod fade;` and `mod assets;` |
| `Cargo.toml` | `features = ["jpeg", "wav"]` on the bevy dependency, plus `rand` for the slideshow |
| `assets/menu/background.jpg` | moved from `assets/sample.jpg` — placeholder art |
| `assets/menu/music.ogg` | new — the looping menu track, converted from mp3, placeholder |
| `assets/splash/bevy.png` | renamed from `assets/splash/bevy_logo_dark.png` |
| `assets/splash/logos/` | new — shown one after another once the Bevy logo is done |
| `assets/loading/*.jpg` | new — the loading slideshow's placeholder pictures |
| `assets/intro/frames/frame_000.png` … `frame_090.png` | 90 frames, moved out of `assets/` |
| `assets/intro/music.ogg` | the soundtrack, which arrived named `frame_038.png` |
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
- `fn button_color(hovered: bool, pressed: bool) -> Color` — the pure function the system
  calls, and what the tests cover. Private, like `splash.rs`'s `logo_alpha`: its caller
  and its tests are both in this file
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

### Three names the prelude does not give you

Verified against the local 0.19.1 sources, because each of these fails in a way the
compiler will not warn about:

| Name | Where it lives | The trap |
| --- | --- | --- |
| `Button` | `bevy::ui_widgets` | `bevy::prelude::Button` is the *classic* one |
| `Hovered` | `bevy::picking::hover` | Nothing inserts it for you |
| `Pressed` | `bevy::ui` | A marker with no field — read it with `Has<Pressed>` |

`Button` is the mild one: an explicit `use` shadows the prelude's glob, so
`use bevy::ui_widgets::Button;` compiles and means the widget. Wrong import, wrong
interaction model, and the `Hovered`/`Pressed` query silently matches nothing.

**`Hovered` is the one that costs an afternoon.** The engine's own updater only touches
entities that already carry the component (`bevy_picking-0.19.1/src/hover.rs:400`):

```rust
for (entity, hoverable) in hovers.iter_mut() {   // only those that already have it
    let is_hovering = hover_ancestors.contains(&entity);
    if hoverable.0 != is_hovering {
        commands.entity(entity).insert(Hovered(is_hovering));
    }
}
```

So `button()` spawns `Hovered::default()` itself, or hover never happens: no panic, no
warning, just a button that never lights up. `Hovered` and not `DirectlyHovered` on
purpose — it counts descendants, so the pointer over the button's `Text` child still reads
as hovering the button.

That `if` is also what makes the sound system's filters honest: the component is
`#[component(immutable)]` and only ever replaced when the value really changes, so
`Changed<Hovered>` means "entered or left", never "still hovering". It does fire once at
spawn, with `Hovered(false)`, which is why the system checks the value before playing
anything.

### Button sounds

`assets/ui/click.wav` and `assets/ui/hover.wav` (moved from `assets/ui_click.wav` and
`assets/ui_hover.wav`). The folder mirrors the module that owns them — `src/ui.rs` and
`assets/ui/` — the same way `splash.rs` and `assets/splash/` already line up. The folder
names the domain, so the files do not repeat it.

The handles load once into a `ButtonSounds` resource — not `UiSounds`, because the module
is already `ui.rs` and the prefix would repeat it, the same redundancy the `BUTTON_*`
constants avoid. The system spawns a one-shot player per press:

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
| Music, and anything longer than a second | OGG | Where compression finally matters, and it needs no extra feature |

On the GPU a PNG and a JPEG both end up as uncompressed RGBA, so the format only decides
the size of what you ship, never the video memory. The placeholder background is 38 KB as
JPEG and 150 KB re-encoded to PNG, because its worn stamp texture is noise, and noise is
what PNG compresses worst.

Licences were checked, not assumed: `hound` (wav) is Apache-2.0 and `zune-jpeg` is
MIT/Apache-2.0/Zlib, all three already in `about.toml`, and `cargo about generate --fail`
passes unchanged.

### The music was an mp3

It arrived as `assets/mainmenu-music.mp3`, 5.8 MB, and was converted rather than kept.
Size is the smaller of the two reasons — 4.2 MB as vorbis, at the same 7:34:

```
ffmpeg -i mainmenu-music.mp3 -vn -c:a libvorbis -q:a 2 assets/menu/music.ogg
```

The real reason is that **mp3 is a feature this project does not have**. `audio` pulls in
`vorbis` and nothing else (`bevy-0.19.1/Cargo.toml:2645`), so keeping the mp3 would mean
adding `"mp3"` next to `"jpeg"` and `"wav"`, another decoder in the tree and another
licence to review — for a file with no reason to stay mp3. Converting costs one command
and leaves `Cargo.toml` alone.

`-vn` is not optional: the mp3 carried its cover art as a second stream, which has no
business in the game's assets.

Two notes for whoever replaces it. The re-encode is lossy over a lossy source, which is
the price of the format it arrived in, not of vorbis — author the real track straight to
ogg and the question disappears. And mp3 keeps encoder padding at both ends, so this
file's loop seam is unlikely to be clean; a track meant to loop should be trimmed to the
loop point in the editor that produced it.

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

**The veil.** A black node covering the screen, running in one direction or the other:

```rust
#[derive(Component)]
struct Veil {
    timer: Timer,
    fading_out: bool,
}
```

It follows `splash.rs`: a `Timer` on the component, an `EaseFunction`, and a pure
function next to its tests — `veil_alpha(elapsed_secs, fading_out) -> f32`.

Nothing is factored out of `splash.rs` to share with it. That fade has four phases and
this one has a single ramp in either direction; a common helper would be an abstraction
over two things that only look alike.

**Coming in.** Spawned opaque on every `OnEnter(Screen::Menu)`, the return from gameplay
included, fading to nothing and then despawned. There is no check for where the player
came from, which is the same principle the overlay follows.

**Going out.** Play does *not* change the state. It spawns a second veil, this one
`fading_out`, and the veil's own system sets `Screen::Loading` when the timer finishes.
That indirection is the entire cost of the effect, and it is one `if` at the end of a
system that already exists.

The destination is hardcoded. `Play` is the only button that fades, so there is nothing
to remember: Quit exits, Settings and Credits open an overlay over a menu that stays put.

**Despawning between the two matters.** A veil left sitting at alpha 0 would be invisible
and still block every click, by the same `Pickable` default that makes it useful. So the
entrance veil is removed when it finishes, and Play spawns a fresh one.

That blocking is what makes the exit safe, too: once the menu starts fading out, no
further button can be pressed, so there is no way to hit Quit halfway through leaving.

## Menu music

`assets/menu/music.ogg`, looping for as long as the menu is on screen:

```rust
commands.spawn((
    AudioPlayer::new(asset_server.load("menu/music.ogg")),
    PlaybackSettings::LOOP,
    MenuMusic,
    DespawnOnExit(Screen::Menu),
));
```

It belongs to `menu.rs` and to nothing else. `DespawnOnExit(Screen::Menu)` is the whole
lifetime rule: the track starts when the menu does and dies when the player leaves it,
and coming back from gameplay starts it again from the top. Remembering the position
across a whole game session would mean a resource outliving the screen, for an effect
nobody notices after three minutes of gameplay.

**The overlays do not stop it**, and that needed no arranging: opening Settings or
Credits never leaves `Screen::Menu`, so the entity is never despawned. The overlay
decision at the top of this page is what pays for that.

**No loading screen is involved.** Spawning an `AudioPlayer` whose handle is still
loading is not an error — `play_queued_audio_system` skips it and tries again next frame
(`bevy_audio-0.19.1/src/audio_output.rs:110`), because its query is `Without<AudioSink>`:

```rust
let Some(audio_source) = audio_sources.get(&source_handle.0) else {
    continue;
};
```

So the music simply starts a beat late on a cold disk, and there is nothing to wait for.

### The volume has to be written every frame

The slider in Settings writes `GlobalVolume`, and that is enough for the button sounds:
each click spawns a fresh sink and reads the value on the way in. **It is not enough for
anything already playing**, and the engine says so in its own doc comment
(`bevy_audio-0.19.1/src/volume.rs:7`):

```rust
/// Note: Changing [`GlobalVolume`] does not affect already playing audio.
```

The reason is one line in `audio_output.rs:193` — `sink.set_volume(settings.volume *
global_volume.volume)` runs once, when the sink is created, and never again. With only
one-shots in the app that was invisible. With a track that plays for minutes it means
dragging the slider does nothing until the player leaves the menu and comes back: a
broken-looking slider, and no compile error anywhere.

So the music owns its own volume, in one system next to it:

```rust
/// Keeps the music at the player's volume, fading it with the veil.
fn update_menu_music_volume(
    music: Single<&mut AudioSink, With<MenuMusic>>,
    veil: Option<Single<&Veil>>,
    global_volume: Res<GlobalVolume>,
)
```

`Volume` multiplies with `Volume` (`volume.rs:250`), so the two factors compose with no
arithmetic of ours:

```rust
sink.set_volume(global_volume.volume * Volume::Linear(fade));
```

The alternative was a shared system re-applying `GlobalVolume` to every live sink
whenever it changes. Turned down: there is exactly **one** long-lived sink in this app, so
that is a general mechanism built for a single case, and it would fight the fade below for
ownership of the same value.

`Single` is right here even though the sink appears late: no sink, no match, the system
skips the frame, and that is exactly the behaviour wanted while the file is still loading.

### The fade falls out of it

Because that system already writes the volume every frame, the fade costs one factor:

```rust
let fade = veil.map_or(1.0, |veil| {
    1.0 - veil_alpha(veil.timer.elapsed_secs(), veil.fading_out)
});
```

Which gives, for free and in both directions:

| When | Veil | Music |
| --- | --- | --- |
| Entering the menu | opaque, fading out | silent, rising to the player's volume |
| Sitting in the menu | despawned | the player's volume, unchanged |
| Play pressed | clear, fading in | falling to silence as the screen goes black |

`veil_alpha` is reused, not re-derived, so the picture and the sound share one curve and
one set of tests. The veil being despawned between the two fades is what makes
`map_or(1.0, ...)` the whole rule rather than a special case: no veil means no fade.

**The ramp is linear in amplitude**, not in perceived loudness — the same thing
`volume_from_slider` exists to correct further down. It is left as it is: over a fade
this short the difference is hard to hear, and `veil_alpha` is already eased. If the exit
ever sounds like it drops early and then lingers, squaring the factor is the fix, and it
is one `powi(2)` in the line above.

## Loading

Between the menu and whatever it launches, one screen whose only job is to hold the flow
until the next destination is ready.

```
Menu --[Play]--> Loading --> Intro --> Gameplay
       fade out   waits for
                  the frames
```

It is not a placeholder for a future need: it has real work today. The intro is 91 files
and 16 MB, they load asynchronously, and starting the frame timer before they arrive
would show blank frames. That wait has to live somewhere, and a screen that shows what is
happening beats hiding it inside `intro.rs`.

The menu has already faded to black by the time this screen appears, so a fast load shows
nothing flickering — black to black.

**The bar.** Count the handles that are ready and draw the proportion:

```rust
asset_server.is_loaded_with_dependencies(id)   // bevy_asset-0.19.1/src/server/mod.rs:1331
```

```rust
/// Returns how far along the load is, from 0.0 to 1.0.
fn loading_progress(loaded: usize, total: usize) -> f32
```

The pure function earns its tests on the edge that is easy to get wrong: `total` of zero
returns 1.0, not a division by zero.

**A failed asset must not hang the screen.** `LoadState` has a `Failed(Arc<AssetLoadError>)`
variant (`mod.rs:2261`), and counting only successes means one missing file leaves the
player stuck on a bar at 99% with no way out. Failures count as finished, with a `warn!`
naming the file. A frame missing from an intro is cosmetic; a screen that never ends is
not.

**Where the handles live.** Not on an entity with `DespawnOnExit(Screen::Loading)` —
that would drop them the instant the intro needs them. They go in a resource, inserted
here and removed on `OnExit(Screen::Intro)`, which is what gives the 177 MB back.

**The soundtrack loads here too**, once it exists. It is one more handle in the same
resource and one more item in the same count, so nothing about this screen changes to
accommodate it. Waiting for it matters more than waiting for the frames: audio that
starts late is audio out of sync with the picture for the whole intro.

### Load Game will not play the intro

Starting a new game and resuming a saved one do not want the same thing: the intro plays
once, at the beginning, not every time someone continues.

```
New Game  --> Loading --> Intro --> Gameplay
Load Game --> Loading ------------> Gameplay
```

So `Loading` has a destination, and it comes from the button that opened it. Today there
is only Play, so the destination is `Screen::Intro` and nothing reads a field to know it.
When saves arrive, that becomes a small resource set alongside the handles, and it is the
only structural change — the screen, the bar and the waiting stay exactly as written.

This is not the "remember where I came from" problem the overlay was built to avoid. That
one was about going *back*, which is why dissolving it was better than tracking it. This
one goes *forward*, and only the caller can know where.

## Intro

Bevy plays no video, so the intro is what a video would have been taken apart into: a
sequence of stills swapped on a timer, with the soundtrack played alongside. Done right
nobody can tell the difference — it *is* what a video file holds, minus the codec.

**91 frames**, `assets/intro/frame_000.png` to `frame_090.png`, 960x540, 16 MB on disk.

```
Loading --> Intro --> Gameplay
              |
              +-- any skip key or click, same set splash.rs already defines
```

It is a `Screen`, not part of gameplay: it has a beginning, an end, and it can be
skipped — the same shape as the splash. `src/screens/intro.rs` ends up reading like
`splash.rs` with an index instead of an alpha.

**Memory.** 91 textures at 960x540 come to roughly **177 MB of video memory** —
uncompressed RGBA is what the GPU holds, whatever the file on disk was. That is fine
while the intro runs and pure waste afterwards, so the resource holding the handles is
removed on `OnExit(Screen::Intro)` and the memory goes back.

**Loading happens before this screen**, in `Screen::Loading`, so by the time `intro.rs`
runs, every frame is in memory and the timer can just start. That also means `intro.rs`
never touches the asset server: it receives handles and plays them.

`bevy_asset_loader` was re-examined here, now that the count went from 3 assets to 94 and
there is a real loading screen to drive. It still loses: what it would replace is one
`is_loaded_with_dependencies` call and a `count()`, and the progress bar has to be drawn
either way.

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

While authoring it: **start and end on black.** That is what replaces a fade out on both
sides of the intro — see "Transitions, and the two cuts left".

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

The menu fades at both ends. Every other screen change is a cut:

| Change | What the player sees |
| --- | --- |
| Menu → Loading | the menu fades to black, and the loading screen is black too — nothing |
| Loading → Intro | a cut into `frame_000`, off a black screen |
| Intro → Gameplay | a cut out of `frame_090` |
| Gameplay → Menu | a cut to black, then the menu fades up out of it |

The loading screen landing between the menu and the intro helps rather than hurts: the
menu's fade out ends on black and the loading screen starts on black, so that join is
invisible without anyone arranging it.

**The menu fades on both sides. Nothing else fades.** The music rides the same veil, so
every fade above is heard as well as seen — see "Menu music". No other sound is involved:
the two UI clicks are one-shots and the intro's soundtrack starts with the intro.

```
Splash ──fade out──>│<──fade in── Menu
                                    │
                               [Play]
                                    │
                          fade out ─┤   the menu goes to black
                                    │
                               Loading   black already, so nothing shows
                                    │
                               (cut)
                                    │
                                 Intro
                                    │
                               (cut)
                                    │
                               Gameplay
                                    │
                                [Esc]
                                    │
                            fade in ─┘   the menu comes back faded
```

The veil belongs to the menu, and only the menu. The intro gets no fade of its own: it is
a placeholder, and the real one will handle its own opening. Leaving it out also keeps
the veil inside `menu.rs` instead of turning it into shared machinery every screen has to
know about.

The cut *into* the intro is not a problem to solve. The menu has already gone black by
then, so the only visible change is whatever `frame_000` happens to be — and when the
real intro is exported, starting it dark makes the join invisible for free.

**Coming back from gameplay works the same way, and needs no extra code.** Esc cuts
straight to black and the menu then draws itself in. There is no separate "cut to black"
step: the menu spawns with the veil already at alpha 1.0, so the first frame after the
state change *is* the black screen. The cut and the start of the fade are the same
instant. Gameplay never fades out; it does not have to.

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

It spawns the fading-out veil for Play, sets `NextState<Overlay>` for Settings and
Credits, or
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

That resource reaches every sound spawned *after* the drag, which is all of them but one.
The menu music is already playing while the slider moves, and it reads `GlobalVolume`
itself every frame for that reason — see "The volume has to be written every frame".

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

`assets/menu/background.jpg`, `assets/menu/music.ogg`, `assets/ui/click.wav`,
`assets/ui/hover.wav`, the pictures in `assets/loading/`, the logos in
`assets/splash/logos/` and the frames and soundtrack in `assets/intro/` are here so the
screens have something to show and something to play. They are meant to be replaced —
a game's own art and sounds are most of its personality, and none of it should come from
a starter template.

The one asset that stays is `assets/splash/bevy.png`, for the reason already
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

Esc does nothing in `Loading` either. There is nothing to skip: the screen ends when the
files are in memory, and leaving early would only mean arriving at the intro without them.

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
| `PlaybackSettings::LOOP` | looping the menu music, with no timer of ours |
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

## What changed while building it

Everything above is the design as it was approved, and it stays as written — a spec is the
history of *why*, and rewriting it to match the code would throw away the only thing it is
for. This section is the other half of that history: where the build disagreed with the
plan, and what changed the answer. Ten of them, in the commits from `9ad7b1f` to `4dd1ddc`.

### The menu is not gated on `Overlay::None`

"Clicks underneath" ends by saying the menu systems are gated with
`.run_if(in_state(Overlay::None))` "so correctness does not rest on that subtlety". They are
not. `menu.rs` gates on the screen and nothing else:

```rust
(animate_veils, update_menu_music_volume.after(animate_veils))
    .run_if(in_state(Screen::Menu)),
```

The extra gate was not belt and braces, it was a bug. Those two systems are the veil
animation and the music volume, and both have to keep running *while* an overlay is open:
opening Settings during the entrance fade would have frozen the veil at whatever alpha it
had reached, and pinned the music at that frame's factor — a grey screen behind the panel
and a track stuck half-way up, both staying that way until the overlay closed.

What the gate was written to guard was never clicks in the first place. The menu has no
polling input system; its buttons are observers on an `Activate` event, and that event can
only come from a picking hit the overlay already blocks. The `Pickable` default is not a
subtlety being worked around, it *is* the mechanism, and the argument for it above is what
holds.

### The fullscreen toggle keeps `Role::CheckBox`, and gains a label

"Settings behaviour" quotes the widget's own doc saying to override the role with `Switch`
"for a toggle switch, which is what this is". The override is not there: the checkbox
spawns with the role its `#[require]` gives it
(`bevy_ui_widgets-0.19.1/src/checkbox.rs:35`).

It is not a toggle switch. What is drawn is a square that turns from red to white, so
`CheckBox` describes it and `Switch` would describe something the player never sees.
Overriding also means writing `AccessibilityNode(accesskit::Node::new(Role::Switch))` out by
hand next to `Checkbox`, replacing what the require attribute already did — a line to keep
in step with the widget, for a worse description. If the square ever becomes a sliding
switch, the role moves with the drawing.

What the accessibility tree really lacked was the word "Fullscreen". The label is a sibling
`Text` node, not a child, so a screen reader landing on the checkbox finds an unnamed
control. `AccessibleLabel` (`bevy_ui-0.19.1/src/accessibility.rs:189`) is a component whose
hooks write into `AccessibilityNode`, so it is one line in the bundle:

```rust
AccessibleLabel::new("Fullscreen"),
```

The volume slider has the same problem and the same line. `Role::Slider` was never the
missing part; the name was.

### `OVERLAY_Z` lives in `overlays.rs`

"Drawing on top" shows the constant without saying where it goes, and a constant next to
the panel it applies to is the obvious reading. Both overlays spawn a panel, so it sits in
`src/overlays.rs`, above `credits.rs` and `settings.rs`, as the component rather than a bare
`i32`:

```rust
const OVERLAY_Z: GlobalZIndex = GlobalZIndex(100);
```

Typed, the spawn sites only list it in their bundle, and there is one place to change if the
number ever has to move. Two copies of a z-order is how two overlays end up at different
depths.

### The volume slider starts from the real volume

The spec's slider spawns with `SliderValue(50.0)`. That is correct exactly once. Drag the
volume to 10, close Settings, open it again, and the thumb is back in the middle while the
sound is not — the panel showing a number the game does not use.

So the value is read from the state that already owns it:

```rust
let slider_value = slider_from_volume(global_volume.volume.to_linear());
```

`GlobalVolume` holds a linear amplitude and the slider is 0..=100 squared, so this needed
`volume_from_slider` to gain an inverse:

```rust
fn slider_from_volume(volume: f32) -> f32 {
    volume.sqrt() * 100.0
}
```

Both directions are tested at silent, a quarter and full. A pair of inverses is worth the
six tests rather than three: a `powi(2)` and a `sqrt` can drift apart with no compile error
and no visible symptom until someone reopens the panel.

### The loading screen shows a slideshow

The spec's loading screen is the progress bar over black, and leans on the join being
invisible: "the menu has already faded to black by the time this screen appears, so a fast
load shows nothing flickering". True, and the load is not fast — 16 MB of frames is seconds
on a cold disk, spent looking at a bar.

So `assets/loading/` is a folder of pictures, and any image in it is a slide. Nothing is
listed in code: the folder is the whole contract, the same way `assets/splash/logos/` is.
Each slide fades in, holds and fades out, and the order is shuffled once per visit so a
short load does not always show the same picture first.

That shuffle is the one new crate in the tree, and it is less of an exception than it looks:
`rand` was already there as a dependency of Bevy, so `rand = "0.10.3"` in `Cargo.toml` adds
a direct use of a crate the lockfile already pinned rather than a new node in the graph, and
`about.toml` needed no new licence.

Reshuffling per lap was left out with its ceiling named in a `ponytail:` comment — a second
lap repeats the first one's order, which needs a load long enough to see three slides twice.

The progress bar is as designed, counting the frames plus the music as one more item.

### The intro reads a folder, not an index

The spec's last word on the intro is explicit: "`intro.rs` should build its handle list by
index, `frame_{i:03}.png`, not by reading the directory. Directory order is the thing that
went wrong here, and a loop over `0..FRAME_COUNT` cannot go wrong the same way."

It reads the directory. `load_folder("intro/frames")`, sorted by file name.

Two things changed the answer. The first is that a gap appeared in the sequence:
`frame_038.png` left it (below), so `0..FRAME_COUNT` would have asked for a file that is not
there, and the count would have to be edited every time the intro is re-exported. A folder
cannot go stale that way — whatever is in it is the sequence.

The second is that by then three screens needed the same read: the splash's logos, the
loading slideshow and the intro's frames. `CONTRIBUTING.md` says shared code moves out on
the third user, so it did, into `src/assets.rs`:

```rust
pub(crate) fn images_in(folder: &LoadedFolder) -> Vec<Handle<Image>>
```

It sorts by asset path and drops anything that is not an image, so a `.txt` or a stray
`.ogg` in the folder is ignored instead of breaking the run. It is not a plugin and has no
systems — the screens call it while spawning, which is written in the file where someone
looking for the missing `plugin` function will find it.

The spec's worry stands, it just moves into a name. Sorting is only time order if the names
sort that way, so zero-padded names are the contract now, and it is stated in the function's
doc comment rather than in a spec nobody re-reads. The `ffmpeg` line above already produces
them; only its output folder moved, to `assets/intro/frames/`.

### `frame_038.png` was the soundtrack

The mirror image of the mix-up the spec already documents. `assets/intro_audio.ogg` turned
out to be a PNG; `frame_038.png` turned out to be an Ogg file — first four bytes `4f 67 67
53`, `OggS`. The same export swapped a pair of extensions in both directions, and finding
the first one is what prompted checking the rest.

It is now `assets/intro/music.ogg`, and three things the spec left open close with it:

| The spec says | What is true now |
| --- | --- |
| 91 frames, `frame_000` to `frame_090` | 90 frames — `038` is a gap in the run, not a missing file |
| "There is no audio" | the intro has its soundtrack, `PlaybackSettings::ONCE` |
| `INTRO_FPS` has nothing to calibrate against | `INTRO_FPS = 15.0`: 90 frames over the track's ~6 s |

So the calibration the spec deferred was possible all along, and the constant is a measured
number rather than a guess between 24 and 30. The comment on it says what it was measured
against, because the next export invalidates it.

"The soundtrack loads here too" predicted the loading screen would not change to
accommodate it, and it did not: one more handle in `IntroAssets`, one more item in the bar's
total, one more `is_loaded_with_dependencies` in the wait.

Everything else in "These frames are a placeholder too" still holds. The countdown is still
scrambled, and it is still not a defect to fix.

### No `warn!` per failed asset

The spec says failures count as finished, "with a `warn!` naming the file". The first half is
built and the second is not, because Bevy already does it. The load task logs the error
itself, with the path and the reason in it
(`bevy_asset-0.19.1/src/server/mod.rs:593`):

```rust
error!("{}", err);
```

A `warn!` of ours would print the same failure a second time, in a worse form — the screen
knows a handle failed, not why. So the screen only has to not hang, which is one `||` per
handle:

```rust
let frames_done = frames_load_state.is_loaded() || frames_load_state.is_failed();
```

The argument above is untouched: a missing frame is cosmetic, a screen that never ends is
not. Only the logging half was already written by someone else.

### The splash plays a folder of logos

Not a change to this design so much as one it made cheap. The splash showed the Bevy logo
and left; it now shows the Bevy logo and then every image in `assets/splash/logos/`, each
with its own fade and its own skip.

The reason is what this repository is. A template's splash screen is the first place whoever
clones it puts their own logo, and "drop a PNG in `assets/splash/logos/`" is a smaller
instruction than "open `splash.rs` and copy the spawn code". `images_in` already existed for
the intro, so the whole feature is a `Vec<Handle<Image>>`, an index and `next_logo`.

The Bevy logo stays first and stays named in code, for the reason in `docs/template.md`,
which has not changed. Its file is now `assets/splash/bevy.png` — the folder beside it
already says the rest, the same rule the other asset names follow.

A folder still loading when the Bevy logo's fade ends is skipped rather than waited for,
with the ceiling named in a `ponytail:` comment next to it. One local PNG against a fade
that lasts three seconds is not a race worth code.

### One `Fade` for the whole game

The largest of the ten, and the one that overturns an argument rather than a detail. "The
veil" says it outright:

> Nothing is factored out of `splash.rs` to share with it. That fade has four phases and
> this one has a single ramp in either direction; a common helper would be an abstraction
> over two things that only look alike.

That was right with two users and wrong with three. The loading slideshow arrived, and a
slide fades in, holds and fades out — the splash's own phases with a delay of zero, not a
third thing that resembles them. `CONTRIBUTING.md` says shared code moves out on the third user,
and this is what the rule is for.

`src/fade.rs` holds the timing and nothing else:

```rust
pub(crate) struct Fade {
    pub(crate) delay_secs: f32,
    pub(crate) fade_in_secs: f32,
    pub(crate) hold_secs: f32,
    pub(crate) fade_out_secs: f32,
}
```

`alpha(elapsed_secs)` samples one `FADE_EASING` shared by the whole game, so no fade feels
different from another, and `total_secs()` is what the callers set their timers from instead
of adding the four constants up by hand.

The screens keep their own timing, as a `const` next to the durations that feed it:
`LOGO_FADE` in `splash.rs`, `VEIL_OPENING` and `VEIL_CLOSING` in `menu.rs`, `SLIDE_FADE` in
`loading.rs`. What moved is the curve, not the numbers.

The menu's two directions fall out of the four fields, which is what made the single ramp
stop looking like a different shape:

| Direction | `Fade` |
| --- | --- |
| Opening | fade out only — starts black, clears |
| Closing | fade in, then `hold_secs: f32::INFINITY` |

The infinite hold is not a trick, it is the behaviour the spec asked for: the closing veil
has to stay black until the state actually changes, and a fade that ran to zero would flash
the menu back for a frame.

`veil_alpha` survives as the two-line chooser between those two constants, keeping the six
tests it already had — they did not have to change, which is the sign the split landed in
the right place. `logo_alpha` is gone; the curve it held is now `Fade::alpha`,
with seven tests of its own covering the phases and the two degenerate cases (no fade in, no
fade out) that the per-screen constants actually use.

### And the tests

"Six pure functions is all the logic there is" became ten, and the four extra were found the
same way — a branch that could be wrong without saying so:

| Function | Why it earned tests |
| --- | --- |
| `fade::Fade::alpha` | the four phases, and the zero-length fade in and out the menu uses |
| `screens::splash::next_logo` | the end of the list, which is what leaves the screen |
| `screens::loading::next_slide` | the wrap back to the first slide |
| `overlays::settings::slider_from_volume` | the inverse of `volume_from_slider`, in both directions |

`ui::button_color`, `veil_alpha`, `loading_progress`, `frame_index`, `toggled_window_mode`
and `volume_from_slider` are covered as planned. `frame_index` gained one case the folder
reading created: no frames at all, which a hardcoded count could not produce.

One thing in "Files" above never landed: `GAME_TITLE`. `main.rs` still writes the window
title inline. "The game's name" argued for the constant and the argument is unchanged —
it is simply not written yet.

## Tests

The rule stands: logic in pure functions, systems stay thin. What gets covered:

| Function | Cases |
| --- | --- |
| `ui::button_color` | idle, hovered, pressed |
| `screens::menu::veil_alpha` | both directions: opaque/clear at each end, half way at the midpoint |
| `screens::loading::loading_progress` | none loaded, half, all, and a total of zero |
| `screens::intro::frame_index` | first frame at 0, last frame at the end, clamped past it |
| `overlays::settings::toggled_window_mode` | windowed to borderless, borderless to windowed |
| `overlays::settings::volume_from_slider` | silent at 0, unchanged at 100, a quarter at 50 |

Floats compared with a tolerance, as in `splash.rs`.

Six pure functions is all the logic there is. Everything else is spawning nodes and
setting a state, which a unit test would only restate.

The music's volume is deliberately not a seventh. It is `global_volume * (1.0 - alpha)`,
and both halves are already covered: `veil_alpha` by its own tests above, and the
multiplication by `Volume`'s implementation. A `music_volume` function would exist only to
give the multiplication a name and a test file.

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
   sounds. `button()` has no caller until step 3, and `just check` runs
   `cargo clippy -- -D warnings`, so this step ends red without one line on it:

   ```rust
   #[expect(dead_code, reason = "the menu in step 3 is its first caller")]
   ```

   The line is the cheapest of the three ways out, and the only one that cannot rot —
   see "Shared code that arrives before its caller"
2. `src/screens/gameplay.rs` plus the `Gameplay` variant — the simplest screen, proves
   the pattern end to end
3. `src/screens/menu.rs` — background, scrim, the four buttons with Play and Quit
   working, the veil fading both ways with its tests, and the looping music fading with
   it. The music's volume system lands here rather than with the slider in step 5:
   `GlobalVolume` already exists and defaults to full, so the system is correct from this
   commit on, and step 5 only gives the player a way to move the value. Play sends the
   player to `Screen::Gameplay` for now, because that is the only destination that exists
   yet; step 6 slots `Loading` and `Intro` in between, and the line it changes is the one
   at the end of the veil system
4. `src/overlays.rs` plus `src/overlays/credits.rs` — the `Overlay` state, the Esc
   system and the simplest overlay, together because the state needs one user
5. `src/overlays/settings.rs` — the fullscreen checkbox and the volume slider, with tests
6. `src/screens/loading.rs` and `src/screens/intro.rs`, plus the `Loading` and `Intro`
   variants — the progress bar, the frame sequence and the skip, with `loading_progress`
   and `frame_index` tested. Together because neither is worth anything alone: a loading
   screen with nothing to load, or an intro whose frames may not have arrived. Last of
   the code because it is the only step waiting on assets that are not final yet
7. `docs/flow.md` — the finished flow, plus one new bullet under Features in the
   `README.md`, and in `docs/template.md` the new files in the project structure and the
   line marking which assets are placeholders

### Shared code that arrives before its caller

Splitting `ui.rs` from the menu is what creates this, and it will happen again to any
shared module that lands before the feature that needs it. `dead_code` is a rustc lint,
a warning by default and harmless under `cargo run` — `just check` is what makes it fatal:

```
cargo clippy -- -D warnings
```

Two things about the lint decide the answer, and both were checked with `rustc` rather
than assumed:

- **Visibility is what triggers it.** The lint walks outward from roots, and a `pub` item
  is a root: the compiler cannot know whether something outside the crate calls it. A
  `pub(crate)` item is not, so the compiler sees every possible caller and sees none.
  `pub(crate) fn button` warns; the same function as `pub` does not, even in a binary.
- **It is transitive.** With `button()` dead, the sizing constants only it reads are dead
  too, and each one is its own error.

Three ways out:

| | Result |
| --- | --- |
| `#[expect(dead_code, reason = ...)]` on `button` | One line, and it covers the constants |
| Land `ui.rs` inside step 3, with the menu | No dead code ever, one bigger commit |
| Widen `pub(crate)` to `pub` | Rejected — silences the lint permanently, and lies about who may call |

The first wins on a property `allow` does not have: **`expect` fails once the expectation
comes true.** In step 3, the frame the menu calls `button()`, the build stops with `this
lint expectation is unfulfilled` and prints the reason back. It is not a note to remember;
it is the compiler handing the line back to be deleted. That an item with `expect` is also
treated as a root is what keeps it to one line instead of one per constant.

It also happens to be the idiom this project already chose: `allow_attributes` and
`allow_attributes_without_reason` are both `warn` in `Cargo.toml`, which is the same
policy — a lint attribute is allowed to exist, with its reason written beside it.

The third option is worth naming only because it is the tempting one. It ends with the
narrowest correct visibility replaced by the widest, to quiet a lint that was right.
Prefer the narrowest visibility that compiles, everywhere: `pub(super)` for the plugins,
`pub(crate)` for `button`, private for `button_color` and the other pure functions, which
the systems in their own file call.

Skipping `just check` until step 3 was considered and turned down. It would mean three
commits that fail the project's own gate, and — because `cargo test` runs inside `check` —
three steps without running the tests written along the way. The point of a step ending
green is the short loop, not the green.
