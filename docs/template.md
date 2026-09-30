# Turning the template into a game

What to change once, at the start, and where things live afterwards.
For setup and the day-to-day commands, see the [README](../README.md).

## Starting a new game

1. Rename the game in three places, keeping them in sync:
   - `name` in `Cargo.toml`, which also names the game's data folder (`%APPDATA%\<name>\` on
     Windows, `~/.local/share/<name>/` on Linux, `~/Library/Application Support/<name>/` on
     macOS), where `settings.ron` lives
   - `game` in `justfile`, which `just release` uses to find the executable
   - the window `title` in `src/main.rs`
2. Trim Bevy's default features in `Cargo.toml` to what the game uses, for smaller and faster builds.
   For a 2D game:
   ```toml
   bevy = { version = "0.19.1", default-features = false, features = ["2d", "ui", "audio"] }
   ```
3. Keep the Bevy logo on the splash screen: it lets players know the game is made with Bevy
   and helps the engine grow. Use it unmodified (see [Assets](#assets)).
4. Replace the [placeholder assets](#placeholders) with the game's own.
   When the intro is re-exported, set `INTRO_FPS` in `src/screens/intro.rs` so the frames
   last as long as the new soundtrack, otherwise picture and sound end apart.

## Project structure

```
assets/             game assets, loaded at runtime
  intro/            the intro's soundtrack
    frames/         the intro's frames, played in file name order
  loading/          the loading screen's slideshow pictures
  menu/             the menu's background and music
  splash/           the Bevy logo
    logos/          the game's own logos, shown after the Bevy one
  ui/               the button sounds
src/
  main.rs           app entry point
  assets.rs         reading a loaded folder's images, shared by the screens
  fade.rs           the fade timing every screen shares
  overlays.rs       Overlay state and overlay plugins
  overlays/
    credits.rs      credits panel
    saves.rs        the list of saved games: new save, or overwrite one
    settings.rs     fullscreen toggle and volume slider, remembered in settings.ron
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
  save.rs           the game in progress, save files, the data folder and safe file writes
  ui.rs             the button and dialog widgets and the button sounds, shared
docs/
  template.md       this file: turning the template into a game
  flow.md           how the screens, overlays and fades fit together
  specs/            one folder per implementation, with its design and plan
.cargo/config.toml  linker configuration
about.toml          accepted dependency licenses
about.hbs           template for the license notices
justfile            project commands
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
