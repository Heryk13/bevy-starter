# bevy-starter

A starter template for building games with [Rust](https://www.rust-lang.org/) and [Bevy](https://bevy.org/) 0.19.

## Features

- Splash screen with the Bevy logo, then every logo in `assets/splash/logos/`, each one skippable
- Main menu with background art, buttons and looping music that fades with the screen
- Credits and Settings overlays over the running screen: fullscreen toggle and volume slider
- Loading screen with a progress bar over the real asset load and a shuffled slideshow
- Intro that plays a frame sequence with its soundtrack, skippable with keyboard or mouse
- Pause menu on Esc that freezes game time: Resume, Settings, Quit to Menu, Quit to Desktop
- Screen flow driven by Bevy states
- Fast dev builds: dynamic linking and optimized dependencies
- FPS overlay and asset hot reloading in dev builds
- Fast linking on Windows with `rust-lld`
- Release packaging into a ready-to-ship `dist/` folder
- Third-party license notices generated on every release
- No console window on Windows release builds

## Requirements

- Rust (latest stable) via [rustup](https://rustup.rs/)
- [just](https://github.com/casey/just)
- [cargo-about](https://github.com/EmbarkStudios/cargo-about), used by `just release`:
  `cargo install cargo-about --locked --features cli`
- **Windows:** `sh` on `PATH` (e.g. [Git for Windows](https://git-scm.com/)), used by `just` recipes
- **Linux:** Bevy's [system dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md)

## Usage

| Command        | Description                                            |
| -------------- | ------------------------------------------------------ |
| `just run`     | Run the game in dev mode                               |
| `just fix`     | Apply clippy fixes and format the code                 |
| `just check`   | Check formatting and lints, and run the tests          |
| `just release` | Build an optimized release and package it into `dist/` |

- Turning this template into your own game: [docs/template.md](docs/template.md)
- Development loop and code conventions: [CONTRIBUTING.md](CONTRIBUTING.md)

## Cargo features

| Feature | Default | Description                                                                          |
| ------- | ------- | ------------------------------------------------------------------------------------ |
| `dev`   | yes     | Dynamic linking, FPS overlay, asset hot reloading and a console window on Windows    |

Release builds must disable it with `--no-default-features`, which `just release` already does.
A binary built with `dev` depends on the Bevy dynamic library and won't run on other machines.

## Releasing

```sh
just release
```

The `dist/` folder contains the executable, the `assets/` folder next to it
and `licenses.html` with the license notices of every dependency.
Zip it and ship it. The executable looks for `assets/` in its own directory.

The release fails if a dependency uses a license not listed in `about.toml`.
Review that license before adding it to the list.

Builds target the current OS only: build on each platform (Windows, Linux, macOS) to ship for it.
