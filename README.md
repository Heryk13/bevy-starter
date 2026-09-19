# bevy-starter

A starter template for building games with [Rust](https://www.rust-lang.org/) and [Bevy](https://bevy.org/) 0.19.

## Features

- Splash screen with fade in/out, skippable with keyboard or mouse
- Screen flow driven by Bevy states
- Fast dev builds: dynamic linking and optimized dependencies
- FPS overlay in dev builds
- Fast linking on Windows with `rust-lld`
- Release packaging into a ready-to-ship `dist/` folder
- No console window on Windows release builds

## Requirements

- Rust (latest stable) via [rustup](https://rustup.rs/)
- [just](https://github.com/casey/just)
- **Windows:** `sh` on `PATH` (e.g. [Git for Windows](https://git-scm.com/)), used by `just` recipes
- **Linux:** Bevy's [system dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md)

## Usage

| Command        | Description                                            |
| -------------- | ------------------------------------------------------ |
| `just run`     | Run the game in dev mode                               |
| `just fix`     | Apply clippy fixes and format the code                 |
| `just release` | Build an optimized release and package it into `dist/` |

## Project structure

```
assets/             game assets, loaded at runtime
src/
  main.rs           app entry point
  screens.rs        Screen state and screen plugins
  screens/
    splash.rs       splash screen
.cargo/config.toml  linker configuration
justfile            project commands
```

## Cargo features

| Feature | Default | Description                                                     |
| ------- | ------- | --------------------------------------------------------------- |
| `dev`   | yes     | Dynamic linking, FPS overlay and a console window on Windows    |

Release builds must disable it with `--no-default-features`, which `just release` already does.
A binary built with `dev` depends on the Bevy dynamic library and won't run on other machines.

## Releasing

```sh
just release
```

The `dist/` folder contains the executable and the `assets/` folder next to it.
Zip it and ship it. The executable looks for `assets/` in its own directory.

Builds target the current OS only: build on each platform (Windows, Linux, macOS) to ship for it.

## Assets

`assets/splash/bevy_logo_dark.png` is the Bevy logo, used as a placeholder.
It has no formal license; unmodified use in splash screens was
[informally allowed](https://github.com/bevyengine/bevy/discussions/10861) by the Bevy project lead.
Replace it with your own logo before shipping a game.
