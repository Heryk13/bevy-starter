# `create-bevy-starter`: a CLI that builds a game from the template

Status: **in progress** — written and approved 2026-10-12. Steps 1 to 10 built. Publishing
brought into scope and the cameras moved out of it on 2026-10-13 (see "What changed while
building it").

## Goal

Starting a game from this template means cloning the right branch and then renaming the game by
hand in three files and clearing out what only the template needs (`docs/template.md`,
"Starting a new game"). `3d/base` already has Bevy's features trimmed, and
`create-bevy-starter` does the rest from a few questions, the way `create-next-app` does for
Next.js:

```
create-bevy-starter [name] [folder]

? Project name: (bevy-template)
? Where to create it: (.)
? Game title: (Bevy Template)
? Kind:
  › Empty
    3D
    2D (coming soon)
? Multiplayer:
  › No
    Yes (coming soon)
? Create bevy-template in ./bevy-template? (Y/n)
```

The cameras are not asked yet: 3D is the level alone until `3d/player` has them. Then a question
for them comes after 3D, and they can be combined: any
one, any two, or all three, switched with F5 when more than one is chosen.

It is built in two phases, both in this spec. **Phase A**, steps 1 to 6: a tool for the
template's author, run from its own repository. **Phase B**, steps 7 to 11: anyone can install
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
- **In English**: the questions, the confirmation and every error. Phase B is for anyone, and
  translating later would rewrite every message and every test that checks one.
- **The cameras are plugins in one branch, not branches.** Git cannot add up sibling branches:
  merging them at creation time conflicts in the files they all touch (the player's spawn, the
  camera, the input), and a branch per combination is seven branches to keep in step. So when
  the cameras come, the three will live side by side in `3d/player`, and the CLI will delete
  the ones not chosen.
- **What is not chosen does not come.** Empty is `main`, 3D with no camera is `3d/base`, and
  the F5 switch comes only with two cameras or more: a switch with one camera is noise.
- **Nothing is written before the confirmation.** Every answer becomes one `Choices` value
  first; Ctrl+C or "n" leaves the disk as it was.
- **All or nothing.** Once it starts writing, any failure deletes the folder it created. It
  only ever writes into a folder that did not exist or was empty, so nothing of the user's is
  in it.

## Out of scope

- Choosing cameras, and removing them by marker. Until the first camera lands in `3d/player`,
  the CLI does not ask for them, and 3D is always `3d/base`. The question, the removal,
  tested on sample text, and `just check-combos` come with that camera, in a spec of their own
  that follows the rules in "Camera markers".
- Questions for 2D: the `2d` branch does not exist yet. The kind is shown, marked "coming
  soon", and cannot be picked. Its questions come when it splits into variants, as 3D did.
- Multiplayer: shown as the last question, its "Yes" marked "coming soon" and impossible to
  pick, like 2D. It combines with every kind and camera, so it cannot be a branch of its own,
  and it reaches into the player's spawn, the input, the save and the pause, so whether a
  marker can remove it is not known yet. Its own spec decides how it combines.
- Anything else to choose (keeping the save system, the languages): no one has asked for it.
- Building the cameras. Each has its own spec on `3d/player`; this spec only sets the rule they
  follow so the CLI can remove them.

## The branch tree

```
main ──► 3d/base ──► 3d/player
```

- `3d/first-person` is renamed `3d/player`, where physics, the player's body and the three
  cameras will go. Its first-person spec becomes the spec of the first camera.
- `3d/third-person` and `3d/combined` are no longer planned as branches: they are the CLI's
  combinations.

| Answers                                        | Branch cloned |
| ---------------------------------------------- | ------------- |
| Empty                                          | `main`        |
| 3D                                             | `3d/base`     |
| 3D, one camera or more (with the first camera) | `3d/player`   |

## What the template promises the CLI

The CLI knows where some things are in the template. These stay where they are, and moving one
means changing the CLI in the same change; `CONTRIBUTING.md` says so.

### The three names

| File          | What the CLI sets                               |
| ------------- | ----------------------------------------------- |
| `Cargo.toml`  | `name`                                          |
| `justfile`    | `game`                                          |
| `src/main.rs` | `GAME_TITLE`                                    |

Bevy's features are not the CLI's: each branch keeps its own. `3d/base` builds only the set
`docs/template.md` already checked, `["3d", "ui", "audio", "jpeg", "wav"]`, and `main` keeps
Bevy's defaults.

### Camera markers

These are the rules for when the cameras come; the CLI does not do this yet.

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

| Removed                                  | Why                                                         |
| ---------------------------------------- | ----------------------------------------------------------- |
| what is inside `docs/specs/`             | the template's history; the game writes its own specs there |
| the `## Branches` section of `README.md` | a game has no branch tree                                   |

The emptied `docs/specs/` keeps an empty `.gitkeep`, since git does not track an empty folder.

`docs/template.md` stays: besides the steps the CLI now does, it holds the project structure,
the assets and their licenses, and `README.md` and `docs/flow.md` link to it.

## How the CLI works

```
arguments ─► questions ─► Choices ─► confirm ("Create {name} in {path}?") ─► build on disk
```

| File          | What it does                                                                                                                            | Writes to disk       |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------- | -------------------- |
| `main.rs`     | reads the arguments, asks, builds                                                                                                       | no                   |
| `prompts.rs`  | the questions, skipping those the arguments answered, and the confirmation naming the game and its folder                               | no                   |
| `choices.rs`  | `Choices`, and the pure functions: which branch, where the game goes, the title from the name, whether a name is valid, every file edit | no                   |
| `scaffold.rs` | clones, deletes, edits, `git init` and the first commit                                                                                 | **yes**, and only it |

The file edits are pure too: `replace_line(text, prefix, new_line)` takes the `Cargo.toml` text
and gives it back, so a test checks the name changed and nothing else did, without a folder.
`scaffold.rs` only reads, calls and writes.

The repository follows this project's `CONTRIBUTING.md`: file layout, separators, comments,
naming, tests, commits. Its `just check` runs the same lints, less the ones only a Bevy game
needs.

### Defaults

| Question    | Enter gives                                                          |
| ----------- | -------------------------------------------------------------------- |
| Name        | `bevy-template`                                                      |
| Folder      | `.`, where the command runs; the project goes into `<folder>/<name>` |
| Title       | from the name: `bevy-template` becomes `Bevy Template`               |
| Kind        | Empty                                                                |
| Multiplayer | No                                                                   |

### Steps on disk

1. `git clone --depth 1 --branch <branch>` from GitHub into `<folder>/<name>`.
2. Delete its `.git`: the game starts its own history, not the template's.
3. Set the three names.
4. Remove what only the template needs.
5. `git init`, and a first commit.

### Libraries

| Need                | Choice                                             | Why                                                                                            |
| ------------------- | -------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `[name] [folder]`   | `std::env::args`                                   | two optional positional arguments need no crate                                                |
| arrow-key menus     | `inquire`                                          | the standard library cannot read arrow keys; why this one, in "What changed while building it" |
| clone, init, commit | the `git` command, through `std::process::Command` | anyone making a game from a template has git; `git2` is heavy for three commands               |

## Errors

Every message is in English and says what to do next.

| Situation                                                                                           | What happens                                                                                                 |
| --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `git` is not installed                                                                              | said first, before any question is asked                                                                     |
| the folder exists and is not empty                                                                  | stop before writing anything; never overwrite                                                                |
| invalid name, in a question                                                                         | explain, ask again                                                                                           |
| invalid name, as an argument                                                                        | explain, exit                                                                                                |
| Ctrl+C or Esc during the questions                                                                  | exit; nothing was written                                                                                    |
| the clone fails (no network, no such branch)                                                        | show git's error, delete the folder it created                                                               |
| any later step fails                                                                                | delete the folder it created                                                                                 |
| a line or heading the CLI edits is not where expected (`name`, `game`, `GAME_TITLE`, `## Branches`) | "the template changed and the CLI did not follow", delete the folder: a broken game made in silence is worse |

A valid name is what Cargo accepts as a package name: ASCII letters, digits, `-` and `_`, not
starting with a digit.

## Tests

| What                                               | Where               | How                                                                                                 |
| -------------------------------------------------- | ------------------- | --------------------------------------------------------------------------------------------------- |
| the branch for every answer                        | `choices.rs`        | pure functions                                                                                      |
| the title from the name, whether a name is valid   | `choices.rs`        | pure functions                                                                                      |
| the two edits, `replace_line` and `remove_section` | `choices.rs`        | sample text in, expected text out                                                                   |
| **every combination compiles** (with the cameras)  | `just check-combos` | builds all nine (Empty, 3D with no camera, and the seven of cameras) and runs `cargo check` in each |

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
7. **No camera question yet.** 3D goes straight to `3d/base`. `Camera`, the cameras in
   `Kind::ThreeD` and the question go, since a choice the CLI cannot build has no variant.
8. **Licenses**: `MIT OR Apache-2.0`, Rust's usual pair and Bevy's own, with `LICENSE-MIT` and
   `LICENSE-APACHE` in both repositories. In `bevy-starter` they cover the code; the assets keep
   the licenses `docs/template.md` lists.
9. **Ready for crates.io**: `Cargo.toml` gets `description`, `license`, `repository`, `readme`,
   `keywords` and `categories`, and loses `publish = false`; the README says how to install it.
   `cargo publish --dry-run` passes.
10. **`bevy-starter` public on GitHub**, so the CLI clones it without the author's credentials.
    A setting, not a commit, done only once confirmed.
11. **Publish**: `cargo publish`, done only once confirmed, then `cargo install
    create-bevy-starter` on a clean folder makes a game from `main` and one from `3d/base`.

## Answered questions

- _Does `bevy new` do this already?_ It runs `cargo-generate` on one repository per template
  (`-t 2d` is `bevy_new_2d`), asks that template's questions, and cannot combine options. It is
  an alpha (`0.1.0-alpha.2`). It is the `cargo-generate` option, with a shortcut to the
  official templates.
- _Is plain `main` a choice?_ Yes: Empty, for a game that really starts from nothing.
- _Should 3D require a camera?_ No: none gives `3d/base`, the level with a still camera, for a
  game whose player is none of the three (a car, a ship).
- _Is 2D shown before it exists?_ Yes, marked "coming soon", and it cannot be picked.
- _Is the name free on crates.io?_ Yes: `cargo search create-bevy-starter` found nothing on
  2026-10-13. Checked again right before step 11.

## What changed while building it

1. **Two generic edits, not one function per file.** The plan had `set_package_name(text, name)`
   for `Cargo.toml` and, by the same pattern, one for each name and for the README. Every one
   of them is the same work on a different line, so `choices.rs` has two instead:
   `replace_line(text, prefix, new_line)`, which sets `name` in `Cargo.toml`, `game` in the
   `justfile` and `GAME_TITLE` in `src/main.rs`, and `remove_section(text, heading)`, which
   takes the `## Branches` section out of the README. Which file, prefix and heading each edit
   uses is left to `scaffold.rs`. Both return `None` when the line or heading is missing, so a
   template that moved one of them stops the CLI instead of making a game half renamed.
2. **`inquire`, and the menus ask again.** The plan left `dialoguer` or `inquire` to step 5, to
   pick the one that shows an option that cannot be picked best. `inquire` has no such option,
   so 2D and multiplayer's Yes are plain lines marked "(coming soon)": picking one says so and
   asks the same question again. Its validator asks again in place for an invalid name, and the
   rule it shows is the same text the CLI prints for an invalid name given as an argument.
3. **Esc leaves like Ctrl+C.** The plan named only Ctrl+C. `inquire` reports Esc apart, and both
   mean the user wants out before anything was written, so both end in "Nothing was created."
4. **Publishing comes in, the cameras go out.** The plan left phase B to a later spec and made
   step 7 the cameras' removal by marker. That step cannot run until the first camera exists in
   `3d/player`, and it would have held back everything after it. So phase B is now steps 7 to
   11 of this spec, the cameras are shown as coming soon until they exist, and their removal
   moves to "Out of scope", to come with the first camera.
5. **Bevy's features live in `3d/base`, not in the CLI.** The plan had the CLI rewrite the
   `bevy` line for 3D. That line carries Bevy's version, so the CLI would have pinned a version
   the template had moved past, and nothing tested that the set it wrote still built. In
   `3d/base` itself, its own `just check` proves the set builds on every commit, and a clone by
   hand gets it too. The cost is one conflicting line when `main` moves to a new Bevy, merged
   into `3d/base`. `3d/base` got it in `build: build only the bevy features 3d uses`.
6. **What `scaffold.rs` settled.** Its failures are one enum, `ScaffoldError`, whose `Display`
   is the message the user reads; each variant came with the sub-step that could fail that way.
   The pure edits reach the disk through one `edit_file(file, edit)`, which reads, calls one
   edit from `choices.rs` and writes, and turns its `None` into "the template changed". The
   clone, the edits and the first commit run in `build_game`, and `create_game` deletes the
   folder when it fails. The first commit is `build: start from bevy-starter`, in the
   template's style.
7. **`just run` passes its arguments untouched.** Pasted into the shell's text, `C:\temp`
   reached the CLI as `C:temp`, a path relative to the current folder, and the game was cloned
   inside this repository. The recipe now takes positional arguments and passes `"$@"`.
8. **No camera question at all, not one marked coming soon.** Entry 4 had the cameras shown
   as coming soon, like 2D. They are left out instead: it takes the question, `Camera` and the
   cameras in `Kind::ThreeD` out of the code until there is a camera to pick, and they come
   back together, with the first camera.
9. **The licenses travel into every game.** The plan named the files, not where they end up.
   They sit in `main`, so `3d/base` and `3d/player` get them on the next merge, and every game
   the CLI makes keeps them: MIT asks that its notice stay with the code it covers. A game adds
   its own license for what it writes on top. The notice names `Heryk13`, the author's GitHub
   name.
10. **What the package holds, and the Rust it needs.** The plan listed the metadata only. The
    package also takes an `include` list, the sources, the README and the licenses, so the files
    for working on the CLI (`CLAUDE.md`, `CONTRIBUTING.md`, the `justfile`) stay out of what
    `cargo install` downloads. And `rust-version = "1.88"`, the first stable Rust with the let
    chains the CLI uses, so an older Rust says so instead of failing on the syntax. `repository`
    points at `create-bevy-starter`, which is private as well: step 10 has to cover it too, or
    the link on crates.io leads nowhere.
11. **Both repositories went public, under a work address.** The plan named `bevy-starter` only;
    `create-bevy-starter` went too, for entry 10's reason. Before that, every commit's author
    and committer address was rewritten to the author's work one, so no personal address went
    public with the history: new hashes on every branch and on `v1.0.0`, pushed with force while
    both were still private. A clone with no credentials now gets the template.
12. **The emptied `docs/specs/` keeps a `.gitkeep`.** Git does not track empty folders, so the
    first commit left it out and a clone of the game had no `docs/specs/`. The CLI now writes an
    empty `.gitkeep` there.
