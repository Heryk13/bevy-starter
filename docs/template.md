# Turning the template into a game

What to change once, at the start, and where things live afterwards.
For setup and the day-to-day commands, see the [README](../README.md).

## Starting a new game

[`create-bevy-starter`](https://github.com/Heryk13/create-bevy-starter) does step 0 and the
three renames of step 1, empties `docs/specs/`, drops the README's Branches section and makes
the first commit. The steps below are the manual path, and what is left after it.

0. Start from the branch closest to the game: `main` for the bare template, or a variant
   such as `3d/base` or `2d/base` (see [Branches](https://github.com/Heryk13/bevy-starter#branches)).
   Every variant is `main` plus its own pieces, so every step below applies to it as well.
1. Rename the game in three places, keeping them in sync:
   - `name` in `Cargo.toml`, which also names the game's data folder (`%APPDATA%\<name>\` on
     Windows, `~/.local/share/<name>/` on Linux, `~/Library/Application Support/<name>/` on
     macOS), where `settings.ron` and the `saves/` folder live. Renaming it leaves the old
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
3. Keep the Bevy logo on the splash screen: it lets players know the game is made with Bevy
   and helps the engine grow. Use it unmodified (see [Assets](#assets)).
4. Replace the [placeholder assets](#placeholders) with the game's own.
   When the intro is re-exported, set `INTRO_FPS` in `src/screens/intro.rs` so the frames
   last as long as the new soundtrack, otherwise picture and sound end apart.

## Project structure

```
assets/             game assets, loaded at runtime
  fonts/            Noto Sans JP, the font of every text, and its license
  intro/            the intro's soundtrack
    frames/         the intro's frames, played in file name order
  loading/          the loading screen's slideshow pictures
  locales/          one folder per language, each with its main.ftl translations
  menu/             the menu's background and music
  shaders/          blur.wesl, the blur behind every floating panel
  splash/           the Bevy logo
    logos/          the game's own logos, shown after the Bevy one
  ui/               the button sounds
src/
  main.rs           app entry point
  assets.rs         reading a loaded folder's images, shared by the screens
  disk.rs           the data folder and safe file writes, shared
  fade.rs           the fade timing every screen shares
  i18n.rs           languages, translations and the UI font, shared
  overlays.rs       Overlay state and overlay plugins
  overlays/
    credits.rs      credits panel
    saves.rs        the lists of saved games: save new or over one, load or delete one
    settings.rs     fullscreen, volume and language, remembered in settings.ron
  screens.rs        Screen state and screen plugins
  screens/
    splash.rs       splash screen
    menu.rs         main menu
    menu/
      veil.rs       black cover that fades the menu in and out
    loading.rs      loading screen: slideshow and progress bar
    intro.rs        intro: frame sequence with its soundtrack
    gameplay.rs     the game itself, empty so far
    gameplay/
      pause.rs      pause state and pause menu, freezing game time
  save.rs           the game in progress and the save files, shared
  ui.rs             the look: palette, type and text, shared
  ui/
    button.rs       buttons, their paint and their sounds
    panel.rs        floating roots, cards, their entrance and dialogs
    blur.rs         the floating camera and the blur behind every panel
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

`assets/fonts/` and `assets/locales/` are not placeholders, and they are not loaded at runtime
either: `src/i18n.rs` compiles them into the game with `include_bytes!` and `include_str!`, so
every word and every glyph exists from the first frame. Editing one needs a rebuild.

### The look

The colors and type sizes of every screen, panel and button are the `Config` section of
`src/ui.rs`: `ACCENT` is the amber of hovered buttons and checked controls, `SURFACE` the panels,
`TEXT` and `TEXT_MUTED` the two text colors, `DANGER` the red of Delete. Recoloring the game is
editing those constants. The game's name on the menu and the window is `GAME_TITLE` in
`src/main.rs`. Anything that should float over the screens with the blur behind it is built on
`floating_root()` and `card()` from `ui.rs`; the `Floating` marker they carry is all the blur needs.

### Translations

Every word the player reads is a key in `assets/locales/<code>/main.ftl`, in
[Fluent](https://projectfluent.org/) syntax. Code names the key, never the words:
`menu_button("menu-new-game")`, `Localized("pause-title")` (it brings its own `Text`),
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
