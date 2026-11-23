# Turning the template into a game

What to change once, at the start, and where things live afterwards.
For setup and the day-to-day commands, see the [README](../README.md).

## Starting a new game

[`create-bevy-starter`](https://github.com/Heryk13/create-bevy-starter) does step 0 and the
three renames of step 1, empties `docs/specs/`, drops the Branches section of the README and
of `CONTRIBUTING.md`, deletes `tools/`, the template's own scripts for its branch tree, and
makes the first commit. The steps below are the manual path, and what is left after it.

0. Start from the branch closest to the game: `main` for the bare template, or a variant
   such as `3d/base` or `2d/base` (see [Branches](https://github.com/Heryk13/bevy-starter#branches)).
   Every variant is `main` plus its own pieces, so every step below applies to it as well.
1. Rename the game in three places, keeping them in sync:
   - `name` in `Cargo.toml`, which also names the game's data folder (`%APPDATA%\<name>\` on
     Windows, `~/.local/share/<name>/` on Linux, `~/Library/Application Support/<name>/` on
     macOS), where `settings.ron` and the `saves/` folder live, each save a `.ron` with its
     picture, a `.jpg`, beside it. Renaming it leaves the old
     saves and settings behind, in the folder under the old name
   - `game` in `justfile`, which `just release` uses to find the executable
   - `GAME_TITLE` in `src/main.rs`, the name on the window and on the menu

   Then the two things that name the game's people and its release: `CREDITS_NAMES` in
   `src/overlays/credits.rs`, and `version` in `Cargo.toml`, which the menu shows in its corner.
2. Trim Bevy's default features in `Cargo.toml` to what the game uses, for smaller and faster builds.
   For a 2D game, keeping the `jpeg` and `wav` the template's assets need:
   ```toml
   bevy = { version = "0.20.0", default-features = false, features = ["2d", "ui", "audio", "jpeg", "wav"] }
   ```
   Every screen, overlay and the blur build with exactly these on 0.20.0, and ran with them on 0.19.1.
   For a 3D game `3d` takes the place of `2d`: it brings the glTF loader, the lit materials, the
   atmosphere, SSAO, SMAA and `Bloom` the playroom uses. `area_light_luts` is the table its sky
   portals' `RectLight` is drawn with (see [Lighting a level](#lighting-a-level)).
   ```toml
   bevy = { version = "0.20.0", default-features = false, features = ["3d", "ui", "audio", "jpeg", "wav", "area_light_luts"] }
   ```
   `3d/base` and every branch below it already build with exactly this line, so a game started
   from them skips this step; only a 3D game started from `main` trims by hand. The playroom
   builds with exactly these on 0.20.0.
   From `3d/player` down, physics is [avian3d](https://github.com/avianphysics/avian), trimmed
   the same way to what the player uses, with its debug drawing only in dev builds:
   ```toml
   avian3d = { version = "0.8.0", default-features = false, features = ["3d", "parry-f32", "collider-from-mesh", "bevy_scene", "parallel"] }
   ```
3. Keep the Bevy logo on the splash screen: it lets players know the game is made with Bevy
   and helps the engine grow. Use it unmodified (see [Assets](#assets)).
4. Replace the [placeholder assets](#placeholders) with the game's own.
   When the intro is re-exported, set `INTRO_FPS` in `src/screens/intro.rs` so the frames
   last as long as the new soundtrack, otherwise picture and sound end apart.

## Project structure

```
assets/             game assets, loaded at runtime
  dev_tools/        photo mode's shutter click, for dev builds
  fonts/            Noto Sans JP, the font of every text, and its license
  gameplay/         playroom.glb, the test level every 3D game starts from
  intro/            the intro's soundtrack
    frames/         the intro's frames, played in file name order
  loading/          the loading screen's slideshow pictures
  locales/          one folder per language, each with its main.ftl translations
  menu/             the menu's background and music
  shaders/          the glass: kawase_down.wesl and kawase_up.wesl blur, glass.wesl draws it
  splash/           the Bevy logo
    logos/          the game's own logos, shown after the Bevy one
  ui/               the button sounds
src/
  main.rs           app entry point
  assets.rs         reading a loaded folder's images, shared by the screens
  dev_tools.rs      the tools a game is made with, left out of release builds
  dev_tools/
    file_explorer.rs  Numpad 0: a drawer browsing the gallery, the assets and a trash
    file_explorer/
      actions.rs    every change to the disk, dragging, files dropped from the OS
      dialog.rs     the layer of the explorer's questions and right-click menu
      drawer.rs     the slide, the height and the grip that resizes it
      list.rs       the folder shown, a row an entry
      listing.rs    reading the disk: folders, Recents, sizes, dates, kinds
      menu.rs       the right-click menus
      preview.rs    the selected picture, decoded on another thread
      rename.rs     renaming in place, and the names refused
      scroll.rs     the faint scrollbars
      title_bar.rs  the close button, the places, the path
      trash.rs      Trash: deleted entries, trash.ron, restore, delete for good
      tree.rs       the place's folders and files as a tree
      usage.rs      where an asset is used, asked before changing it
    fly.rs          F7: the player flies through everything, Space and Shift up and down
    fps.rs          the FPS overlay and its key
    free_camera.rs  photo mode's free camera, back where it was as photo mode ends
    keys_panel.rs   F1: a window listing every dev key
    photo_mode.rs   F3: the game frozen, the dev windows hidden, a bar of its keys
    photo_mode/
      photo.rs      E: a PNG of the game alone in gallery/, a flash, a click, a notice
    physics_debug.rs  F6: the outline of every collider, the player's capsule included
    prop_editor.rs  F10: props from assets/props/ placed into assets/map.ron, its state
    prop_editor/
      edits.rs      every change to the map: place, update, delete; holding, R, the wheel
      library.rs    the props on disk, files dropped in, favourites and recents
      window.rs     the drawer's tabs, search, sort and grid of thumbnails
    prop_editor_3d.rs  the 3D props: models, thumbnails, placing on the surface
    prop_editor_3d/
      mode_menu.rs  a right click: fixed, physics or no collision
    recording.rs    F9: a video of the game alone into gallery/, its states and REC
    recording/
      bar.rs        the bar: sound, frame rate, timer, Record
      capture.rs    the frame copied under the dev UI, read back; start and stop
      encoder.rs    H.264 and Opus into an .mp4, on a thread of its own
      sound.rs      the game's own sound, through Windows' process loopback
    slow_motion.rs  F4: the game slowed down, stepped with Numpad − and +
    speed.rs        F8: the player's speed, and a multiplier for its walk
  disk.rs           the data folder and safe file writes, shared
  fade.rs           the fade timing every screen shares
  i18n.rs           languages, translations and the UI font, shared
  input.rs          the game's actions and the keys they are bound to, shared
  overlays.rs       Overlay state and overlay plugins
  overlays/
    credits.rs      credits panel
    saves.rs        the lists of saved games: save new or over one, load or delete one
    settings.rs     the settings panel and its pages; the General page; settings.ron
    settings/
      controls.rs   the pages of controls: the actions, each beside the keycap of its key
  screens.rs        Screen state and screen plugins
  screens/
    splash.rs       splash screen
    menu.rs         main menu
    menu/
      veil.rs       black cover that fades the menu in and out
    loading.rs      loading screen: slideshow and progress bar
    intro.rs        intro: frame sequence with its soundtrack
    gameplay.rs     the game itself, the playroom so far
    gameplay/
      map.rs        the map file: where each prop stands, spawned in every build
      pause.rs      pause state and pause menu, freezing game time
      player.rs     the player's body, walking, and what a save keeps of them
      player/
        first_person.rs  the camera at eye height, the mouse look and the cursor lock
      playroom.rs   the test level, its colliders, sun and sky, and the kit its cameras carry
      playroom/
        lighting.rs   dark insides, lamps and the sky's light at openings, from the level's marks
        sun_glare.rs  the glare drawn over the sun, as a camera sees it
      props.rs      a prop's model and the body of its mode
  save.rs           the game in progress and the save files, shared
  save/
    picture.rs      the picture a save carries: taking it, encoding it, its file
  ui.rs             the look: palette, type and text, shared
  ui/
    button.rs       buttons, their paint and their sounds
    panel.rs        floating roots, cards, their entrance and dialogs
    blur.rs         a camera per floating layer and the glass behind every panel
art/                playroom.blend, the Blender source of playroom.glb
docs/
  template.md       this file: turning the template into a game
  flow.md           how the screens, overlays and fades fit together
  specs/            one folder per implementation, with its design and plan
.cargo/config.toml  linker configuration
about.toml          accepted dependency licenses
about.hbs           template for the license notices
justfile            project commands
LICENSE-MIT         the code's license: MIT or Apache-2.0, at your option
LICENSE-APACHE      the Apache-2.0 half of that license
README.md           what this is, requirements and commands
CONTRIBUTING.md     development loop and code conventions
CLAUDE.md           instructions for Claude Code
```

### A player of your own on F8

Speed's capsule shows whichever body carries a `Speedometer` from `dev_tools::speed`, and the
game walks it as many times faster as `SpeedMultiplier` says. A game's own player connects to
it in two places, each behind `#[cfg(feature = "dev")]`, since neither exists in a release
build: the body spawns with `Speedometer { unit: "m/s" }` (any unit; `2d/player` writes
`"px/s"`), and its walk multiplies its speed by the `SpeedMultiplier` resource. The speed is
measured from the body's `GlobalTransform`, so the player writes nothing else. `3d/player`
and `2d/player` do both in their `src/screens/gameplay/player.rs`. With no body carrying one,
the capsule shows `—`.

### Lighting a level

The sky lights every surface by the way it faces, ceilings or not, so a level's insides are
marked in Blender, and `src/screens/gameplay/playroom/lighting.rs` lights them from the marks.
A mark is an empty with a custom property `kind` (Object Properties → Custom Properties),
the way the playroom already names a door `"door"`:

- **`"interior"`**: a dark box, keeping the sky out. A Cube empty of display size 1, drawn
  on the walls' inner faces, from the floor to the ceiling's underside; a room that is not a
  box takes several, as the playroom's corridor takes three. An optional `sky_light` from
  0.0 to 1.0 lets that much of the sky in (`House_Interior` has 0.01); without one, the box
  takes `INTERIOR_SKY_LIGHT`. At most 8 boxes light a view at once.
- **`"light_point"`**: a lamp, a point light with shadows where the empty is
  (`LAMP_INTENSITY_LM`, `LAMP_RANGE_M`).
- **`"sky_portal"`**: the sky's light coming in at a door or window. An Image empty with no
  image, scaled to the opening and placed on the wall's inner face, its Z pointing in; the
  light is a `RectLight` that size, as bright as the sky over its area. It casts no shadows
  in Bevy 0.20, so it lights through walls within `SKY_PORTAL_RANGE_M`.

Blender's glTF exporter leaves custom properties out unless **Include → Custom Properties**
is on (File → Export → glTF 2.0): without it no mark reaches the game. Every other option of
the playroom's export is Blender's default.

How dark is dark is the eye, the camera's `AutoExposure` in `playroom.rs`:
`PLAYROOM_CAMERA_EYE_DARK_LIMIT_EV` is how many stops it brightens a dark view at most, and
`PLAYROOM_CAMERA_EYE_TARGET_EV` keeps daylight where the fixed exposure put it.

## Assets

`assets/splash/bevy.png` is the Bevy logo, shown on purpose so players know the game is made with Bevy.
It has no formal license; unmodified use in splash screens was
[informally allowed](https://github.com/bevyengine/bevy/discussions/10861) by the Bevy project lead,
so keep it unmodified.

### Placeholders

Everything else shipped in `assets/` is a placeholder, there to make the screens run. Replace it:

- `assets/menu/background.jpg` and `assets/menu/music.ogg`: the menu's artwork and looping track
  (the track is nine of Signature Sounds' [Space Tension Beds](https://signaturesounds.org),
  CC0, crossfaded into one seamless loop)
- `assets/ui/click.wav` and `assets/ui/hover.wav`: the sounds every button plays
- `assets/loading/*.jpg`: the loading screen's slideshow
- `assets/intro/frames/*.png` and `assets/intro/music.ogg`: the intro and its soundtrack

Three folders are read whole, so what to drop in them matters more than the file names:

- any image in `assets/loading/` becomes a slide, shown in a shuffled order
- any image in `assets/splash/logos/` becomes a logo, shown after the Bevy one
- `assets/intro/frames/` plays in file name order, so keep the names zero-padded
  (`frame_000.png`), or frame 10 comes before frame 2

Formats follow Bevy's features in `Cargo.toml`: `jpeg` for the photographic backgrounds and
`wav` for the short UI sounds, on top of the png and ogg vorbis Bevy already reads.

`assets/props/` and `assets/map.ron` are the prop editor's (F10): the props it offers, and where
each one placed stands. `main`, the Empty game, has neither, and a missing map is an empty one.
`3d/player` ships a Douglas pine (`douglas_pine.glb`) and `2d/player` a picture of it
(`douglas_pine.png`), each placed once in its map: delete it in the editor and its file to start
with none. A genre says which files its props are by inserting a `PropKind`, and observes
`PropFile` to show each prop.

`assets/fonts/` and `assets/locales/` are not placeholders, and they are not loaded at runtime
either: `src/i18n.rs` compiles them into the game with `include_bytes!` and `include_str!`, so
every word and every glyph exists from the first frame. Editing one needs a rebuild.

### The look

The colors and type sizes of every screen, panel and button are the `Config` section of
`src/ui.rs`: `ACCENT` is the white of hovered buttons and checked controls, `SURFACE` the tint
of the panels over their glass, `TEXT` and `TEXT_MUTED` the two text colors, both a white let
through in part, `DANGER` the red of Delete. Recoloring the game is editing those constants.
An icon is `icon(Icon::Trash2)`, any of [Lucide](https://lucide.dev)'s by its name: the
`lucide-icons` crate builds the font and the names into the game, so there is no icon file to
add. The game's name on the menu and the window is `GAME_TITLE` in `src/main.rs`. Anything that should float over the screens with the blur behind it is built on
`floating_root()` and `card()` from `ui.rs`; the `Floating` marker they carry is all the blur needs.
A panel on a layer that exists (`OVERLAY_Z_INDEX` for a new overlay) needs nothing else. A new
layer is a new `*_Z_INDEX` pushed onto `GlassLayers` from its plugin, or it logs a warning and
draws under every other layer, with no glass behind it (see "The cameras and the blur" in
`docs/flow.md`). A feature that stops everything behind the panels, as the pause menu does, runs
`freeze_background` from `ui::blur` when that begins and `unfreeze_background` when it ends, and
the glass over it is blurred once instead of every frame. Without them the glass is still right,
only slower. A camera that draws only the game's world may stop while `ScreensCovered` is on,
when a panel's blur hides all of it: it sets `Camera::is_active` from it in `PostUpdate`,
after `ScreensCoveredSystems` and before `CameraUpdateSystems` (see the same section).

### Translations

Every word the player reads is a key in `assets/locales/<code>/main.ftl`, in
[Fluent](https://projectfluent.org/) syntax. Code names the key, never the words:
`menu_word("menu-new-game")`, `Localized("pause-title")` (it brings its own `Text`),
`dialog("dialog-delete-save", ...)`. Text built from values, like a save's row, is formatted
with `Translations::text_with`.

- **Adding a word:** add its key to all three files. A test fails while any file lacks a key
  English has, or has one English lacks.
- **Adding a language:** a `Language` variant with its `code()`, `native_name()` and
  `source()` (an `include_str!` of its file) in `src/i18n.rs`, a place in `Language::ALL`,
  and its `main.ftl`. The Settings row is built
  from `Language::ALL`, so it needs no change. Check that Noto Sans JP draws its script; it does
  not draw Arabic, Hebrew, Korean or Chinese-only characters.
- **The Japanese file is a draft**, not yet read by a native speaker. Have one read it before
  a release.
