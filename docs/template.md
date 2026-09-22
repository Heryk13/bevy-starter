# Turning the template into a game

What to change once, at the start, and where things live afterwards.
For setup and the day-to-day commands, see the [README](../README.md).

## Starting a new game

1. Rename the game in three places, keeping them in sync:
   - `name` in `Cargo.toml`
   - `game` in `justfile`, which `just release` uses to find the executable
   - the window `title` in `src/main.rs`
2. Trim Bevy's default features in `Cargo.toml` to what the game uses, for smaller and faster builds.
   For a 2D game:
   ```toml
   bevy = { version = "0.19.1", default-features = false, features = ["2d", "ui", "audio"] }
   ```
3. Keep the Bevy logo on the splash screen: it lets players know the game is made with Bevy
   and helps the engine grow. Use it unmodified (see [Assets](#assets)).

## Project structure

```
assets/             game assets, loaded at runtime
src/
  main.rs           app entry point
  screens.rs        Screen state and screen plugins
  screens/
    splash.rs       splash screen
docs/
  template.md       this file: turning the template into a game
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
