# `create-bevy-starter` 0.4.0: the cameras, the split screen and multiplayer

Status: **built** 2026-11-24 in the CLI's repository; `just check-combos` result below, in "What
changed while building it".
Date: 2026-11-24
Template: 1.1.0
Roadmap: row 21's CLI part, ahead of the release it belongs to: the user's call, since rows 10 to
12, 14, 16, 19 and 20 are not built, so the template goes out as 1.1.0 and the CLI as 0.4.0, not
2.0. The rules it follows are `docs/specs/create-bevy-starter/design.md`'s "Camera markers".

## Goal

Every option the template now has can be picked, and what is not picked does not come: a game
with the third-person camera alone has no first-person file, no F5, and builds.

## Decided

| Question | Decision |
| --- | --- |
| Cameras | One question of a 3D game, ticked with Space: First person, Third person, Isometric (coming soon, asked again if ticked). None ticked is `3d/base`; one or both is `3d/player`, which has every camera |
| Split screen | Asked of a 3D game with a camera, No by default |
| Multiplayer | Asked of every kind, No by default: `main` has it, so every branch does. Without a player it still shares the world and the props |
| Removing | `Removal { marker, files }`, one per option: `MULTIPLAYER`, `FIRST_PERSON`, `THIRD_PERSON`, `CAMERA_SWITCH` (fewer than two cameras), `SPLIT_SCREEN`. The CLI deletes the files that are there (`others.rs` is in the player branches only) and every line of `Cargo.toml` and the Rust sources ending `// <marker>` or `# <marker>`. A removal none of whose files is there is `TemplateChanged`: the template moved it |
| After removing | `cargo fmt`, since a call whose marked argument went can now fit one line or end without its comma, and `cargo metadata`, which drops from `Cargo.lock` the crates only the removal needed and updates nothing, so the game's `just check` passes and its first build leaves the first commit clean. Best effort: the game builds either way |
| The locales | Multiplayer's keys stay in a game without it: Fluent has no comment at the end of a line to mark them by, and a key no code names costs nothing |
| `just check-combos` | An ignored test: makes the 20 games the CLI can make (Empty, 3D alone, 2D alone and with the player, each with and without multiplayer; and every camera set with and without the split screen, each with and without multiplayer), from the template on GitHub, and runs each one's `just check`, with one target folder for all |

## Out of scope

The docs a game keeps (`README.md`, `docs/flow.md`, `docs/template.md`) still describe the
options it left out, as they still name template branches.

## What changed while building it

- **The game is formatted again after the removals** (found by the first `just check-combos`):
  without multiplayer, `cargo fmt --check` failed in three files, a signature that now fits one
  line and two calls whose last argument had gone. It compiled; only the format differed.
