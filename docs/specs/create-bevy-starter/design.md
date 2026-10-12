# `create-bevy-starter`: a CLI that builds a game from the template

Status: **draft** — written 2026-10-12, waiting for review. Nothing built.

## Goal

Starting a game from this template means cloning the right branch and then renaming the game by
hand in three files, trimming Bevy's features and clearing out what only the template needs
(`docs/template.md`, "Starting a new game"). `create-bevy-starter` does all of it from a few
questions, the way `create-next-app` does for Next.js:

```
create-bevy-starter [name] [folder]

? Project name: (bevy-template)
? Where to create it: (.)
? Game title: (Bevy Template)
? Kind:
  › Empty
    3D
    2D (coming soon)
? Cameras (space to mark, enter to confirm; none = the level only, no player):
  ◯ First-person
  ◯ Third-person
  ◯ Isometric
? Create bevy-template in ./bevy-template? (Y/n)
```

The cameras can be combined: any one, any two, or all three, switched with F5 when more than
one is chosen.

It is built in two phases. **Phase A**, this spec: a tool for the template's author, run from
its own repository, never published. **Phase B**, later and in its own spec: anyone can install
it (`cargo install`, crates.io). Nothing in phase A should have to be undone for phase B.

## Decided

- **Its own program, in Rust**, so the template keeps compiling on every branch, 2D can be
  shown as "coming soon", and the menus take the arrow keys. The two ways turned down fail at
  exactly these:
  - `cargo-generate` (and `bevy new`, which runs it) would need `{% if %}` tags inside the
    `.rs` files to include or leave out a camera, and a branch full of them no longer compiles
    on its own, so its `just check` would fail. It also cannot show a choice that cannot be
    picked yet, nor pick a branch from an answer.
  - A script (`just` or PowerShell) has no arrow-key menus, does not run the same on every OS,
    and would be thrown away in phase B.
- **Its own repository, `create-bevy-starter`.** The CLI builds games, it is not part of one:
  in `main` it would ship inside every game it creates and travel down every merge, and as a
  branch it would start from nothing, against the branch tree's rule that a branch only adds to
  the one it starts from. The name follows `create-next-app` and `create-vite`, and names the
  template it builds from. It stays clear of `bevy new`, the community `bevy_cli`'s command.
- **In English**: the questions, the summary and every error. Phase B is for anyone, and
  translating later would rewrite every message and every test that checks one.
- **The cameras are plugins in one branch, not branches.** Git cannot add up sibling branches:
  merging them at creation time conflicts in the files they all touch (the player's spawn, the
  camera, the input), and a branch per combination is seven branches to keep in step. So the
  three cameras live side by side in `3d/player`, and the CLI deletes the ones not chosen.
- **What is not chosen does not come.** Empty is `main`, 3D with no camera is `3d/base`, and
  the F5 switch comes only with two cameras or more: a switch with one camera is noise.
- **Nothing is written before the confirmation.** Every answer becomes one `Choices` value
  first; Ctrl+C or "n" leaves the disk as it was.
- **All or nothing.** Once it starts writing, any failure deletes the folder it created. It
  only ever writes into a folder that did not exist or was empty, so nothing of the user's is
  in it.

## Out of scope

- Phase B: publishing, `cargo install`, checking the name is free on crates.io.
- Questions for 2D: the `2d` branch does not exist yet. The kind is shown, marked "coming
  soon", and cannot be picked. Its questions come when it splits into variants, as 3D did.
- Anything else to choose (keeping the save system, the languages): no one has asked for it.
- Building the cameras. Each has its own spec on `3d/player`; this spec only sets the rule they
  follow so the CLI can remove them.

## The branch tree

```
main ──► 3d/base ──► 3d/player
```

- `3d/first-person` is renamed `3d/player`: physics, the player's body, and the three
  cameras. Its first-person spec becomes the spec of the first camera.
- `3d/third-person` and `3d/combined` are no longer planned as branches: they are the CLI's
  combinations.

| Answers                | Branch cloned |
| ---------------------- | ------------- |
| Empty                  | `main`        |
| 3D, no camera          | `3d/base`     |
| 3D, one camera or more | `3d/player`   |

## What the template promises the CLI

The CLI knows where some things are in the template. These stay where they are, and moving one
means changing the CLI in the same change; `CONTRIBUTING.md` says so.

### The three names

| File          | What the CLI sets                               |
| ------------- | ----------------------------------------------- |
| `Cargo.toml`  | `name`, and Bevy's features for the kind chosen |
| `justfile`    | `game`                                          |
| `src/main.rs` | `GAME_TITLE`                                    |

The features for 3D are the set `docs/template.md` already checked to build:
`["3d", "ui", "audio", "jpeg", "wav"]`. Empty keeps the template's own line.

### Camera markers

Deleting `first_person.rs` is not enough: `mod first_person;` and the line that adds its plugin
still name it, and the game no longer compiles. So every line outside a camera's file that
names it ends with a marker, and the CLI deletes the file and every line carrying its marker:

```rust
mod first_person; // camera: first-person
```

| Piece                                 | Marker                    | Removed when               |
| ------------------------------------- | ------------------------- | -------------------------- |
| `first_person.rs` and its lines       | `// camera: first-person` | first-person is not chosen |
| `third_person.rs` and its lines       | `// camera: third-person` | third-person is not chosen |
| `isometric.rs` and its lines          | `// camera: isometric`    | isometric is not chosen    |
| `camera_switch.rs` (F5) and its lines | `// camera: switch`       | fewer than two are chosen  |

The F5 file names each camera on a marked line, so a camera removed leaves the switch too, and
no list of cameras has to exist anywhere else. The rule for `3d/player`: **each camera is one
file, and nothing outside it names it except marked lines.**

Markers were picked over the two other ways:

- searching for the text `mod first_person;`: breaks the day the line is reformatted;
- a Cargo feature per camera: the code of the cameras not chosen would stay in the game, off.

### What only the template needs

| Removed                                              | Why                                                         |
| ---------------------------------------------------- | ----------------------------------------------------------- |
| what is inside `docs/specs/`                         | the template's history; the game writes its own specs there |
| the `## Branches` section of `README.md`             | a game has no branch tree                                   |
| the line of `CLAUDE.md` naming the 1.0 refactor plan | it points at a spec that was just removed                   |

`docs/template.md` stays: besides the steps the CLI now does, it holds the project structure,
the assets and their licenses, and `README.md` and `docs/flow.md` link to it.

## How the CLI works

```
arguments ─► questions ─► Choices ─► summary + confirm ─► build on disk
```

| File          | What it does                                                                                                                       | Writes to disk       |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------- |
| `main.rs`     | reads the arguments, asks, shows the summary, builds                                                                               | no                   |
| `prompts.rs`  | the questions, skipping those the arguments answered                                                                               | no                   |
| `choices.rs`  | `Choices`, and the pure functions: which branch, what to delete, the title from the name, whether a name is valid, every file edit | no                   |
| `scaffold.rs` | clones, deletes, edits, `git init` and the first commit                                                                            | **yes**, and only it |

The file edits are pure too: `set_package_name(text, name)` takes the `Cargo.toml` text and
gives it back, so a test checks the name changed and nothing else did, without a folder.
`scaffold.rs` only reads, calls and writes.

The repository follows this project's `CONTRIBUTING.md`: file layout, separators, comments,
naming, tests, commits. Its `just check` runs the same lints.

### Defaults

| Question | Enter gives                                                          |
| -------- | -------------------------------------------------------------------- |
| Name     | `bevy-template`                                                      |
| Folder   | `.`, where the command runs; the project goes into `<folder>/<name>` |
| Title    | from the name: `bevy-template` becomes `Bevy Template`               |
| Kind     | Empty                                                                |
| Cameras  | none, which means `3d/base`                                          |

### Steps on disk

1. `git clone --depth 1 --branch <branch>` from GitHub into `<folder>/<name>`.
2. Delete its `.git`: the game starts its own history, not the template's.
3. Delete the cameras not chosen and their marked lines, and the switch with fewer than two.
4. Set the three names, and the features.
5. Remove what only the template needs.
6. `git init`, and a first commit.

### Libraries

| Need                             | Choice                                             | Why                                                                                                                              |
| -------------------------------- | -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `[name] [folder]`                | `std::env::args`                                   | two optional positional arguments need no crate                                                                                  |
| arrow-key menus and multi-select | `dialoguer` or `inquire`                           | the standard library cannot read arrow keys; pick, when building step 5, the one that shows an option that cannot be picked best |
| clone, init, commit              | the `git` command, through `std::process::Command` | anyone making a game from a template has git; `git2` is heavy for three commands                                                 |

## Errors

Every message is in English and says what to do next.

| Situation                                       | What happens                                                                                                 |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `git` is not installed                          | said first, before any question is asked                                                                     |
| the folder exists and is not empty              | stop before writing anything; never overwrite                                                                |
| invalid name, in a question                     | explain, ask again                                                                                           |
| invalid name, as an argument                    | explain, exit                                                                                                |
| Ctrl+C during the questions                     | exit; nothing was written                                                                                    |
| the clone fails (no network, no such branch)    | show git's error, delete the folder it created                                                               |
| any later step fails                            | delete the folder it created                                                                                 |
| a camera's file or marker is not where expected | "the template changed and the CLI did not follow", delete the folder: a broken game made in silence is worse |

A valid name is what Cargo accepts as a package name: ASCII letters, digits, `-` and `_`, not
starting with a digit.

## Tests

| What                                                                                              | Where               | How                                                                                                 |
| ------------------------------------------------------------------------------------------------- | ------------------- | --------------------------------------------------------------------------------------------------- |
| the branch for every answer                                                                       | `choices.rs`        | pure functions                                                                                      |
| the title from the name, whether a name is valid                                                  | `choices.rs`        | pure functions                                                                                      |
| what is deleted, including the switch below two cameras                                           | `choices.rs`        | pure functions                                                                                      |
| each edit: `name`, `game`, `GAME_TITLE`, marked lines, the Branches section, the `CLAUDE.md` line | `choices.rs`        | sample text in, expected text out, the rest unchanged                                               |
| **every combination compiles**                                                                    | `just check-combos` | builds all nine (Empty, 3D with no camera, and the seven of cameras) and runs `cargo check` in each |

`just check-combos` stays out of `just check`: it takes minutes and needs the network.

## Implementation order

In this repository, `bevy-starter`:

1. **Docs for the new tree**: the README's diagram with `3d/player`, `CONTRIBUTING.md` (no more
   third-person from first-person; the camera markers; the three names that stay put), and
   `docs/template.md`, which names `3d/first-person`.
2. **Rename `3d/first-person` to `3d/player`**, locally and on GitHub. A git operation, not a
   commit, done only once confirmed.

In the new repository, `create-bevy-starter`:

3. The skeleton: `Cargo.toml`, a `justfile` with `just check`, this project's lints.
4. `choices.rs`: `Choices` and the pure functions, with their tests.
5. `prompts.rs`: the questions.
6. `scaffold.rs`: clone, edit, remove what only the template needs, `git init`. From here it
   already makes games from `main` and `3d/base`.
7. Removing cameras by marker, tested on sample text. It is used for real once the first camera
   lands in `3d/player`; `just check-combos` follows once there are two.

## Answered questions

- _Does `bevy new` do this already?_ It runs `cargo-generate` on one repository per template
  (`-t 2d` is `bevy_new_2d`), asks that template's questions, and cannot combine options. It is
  an alpha (`0.1.0-alpha.2`). It is the `cargo-generate` option, with a shortcut to the
  official templates.
- _Is plain `main` a choice?_ Yes: Empty, for a game that really starts from nothing.
- _Should 3D require a camera?_ No: none gives `3d/base`, the level with a still camera, for a
  game whose player is none of the three (a car, a ship).
- _Is 2D shown before it exists?_ Yes, marked "coming soon", and it cannot be picked.

## What changed while building it

Nothing yet.
