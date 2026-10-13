# Contributing

How every change in this project is made. Follow the loop for each feature, screen or fix.

Every rule below exists for a reason. When the rule and its reason disagree, the reason
wins: do what makes the code better, and write the exception down in a comment where it
lives, saying why the usual rule does not fit there. An exception with its reason beside
it is a decision. An exception without one is drift.

## The loop

1. **Plan**: find where the change belongs. Code is organized by feature, not by who wrote it:
   each screen or feature is a plugin in its own file (e.g. `src/screens/menu.rs`).
   A change bigger than one commit is designed first in `docs/specs/<feature>/design.md`:
   the decisions with their reasons, then an implementation order of one commit per step.
   What the build changes from the plan goes in its "What changed while building it".
2. **Write** following the [code conventions](#code-conventions).
3. **Test** the logic (see [Tests](#tests)).
4. **Fix and check**: `just fix`, then `just check`.
5. **Run**: `just run` and try the change in the game.
6. **Docs**: update `README.md` if features, commands or requirements changed,
   `docs/flow.md` if a screen, an overlay or the way the player moves between them changed,
   `docs/template.md` if what a game made from the template changes or where it lives moved,
   and the rest of `docs/` if the project structure or the way a feature works changed.
   A built spec is history: only its status line changes after it lands.
7. **Commit** (see [Commits](#commits)).

## Code conventions

### File layout

Every `.rs` file follows the same order, skipping the sections it doesn't have:

1. imports: external crates first, a blank line, then crate-local ones (`use super::...`)
2. child modules (`mod ...;`)
3. `Config`: tunable constants
4. `Plugin`: the `plugin` function, a summary of what the file does
5. in a shared file, what it hands to other files, in a section named after it
   (`Widgets` in `ui.rs`), not under `Helpers`
6. types: `State`, `Events`, `Components`, `Resources`
7. `Systems`: in the order they run (spawn, update, ...)
8. `Helpers`: pure functions used by the systems, next to the tests that cover them
9. `Tests`

### Splitting files

- Split by feature, never by kind of code: `player.rs` and `enemy.rs`,
  not `components.rs` and `systems.rs`.
- Code that several features share is expected, and it gets its own file at the top of
  `src/`, named after its domain: `ui.rs`, `audio.rs`, `save.rs`. Being at the top of
  `src/` is already what marks it as shared, so it needs no `common/` or `shared/`
  folder around it. Names like `common`, `utils` or `global` describe status instead of
  content: everything fits in them and nothing ever leaves. A domain name still answers
  "does this belong here?", so the file can be kept honest — and split the usual way
  when it grows (`ui.rs` next to `ui/`).
- Move something there once a third feature needs it. With one user it stays inside that
  feature; with two, it stays with the first and waits.
- Keep a feature in one file while it reads as one thing.
  Split when a part makes sense on its own, usually around 300 to 400 lines
  or when you find yourself scrolling to look for "the part about X".
  That part becomes its own file.
- A split module follows the `screens.rs` pattern: `player.rs` next to a `player/` folder
  with the parts (`player/movement.rs`), never `mod.rs`; clippy's `mod_module_files` fails
  `just check` on one.
- Unit tests stay in the file they test, `tests/` is only for integration tests.

### Separators

Every `.rs` file splits its sections with separator lines ending at column 80.
The character tells the kind of boundary:

```rust
// ==== Systems ================================================================
// ---- logo animation: wait, fade in, hold, fade out --------------------------
```

- `====`: a section of the file. Title Case.
- `----`: a group inside a section, lowercase, may carry a short note.
  Inside a function body it is indented with the code and still ends at column 80.

### Comments

- Comments explain *why*, not *what*. Code that already says what it does gets no comment.
- Every system and type has a `///` doc comment written as a sentence:
  capitalized, ending with a period (`/// Spawns the splash screen.`).
- Plain `//` comments are lowercase, without a final period.

### Bevy patterns

- One plugin per screen or feature: `pub(super) fn plugin(app: &mut App)`.
- Screen entities get `DespawnOnExit(Screen::X)`, screen systems get `.run_if(in_state(Screen::X))`.
  The same goes for `Overlay::X` and `Pause::On`: whatever a state spawns, its exit despawns.
- An observer gets the same `run_if` as the state its buttons live in, so it never runs, nor
  asks for that state's resources, anywhere else. Only observers whose buttons live
  everywhere (`close_overlay`, `close_dialog`) stay global.
- `Screen` is where the player is, `Overlay` what is drawn on top, `Pause` whether the game is
  frozen. Anything drawn over a live screen is an overlay, never a screen, so the screen
  underneath is never torn down (see `docs/flow.md`).
- A button carries a component naming what it does (`MenuAction`, `CloseOverlay`), and one
  observer per file reads it off `activate.entity`.
- Anything that must keep moving while the game is paused (the UI, the blur) counts
  `Time<Real>`. Gameplay counts `Res<Time>`, which the pause menu freezes for free.
- Floating UI (overlays, the pause menu, dialogs) is built from `floating_root()` and `card()`
  in `ui.rs`; colors and type sizes come from its `Config`, never from a literal.
- Roots that must stack carry a named `GlobalZIndex` constant (`*_Z_INDEX`), listed in
  `docs/flow.md`.
- Files on disk live in the game's data folder and are written with `write_atomic`, so a
  crash mid-write leaves the old file whole.
- State that belongs to an entity is a component on it, not a global resource.
- Exactly one entity: `Single<...>`. Zero or more: `Query<...>`.
- Ask for the minimum access: `&` to read, `&mut` only to write.
- When a system reads what another writes, make the order explicit (`.after(...)` or `.chain()`).
- Tunable values are named constants in the `Config` section, never magic numbers.
  A value that follows from other constants is derived from them in its own group, never
  restated: two spellings of the same number drift apart the day one of them changes.
- A set of assets that can grow lives in a folder read with `load_folder`, not in a list of
  file names. Order that matters comes from sorting by name, so those names stay zero-padded.
- Prefer the call that can say no over the one that panics: `try_typed` over `typed`,
  `first()` and `get()` over `[0]`, `saturating_sub` where a `usize` could go below zero.
  A missing asset should cost a frame of the animation, never the whole game.
- Dev-only code goes behind `#[cfg(feature = "dev")]`.

### Text

- Every word the player reads is a key in every `assets/locales/<code>/main.ftl`. Code names
  the key, never the words: `label("menu-new-game")`, `Localized("pause-title")`, and
  `Translations::text_with` for text built from values. A test fails while one file lacks a
  key another has, or uses a variable (`{ $count }`) the English message does not pass.
- A key belongs to one place in the UI, even when its English repeats another's
  (`menu-settings`, `pause-settings`): another language may need two different words.
- Proper nouns are never translated: the game's title, people's names, each language's own
  name in the language picker.
- Casing copies the English on buttons, titles and labels (`Tela Cheia`, `Sem Nome`), and
  questions and status lines are written as sentences (`Sair sem salvar?`). It is the rule of
  Microsoft's pt-BR style guide, and it decides every new key without a judgment call.

### Naming

- Systems and observers: verb + noun (`spawn_splash_screen`, `advance_splash_logos`).
- Constants: `SCREAMING_SNAKE_CASE`, with the unit in the name (`LOGO_HOLD_SECS`).

## Tests

- Logic lives in pure functions: plain values in, plain values out.
  Systems stay thin: read from the ECS, call the function, write the result.
- Tests sit at the bottom of the same file, in `#[cfg(test)] mod tests` under `// ==== Tests ====`.
- Compare floats with a small tolerance instead of `assert_eq!`.
- Every test asserts something. A test with an empty body passes and proves nothing, which is
  worse than no test: it reports green while the code it names is unprotected.
- The name says what the test guarantees, as a sentence about the subject
  (`logo_is_invisible_at_the_start`), not what the function under test is called.
- Write the times and sizes a test uses as expressions over the constants
  (`SLIDE_SECS / 2.0`), so the test follows the code when a constant changes, and pick them
  away from the boundary between two phases, where `f32` rounding decides the answer.
- A test that touches the disk starts with `fresh_test_dir("bevy-starter-list-saves")` from
  `disk.rs`: a folder of its own under `std::env::temp_dir()`, emptied first, so tests running
  in parallel never share files and no earlier run leaves any behind. It never touches the
  real data folder, where the player's saves are.

## Commits

- [Conventional Commits](https://www.conventionalcommits.org/), in English, imperative, short title:
  `feat: add main menu`, `fix: ...`, `refactor: ...`, `test: ...`, `style: ...`, `docs: ...`, `build: ...`.
- One change per commit.
- `git commit --amend` only before pushing.

## Branches

- The branches are a tree. `main` is the template for any game, a genre's base branch
  (`3d/base`) is cut from `main`, and every other branch is cut from the one it builds on:
  `3d/player` from `3d/base`. The README lists them.
- A change goes into the highest branch that every user of it lives under, and nowhere else:
  something every game needs into `main`, something every 3D game needs into `3d/base`.
  Written once there, it reaches the branches below by merging down, one level at a time:
  `main` into `3d/base`, `3d/base` into `3d/player`, and so on down.
- A genre's base stays as small as the genre allows: `3d/base` is the level on screen
  and nothing that moves, so no branch below it has to take apart a body or a camera it
  does not want.
- A branch only adds to the one it starts from. Anything it changes in the shared code is a
  change its parent is missing, so it goes to the parent first and arrives by the merge.
- A branch's specs are about that branch: `3d/base` designs only what every 3D game shares,
  so a branch cut from it never inherits plans that are not its own.
- Options a game combines freely, like the cameras, are not branches: git cannot add up two
  sibling branches, since both change the same files. They will live side by side in one
  branch, and `create-bevy-starter` will delete the ones a game does not keep once the first
  camera exists (see `docs/specs/create-bevy-starter/design.md`). For it to find them, each
  option will be one file, and every line outside that file naming it will end with its marker
  (`mod first_person; // camera: first-person`).
- `create-bevy-starter` renames the game where `docs/template.md` says to: `name` in
  `Cargo.toml`, `game` in `justfile`, `GAME_TITLE` in `src/main.rs`. It also removes the
  `## Branches` section of `README.md` and empties `docs/specs/`. Moving one of these, or
  renaming that heading, means changing the CLI in the same change.
- Git cannot hold a branch `3d` next to `3d/base`, since a branch name is a file and a
  `/` makes it a folder: a genre's base is always `<genre>/base`.
- Models stay plain binary files in `.gitattributes`, not Git LFS: this repository has LFS
  disabled on GitHub, which refuses the push.

## Dependencies

- New crates are fine when they save real work, but check first if Bevy or the standard library already does it.
  In a template every crate is friction for whoever clones it: each one has to move to the
  next Bevy release before the template can.
- `just release` checks every dependency license against `about.toml`.
  Review a new license before adding it to the list.
