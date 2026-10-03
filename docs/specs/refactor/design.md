# Refactor plan for the 1.0 template

Status: **built** — written 2026-10-02 after the 1.0 features landed (`87df405`), built
2026-10-03 one commit per step, tagged `v1.0.0`. What differs from the plan is recorded at the
bottom, under "What changed while building it"; everything above it is the plan as written.

## Goal

The template works and is tested. This plan makes it smaller, easier to read and easier to grow
from, without changing what the player sees, except for the one bug it fixes (step 1). It also
records what was checked and turned down, so nobody repeats the search.

What it is not: new features. The review also found gaps in the game's flow and UX (keyboard and
gamepad menus, autosave growth, the intro skip). Those change behaviour, so they are listed in
[Beyond the refactor](#beyond-the-refactor) as candidates for their own specs.

## How this was checked

- Every file read: the 19 `.rs` files (4,687 lines), the three `.ftl` files, the shader, the
  README, `docs/flow.md`, `docs/template.md` and the four specs.
- Baseline: `just check` passes, **114 tests**.
- **Every code change in steps 1 to 13 was made and run** in a throwaway `git worktree` (never
  in the working tree), sharing `target/` so it built incrementally, then thrown away. All of
  them together pass `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` in dev
  and release configs, and **121 tests**; the game launched from that copy ran with no panic,
  error or warning. What a launch cannot show is anything that needs a click: steps 3 to 6
  change a click's path, so run `just run` and try them. Step 14 is text, not checked by a
  build beyond the translation tests.
- Two theories were proven by making them fail first: today's code panics without `Session`
  (step 3), and a blank save name shows blank (step 1).
- After the plan was written, an independent review checked every claim in it against the code,
  the experiment's diff and the Bevy source. Its 14 corrections are in this text, and it is why
  steps 1, 7, 8 and 13 were then built and run too, rather than only described.
- The decisions were checked on the web (sources at the end) and against the local Bevy 0.19.1
  source, and the specs' recorded decisions were read before proposing to change any of them.

## Summary

| # | Step | Kind | Who | Size (approx.) |
| --- | --- | --- | --- | --- |
| 1 | A blank save name reads "Unnamed" | `fix` | user | 2 lines + 1 test |
| 2 | `write_atomic` makes its own folder | `refactor` | user | −4 lines |
| 3 | Scope observers with `run_if` | `refactor` | user | −8 lines + 1 test |
| 4 | `SaveFinished`: the pause menu owns its status line | `refactor` | user | −25 lines |
| 5 | One `Quit(QuitTo)` action in the pause menu | `refactor` | user | −15 lines |
| 6 | Esc as a tested pure function | `refactor` | user | +5 tests |
| 7 | One helper for "the saves on disk" | `refactor` | user | −6 lines |
| 8 | Small simplifications | `refactor` | user | −20 lines |
| 9 | A narrow public surface for `screens` | `refactor` | Claude (move) | ±0 |
| 10 | `disk.rs` out of `save.rs` | `refactor` | Claude (move) | ±0 |
| 11 | `ui.rs` next to `ui/`, `blur.rs` inside it | `refactor` | Claude (move) | ±0 |
| 12 | Lint test code and the "no `mod.rs`" rule | `build` | user | 2 lines + 9 test fixes |
| 13 | Tests: fresh folders, translation variables | `test` | user | +2 tests |
| 14 | Polish the Portuguese and Japanese text | `fix` | user | `.ftl` only |
| 15 | Docs pass | `docs` | Claude | docs only |

"Claude (move)" follows the standing rule: pure renames and moves are Claude's, logic is the
user's. Steps 1 to 8 change logic; 9 to 11 only move code.

## Architecture

### Verdict: keep the shape, tighten the seams

The question was whether to restructure around Clean Architecture, DDD or a modular monolith.
The answer is that the template **already is a modular monolith**, and the parts of those ideas
that fit an ECS game are already here or come in with steps 3, 4 and 9:

| Idea | What it means here | Where it already is |
| --- | --- | --- |
| Modular monolith: one binary, modules with clear boundaries | one plugin per feature, each with a small `pub(crate)` surface | `screens/`, `overlays/`, `ui.rs`, `save.rs`, `i18n.rs` |
| DDD bounded contexts | a module per domain, named after it | `save` (sessions and save files), `i18n` (language), `ui` (look) |
| DDD ubiquitous language | the same word in code, docs and UI | `Session`, `Screen`, `Overlay`, `Pause`, `StartGame` |
| DDD domain events | a module announces what happened, others react | `StartGame` today; `SaveFinished` in step 4 |
| Clean Architecture's core rule: logic independent of the framework | pure functions with tests, thin systems around them | CONTRIBUTING's Tests section |
| Dependency rule: shared code never depends on features | `ui`, `blur`, `save`, `i18n`, `fade`, `assets` import no screen or overlay | holds today |

What does **not** fit is the folder layout those styles are usually drawn with:
`domain/`, `application/`, `infrastructure/`, `presentation/`. In an ECS, a feature is one plugin
whose components, systems and pure logic belong together. Layer folders would scatter the menu
over four directories, which is exactly the "split by kind" that CONTRIBUTING forbids
(`components.rs`/`systems.rs`). Bevy's own guidance is "games themselves are also implemented as
plugins", and the official community template `bevy_new_2d` splits by feature too. The one
serious write-up on layering a Bevy game (Morgenthum, 2026) layers by *crate* once a project is
large (engine modules, game core, features, bootstrap), with two rules this plan adopts now:
dependencies only point downward, and features talk through public APIs, never each other's
internals. Splitting into crates is a later step, for when compile times ask for it.

### The dependency rule, written down

1. `main.rs` knows every top-level module and nothing knows `main.rs`, except for `GAME_TITLE`.
2. Shared modules (`ui` with `blur`, `save`, `disk`, `i18n`, `fade`, `assets`) never import
   `screens` or `overlays`.
3. Features reach another feature only through what its parent module re-exports
   (`crate::screens::{Screen, StartGame, LoadingDestination, Pause}`), never by a path into its
   files (`crate::screens::gameplay::pause::...`).

Today, rule 2 holds and rule 3 has three exceptions: `overlays.rs` and `overlays/saves.rs` reach
into `screens::gameplay::pause` (for `Pause` and `SaveStatus`) and into `screens::loading` (for
`LoadingDestination`), which is why those two modules are `pub(crate)`. Step 4 removes the
`SaveStatus` edge, step 9 the other two.

### The tree, before and after

```
src/                          src/
  main.rs                       main.rs
  assets.rs                     assets.rs
  blur.rs                       disk.rs        <- data folder + write_atomic (step 10)
  fade.rs                       fade.rs
  i18n.rs                       i18n.rs
  overlays.rs                   overlays.rs
  overlays/                     overlays/
    credits.rs                    credits.rs
    saves.rs                      saves.rs
    settings.rs                   settings.rs
  save.rs                       save.rs        <- sessions and save files only
  screens.rs                    screens.rs     <- + LoadingDestination, re-exports Pause
  screens/                      screens/
    splash.rs                     (unchanged)
    menu.rs, menu/veil.rs
    loading.rs
    intro.rs
    gameplay.rs, gameplay/pause.rs
  ui.rs                         ui.rs          <- palette, type, text widgets
                                ui/
                                  blur.rs      <- moved (step 11)
                                  button.rs    <- buttons, their paint and sounds
                                  panel.rs     <- floating_root, card, dialog, entrance
```

Read at a glance: `screens/` is where the player is, `overlays/` what is drawn on top, and every
file at the top of `src/` is a shared domain named after what it holds. No `common/`, `core/` or
`utils/`, for the reason CONTRIBUTING already gives.

## Bugs found

**A save name made of spaces is shown blank.** `save_new_game`'s doc comment and `docs/flow.md`
both say the name is the field "trimmed", but the code stores `field.value().to_string()` as is.
The `trim()` is in `overwrite_save` instead, on a name that came from a save already written.
Proven with a test before writing this: a save named `"   "` reads `"   "` in the list, not
"Unnamed". Fixed in step 1.

**`just check` never lints the tests.** Its clippy runs without `--all-targets`, so code under
`#[cfg(test)]` is not checked, and it already has 9 errors with the same lints the game code
passes (`field_reassign_with_default` in `save.rs`'s tests). Fixed in step 12.

Nothing else misbehaved in the review. The docs had drifted from the code in a handful of places,
fixed in the same pass that wrote this plan (see [Docs already fixed](#docs-already-fixed)).

## Steps

Each step: why, the facts needed to write it, what was proven, the tests, and a commit message.
Run `just fix` and `just check` after each, and the end-of-step review (names and order).

### 1. A blank save name reads "Unnamed"

**Why.** The bug above.

**Facts.**
- `EditableText::value()` returns parley's `SplitString`, not a `&str`, so there is no `.trim()`
  on it. The compiler says so: `no method named 'trim' found for struct 'SplitString'`. Convert
  first: `.to_string()`, then `.trim()`, then `.to_string()` again for the owned `String`.
- Trim in `save_new_game`, where the player's text comes in, and drop the `.trim()` in
  `overwrite_save`.
- Make `save_name` treat a whitespace-only name as no name (`save.name.trim().is_empty()`), so a
  save already written with spaces also reads "Unnamed".

**Proven.** Before: a name of `"   "` reads `"   "`. After: it reads "Unnamed", and the test
passes.

**Test.** `blank_name_is_called_unnamed` next to `save_without_a_name_is_called_unnamed`.

**Commit.** `fix: show a save named only with spaces as Unnamed`

### 2. `write_atomic` makes its own folder

**Why.** Both callers make the parent folder first: `write_save` with three lines,
`save_settings` with four, its own `error!` included. Every file written atomically needs its
folder, so the function that writes should make it.

**Facts.** Move the `if let Some(parent) = path.parent() { create_dir_all(parent)?; }` from
`write_save` to the top of `write_atomic`, and delete the `create_dir_all` block in
`save_settings`; its `write_atomic` error already logs.

**Proven.** All save and settings tests pass, including `save_is_written_into_a_missing_folder`.

**Commit.** `refactor: let write_atomic create the folder it writes into`

### 3. Scope observers with `run_if`

**Why.** Every observer here is global: each `Activate` in the app runs all eleven of them, and
each checks for its marker. That cost is nothing, but one consequence is not:
`activate_pause_button` takes `Res<Session>`, so a click on the *menu* needs a `Session` to exist. That is why
`gameplay.rs` creates one at startup, with a comment explaining the panic it prevents. And the
two `start_game_from_*` observers each open with an `if *screen.get() != Screen::X { return; }`.

Bevy 0.19 added run conditions to observers ("Previously, run conditions only worked for
ordinary systems. Observers couldn't use them. Now, they can!" — 0.19 release notes). So an
observer can carry the same `.run_if(in_state(...))` the screen systems already carry.

**Facts.**
- `app.add_observer(activate_pause_button.run_if(in_state(Pause::On)))` compiles as is:
  `ObserverSystemExt::run_if` is in the prelude.
- Scope each observer by where its buttons live: `Screen::Menu` (menu button, start from menu),
  `Screen::Gameplay` (start from gameplay), `Pause::On` (pause button, confirm quit),
  `Overlay::SaveGame` (new save, ask to overwrite, overwrite), `Overlay::LoadGame` (load, ask to
  delete, delete), `Overlay::Settings` (fullscreen, volume, language). `close_overlay` and
  `close_dialog` stay global: their buttons live everywhere.
- Delete the `screen: Res<State<Screen>>` parameter and early `return` in both
  `start_game_from_*`.
- `app.init_resource::<Session>()` in `gameplay.rs` is then no longer needed by any click, and
  the experiment removed it. **Recommended: keep it anyway, with a new comment.** A default
  `Session` is a valid new game, and with it a developer can start straight in
  `Screen::Gameplay` (a common shortcut while building the game) without a panic in
  `count_play_time`. Only its reason changes: from "a click panics without it" to "gameplay can
  be entered without the menu".
- `in_state` is false when the state does not exist, so `in_state(Pause::On)` is safe outside
  gameplay.

**Proven.** A headless test (`MinimalPlugins` + `StatesPlugin`, the pause plugin, one
`Activate` triggered on an empty entity, no `Session`):
- today's code: `panicked ... Encountered an error in observer activate_pause_button:
  Parameter Res<'_, Session> failed validation: Resource does not exist`
- with the `run_if`: passes.

The whole step then passes `just check`, and the game launched and ran 40 s without a panic.

**Test.** Keep that headless test, `a_click_outside_gameplay_needs_no_session`, in a
`// ==== Tests ====` section at the bottom of `pause.rs`: it is the guard against this class of
panic coming back. What it needs:
- `bevy::state::app::StatesPlugin` next to `MinimalPlugins`, then `init_state::<Screen>()`
  (the `Pause` sub-state hangs off it) and `init_state::<Overlay>()` (`activate_pause_button`
  takes `NextState<Overlay>`), then the pause `plugin` and one `app.update()`
- `app.world_mut().trigger(Activate { entity })` on a `spawn_empty()` entity, and another
  `update()`
- an assertion, as CONTRIBUTING requires: the app is still on `Screen::Splash`
  (`app.world().resource::<State<Screen>>()`). Reaching it is the point, since the old code
  panicked before it.

**Docs.** CONTRIBUTING's Bevy patterns gain "screen-scoped observers get the same `run_if` as
the screen's systems"; `docs/flow.md` rewrites why the `Session` is "also created at startup"
(Gameplay) and drops "each checks `State<Screen>` first" (Starting a game).

**Commit.** `refactor: scope observers to their state with run_if`

### 4. `SaveFinished`: the pause menu owns its status line

**Why.** `overlays/saves.rs` writes the pause menu's text directly: it imports
`screens::gameplay::pause::SaveStatus`, queries `(Text, TextColor, SaveStatus)`, picks the
translation and the color, and resets the fade. The same five-line loop appears in
`save_new_game` and `overwrite_save`. An overlay knowing the pause menu's widgets is the worst
of the three places where a feature reaches into another's internals (rule 3 above).

The fix is a domain event: the save list says what happened, the pause menu decides how to show
it, the same shape as `StartGame`.

**Facts.**
- In `save.rs`: `#[derive(Event)] pub(crate) struct SaveFinished { pub(crate) saved: bool }`.
- In `saves.rs`: both observers `commands.trigger(SaveFinished { saved })`. Replace
  `save_status` with a helper that returns whether the write worked and logs why not;
  `Result::inspect_err(...)` then `.is_ok()` is the one-line form.
- In `pause.rs`: a `show_save_status(finished: On<SaveFinished>, ...)` observer, and
  `SAVED_TEXT`, `SAVE_FAILED_TEXT` and the accent/red choice move there. `SaveStatus` and its
  field become private.
- `saves.rs` then drops its imports of `SaveStatus`, `DANGER`, and the `Translations`/`Language`
  parameters of both observers.

**Proven.** Compiles in both configs, all tests pass. `status_color`'s two tests go away with
the function: they only restated its `if`.

**Commit.** `refactor: let the pause menu show how a save went`

### 5. One `Quit(QuitTo)` action in the pause menu

**Why.** `QuitToMenu` and `QuitToDesktop` are the same arm twice (ask if unsaved, else leave),
and `confirm_quit` repeats the leaving a third time.

**Facts.**
- `PauseAction::Quit(QuitTo)` replaces the two variants; `QuitTo` needs `#[derive(Clone, Copy)]`
  to be copied out of the query (`*to`).
- One helper does the leaving, called from both observers: it takes `to`, `&mut
  NextState<Screen>` and `&mut Commands`. A `ResMut<NextState<Screen>>` passes as
  `&mut next_screen`.

**Proven.** Compiles, tests pass.

**Commit.** `refactor: merge the pause menu's two quits into one action`

### 6. Esc as a tested pure function

**Why.** `go_back_on_esc` holds the game's most important priority rule (dialog, then overlay,
then pause toggle) and it is the one rule with branches and no test. CONTRIBUTING says logic
lives in pure functions.

**Facts.**
- `bevy::input::common_conditions::input_just_pressed(KeyCode::Escape)` as the system's run
  condition replaces the outer `if` and the `keyboard_input` parameter.
- A pure `esc_action(dialog_open: bool, overlay: Overlay, pause: Option<Pause>) -> EscAction`,
  with `EscAction { CloseDialog, CloseOverlay, SetPause(Pause), Nothing }`; the system matches
  on it.
- `Pause` must derive `Copy` to go from `Res<State<Pause>>` to `Option<Pause>` (the compiler:
  `cannot move out of a shared reference`). `Screen` and `Overlay` already do.

**Tests.** Five, one per rule: a dialog first, an overlay before resuming, resume, pause, and
nothing outside gameplay.

**Commit.** `refactor: make the Esc rule a tested pure function`

### 7. One helper for "the saves on disk"

**Why.** `saves_dir().map(|dir| list_saves(&dir)).unwrap_or_default()` is written four times:
`show_continue_button`, the Continue click, and both overlays. CONTRIBUTING's third-user rule
says it moves to the shared file.

**Facts.** A `pub(crate) fn` in `save.rs` with no parameters returning `Vec<SaveEntry>`.
`list_saves(dir)` keeps its parameter: that is what lets its tests use a temporary folder.
`menu.rs` then no longer imports `list_saves` or `saves_dir`.

**Proven.** All four call sites replaced; compiles and passes.

**Commit.** `refactor: read the saves folder through one helper`

### 8. Small simplifications

Each one is a few lines, all in one commit, `refactor: simplify small spots across the screens`.

- `spawn_load_game_overlay`: the `Incompatible` and `Unreadable` arms are identical but for the
  key. One arm, `SaveEntry::Incompatible(_) | SaveEntry::Unreadable(_)`, with the key from a
  small helper (`matches!(entry, SaveEntry::Incompatible(_))` picks it). Same in
  `spawn_save_game_overlay`. `saves.rs` loses about 35 lines.
- `spawn_save_game_overlay` skips autosaves at the top of its loop, then passes
  `is_autosave(path)` to `save_name`, which is therefore always `false` there.
- `next_logo`: `(index + 1 < count).then_some(index + 1)` is the whole function.
- `fade_save_status`: the `if` that clamps `shown_secs` is
  `(status.shown_secs + time.delta_secs()).min(STATUS_FADE.total_secs())`.
- Three constants of 10 px say "the gap between stacked buttons": `BUTTON_GAP_PX` (menu),
  `PAUSE_BUTTON_GAP_PX`, `DIALOG_BUTTON_GAP_PX`. One `pub(crate) BUTTON_GAP_PX` in `ui.rs`.
- Name judgments for the user: `show_continue_button` also hides it; `STATUS_FADE` is only for
  the save status (`SAVE_STATUS_FADE`); the `.ftl` key `saves-none` is the only plural `saves-`.

**Proven.** All of the above together compile, pass, and launch clean.

### 9. A narrow public surface for `screens`

**Why.** Rule 3 of the dependency rule. `screens::loading` and `screens::gameplay` are
`pub(crate)` only so other features can name `LoadingDestination` and `Pause` by path.

**Facts.** Move `LoadingDestination` into `screens.rs` beside `StartGame`, which carries it.
Add `pub(crate) use gameplay::pause::Pause;` in `screens.rs`. Then `mod loading;` and
`mod gameplay;` are private, and callers write `crate::screens::{LoadingDestination, Pause}`.

**Proven.** Compiles and passes; no other change needed.

**Commit.** `refactor: expose only what other features use from screens`

### 10. `disk.rs` out of `save.rs`

**Why.** `settings.rs` imports `crate::save::{game_data_dir, write_atomic}`: settings depend on
"save" for something that is not about saves. The `Disk` section is a domain of its own, the
game's data folder and how files reach it, and CONTRIBUTING's "does this belong here?" test
fails for it in `save.rs`.

**Facts.** Move `data_dir`, `game_data_dir`, `write_atomic` and their tests (the six `data_dir`
tests and `written_file_reads_back_whole`) to `src/disk.rs`, under a `Data Folder` section.
`saves_dir`, the save paths and the save reading/writing stay in `save.rs`. `save.rs` goes from
656 to about 560 lines; `disk.rs` is about 115.

**Proven.** Pure move; compiles and passes unchanged.

**Commit.** `refactor: move the data folder and atomic writes into disk.rs`

### 11. `ui.rs` next to `ui/`, `blur.rs` inside it

**Why.** `ui.rs` is 554 lines, and buttons are a part that makes sense on its own (their colors,
easing, sounds and six tests), the trigger CONTRIBUTING gives for a split. `blur.rs` only exists
for floating UI, so it belongs with it.

**Facts.**
- `ui.rs` keeps the palette, the type sizes, `BORDER_PX`, step 8's `BUTTON_GAP_PX` (both
  `button.rs` and `panel.rs` use it, so it stays in the parent), `font`, `label`, `heading`, and
  its plugin adds the three sub-plugins. It re-exports what others use:
  `pub(crate) use button::{ButtonKind, button, menu_button, row_button};` and
  `pub(crate) use panel::{Dialog, card, dialog, floating_root};`, so no caller changes.
- `ui/button.rs`: the button constants, `ButtonKind`, `ButtonMotion`, `ButtonSounds`,
  `paint_buttons`, `play_button_sounds`, `button_colors` and its tests.
- `ui/panel.rs`: `SCREEN_DIM`, card, entrance and dialog constants, `floating_root`, `card`,
  `dialog`, `Dialog`, `CloseDialog`, `Card`, `Entrance`, `animate_entrances`, `close_dialog`,
  `entrance_transform` and its tests.
- `ui/blur.rs`: moved whole, `pub(crate) mod blur;` in `ui.rs`; `main.rs` imports
  `ui::blur::BlurStrength` and stops adding `blur::plugin` itself.

Result, measured before step 8: `ui.rs` 70 lines, `button.rs` 303, `panel.rs` 219, `blur.rs`
157. This step depends on step 8 only through `BUTTON_GAP_PX`; done in this order, nothing else
crosses.

**Proven.** Compiles in both configs, all tests pass, and the game launched from it ran clean.

**Commit.** `refactor: split ui.rs into buttons, panels and the blur`

### 12. Lint test code and the "no `mod.rs`" rule

Two commits.

**Lint the tests.** `just check` runs `cargo clippy -- -D warnings`, which skips `#[cfg(test)]`
code. Add `--all-targets` to both clippy lines in the `justfile` (and to `just fix`'s, so it
fixes what it now finds). Today that reports 9 errors, all `field_reassign_with_default` in
`save.rs`'s tests: `let mut session = Session::default(); session.play_secs = X;` is
`Session { play_secs: X, ..default() }`. After that, `just fix` removes the `mut`s left over.
**Proven:** with the fixes, `--all-targets` passes in dev and release.
Commit: `build: lint the tests too`.

**No `mod.rs`.** CONTRIBUTING forbids `mod.rs`, and today only review enforces it. Clippy can.
(`bevy_new_2d` uses `mod.rs` everywhere, so this is this project's choice, not Bevy's; the
lint makes the choice stick.) `mod_module_files = "warn"` under `[lints.clippy]`; `just check`
turns it into an error. **Proven:** silent on today's tree; a `src/probe/mod.rs` fails with
``error: `mod.rs` files are not allowed``.
Commit: `build: forbid mod.rs files with a clippy lint`.

Also in `Cargo.toml`, the user's call: `version = "0.1.0"` is what the menu's corner shows, for a
template the user calls 1.0. Either `1.0.0` here plus a `v1.0.0` git tag, or leave the package at
0.1.0 for the games made from it and only tag. Separate commit: `build: version the template 1.0.0`.

### 13. Tests

See [Tests review](#tests-review) for the reasoning; the commit adds:

- One `#[cfg(test)] pub(crate) fn fresh_test_dir(name: &str) -> PathBuf` in `disk.rs` (after
  step 10) that removes the folder (ignoring "not found") and creates it empty; every disk test
  in `disk.rs` and `save.rs` starts with it. Four of them write into a fixed folder under the
  temp dir and never empty it, so files from an earlier run are still there next time. One
  helper rather than a copy per test module, unlike `TOLERANCE`: this is logic, not a number.
  The folder names stay as they are (`bevy-starter-list-saves`, ...).
- `underscore_locale_is_read_like_a_dash`: `language_from_locale("pt_BR")`, the case the comment
  in `language_from_locale` promises and no test covers.
- `no_translation_uses_a_variable_english_lacks`: walks each message's `fluent_syntax` AST
  (`Expression::Inline`/`Select`, `InlineExpression::VariableReference`) and checks every
  variable in a message's value is one English passes. It does not walk message attributes or
  function arguments (`NUMBER($x)`); no `.ftl` here uses either, and the test should grow when
  one does. **Proven:** passes on today's files, and
  fails on `age-days = há { $cuont }d` with `pt-BR age-days: {"cuont"}`. Today that typo only
  shows up in the game, as broken text.

**Commit.** `test: isolate disk tests and check translation variables`

### 14. Polish the Portuguese and Japanese text

Only `.ftl` changes. The user is the native Portuguese speaker, so each line is a proposal:

| Key | Now | Proposed | Why |
| --- | --- | --- | --- |
| `pause-resume` | Voltar | Retomar | "Voltar" reads as "back"; Minecraft pt-BR uses "Retomar partida" |
| `pause-quit-to-menu` | Ir ao Menu | Voltar ao Menu | Minecraft: "voltar ao menu principal" |
| `dialog-overwrite-save` | Sobrescrever este save? | Sobrescrever este jogo salvo? | published pt-BR games avoid the noun "save" |
| `dialog-delete-save` | Apagar este save? | Apagar este jogo salvo? | same |
| `save-new` | Novo Save | Novo Salvamento | same |
| `saves-none` | Nenhum save ainda | Nenhum jogo salvo | same |
| `save-incompatible` | Save incompatível | Jogo Salvo Incompatível | same, and the capitalization rule below |
| `save-unreadable` | Save ilegível | Jogo Salvo Ilegível | same |
| `save-autosave` | Autosave | Salvamento Automático | Minecraft: "salvamento automático" |
| `settings-fullscreen` | Tela cheia | Tela Cheia | capitalization rule below |
| `save-unnamed` | Sem nome | Sem Nome | same |
| Settings language row | Português | Português (Brasil) | the language is pt-BR, and pt-PT players are sent to it too; W3C: name languages in their own language, variant included |
| `pause-quit-to-menu` (ja) | タイトルへ | タイトルへ戻る | the complete phrase, as Minecraft ja uses |

**Capitalization.** The Portuguese file mixes Title Case ("Novo Jogo", "Salvar Jogo") with
sentence case ("Tela cheia", "Sem nome"). Microsoft's pt-BR style guide (4.1.5) follows the
English source's capitalization in UI options and uses sentence case for messages, "for
localization simplicity". Minecraft uses sentence case throughout. Either is defensible; the
proposal follows Microsoft's, because it is mechanical for every future key: copy the English
casing on buttons and titles, sentence case on questions and status lines. Whichever is picked
goes into CONTRIBUTING's Text section.

Kept as they are, with a source: "Opções", "Apagar" (Minecraft; Microsoft says "Excluir", both
defensible), "Pausado", "Sair do Jogo", "Feito com Bevy". Japanese: はじめから/つづきから,
セーブ/ロード, オートセーブ, 削除, 互換性のないデータ are standard; 読み込めないデータ is honest when the
cause is unknown (破損したデータ would claim corruption). The Japanese file stays a draft until a
native speaker reads it.

The `native_name` change is code (`i18n.rs`), and "Português (Brasil)" is longer: check with
`just run` that the radio row still fits the card.

**Commit.** `fix: polish the Portuguese and Japanese wording`

### 15. Docs pass

Claude, after step 14: `docs/flow.md` (observers scoped, `SaveFinished`, the Esc function, the
new files), `docs/template.md` (project structure tree), the CONTRIBUTING rules each step names,
and this spec's status. `just check` is skipped for it: docs only.

## Beyond the refactor

These change what the player sees or does, so each needs its own spec and the user's decision.
Ranked by how much a shipped game would miss them.

1. **Keyboard and gamepad navigation of every menu.** Today the menus are mouse-only. Steam Deck
   Verified requires that the default controller configuration reach all content with no
   setting to enable it, and that text entry (the save name) open the on-screen keyboard. Xbox
   Accessibility Guideline 107 asks that a player can start, adjust settings, play and quit with
   the keyboard alone, and for digital and analog menu navigation. The UI style spec deferred
   this to its own spec; `bevy_input_focus` (already in the tree) has tab and directional
   navigation to build it on.
2. **Autosaves grow without end.** Every autosave is kept (save-game spec, item 25): 12 files an
   hour, all listed in Load Game. No shipped game found keeps them all: Skyrim and Fallout 4
   rotate 3 slots, Baldur's Gate 3 lets the player set a maximum. The reason recorded for keeping
   all was being able to go back to any autosave; keeping the newest N (3 to 5) still allows
   going back N, and the list stays readable. A pure `autosaves_to_delete(entries, keep)` with
   tests would hold the logic.
3. **Loading from the pause menu asks nothing** (save-game spec, item 37, "can be added if it
   turns out to be missed"). Quitting asks; loading loses the same progress. Factorio has the
   same asymmetry, and a player reported it as a bug. The quit's `dialog()` and
   `has_unsaved_progress` already do the work.
4. **Any key skips the whole intro, and it can never be replayed.** Accidental skips are a
   documented complaint (ScummVM's Chewy port); Game Accessibility Guidelines ask that narrative
   be replayable. The usual fixes: the first press shows "press again to skip", or a short hold
   (XAG 107 warns against long holds). Splash logos stay one-press skippable, which matches what
   players want.
5. **A loaded game appears with a cut.** Continue and Load Game go through Loading, which leaves
   on the next frame (nothing to load yet), so the player sees one black frame, then gameplay
   with no fade. Either skip Loading when there is nothing to load, or give it a minimum time,
   and fade gameplay in from black (the veil already exists in the menu).
6. **Accessibility labels are English-only.** `AccessibleLabel::new("Fullscreen")` and
   `("Volume")` never change language. XAG 106: what a sighted player reads, a screen reader
   should say. Localize them from the same keys as their visible labels.
7. Smaller, optional: pause when the window loses focus (common as an option, off by default);
   Esc on the main menu opening a quit confirmation (a convention, no guideline found).

## Tests review

114 tests today. What they cover is right: pure functions, one assertion of behaviour each, named
as sentences. The review per group:

| Group | Verdict |
| --- | --- |
| `fade`, `approach`, veil, splash, slideshow, intro frames | keep; they pin the timing |
| `screen_after_veil` (2) | keep: it was added after swapped arms passed every check |
| `screen_after_loading` (2) | low value, they restate a one-to-one `match`; keep only because removing them buys nothing |
| `status_color` (2) | removed by step 4 with the function |
| `entrance_is_complete_once_its_time_is_up` | duplicates what the `fade` tests prove for any `Fade`; harmless |
| `button_colors` (6), `entrance_transform` (2) | keep |
| `i18n` (12) | keep; add the underscore locale and the variables test (step 13) |
| `settings` (13) | keep |
| `save` (32) | keep; isolate the four disk tests (step 13) |
| `saves` (12) | keep; add the blank-name test (step 1) |

Missing, added by the steps: the Esc rule (step 6, five tests), the observer scoping guard
(step 3), the blank name (step 1), translation variables and the underscore locale (step 13):
114 − 2 + 9 = **121**. And the tests themselves get linted from step 12 on.

Not proposed: a full-app smoke test. It would need a window, a GPU and the assets, which `cargo
test` on a plain machine does not have; the headless test in step 3 covers the one thing such a
test caught in this review.

## Considered and turned down

- **An entity observer per button** (`observe(handler)` from `bevy_ui_widgets`), as Bevy's
  `standard_widgets` example and `bevy_new_2d` do. It would remove the action enums and markers.
  Turned down: the menu-screens spec chose markers read by one observer per file on purpose
  ("keeps the spawn code declarative and the behaviour in one place"), step 3 removes the one
  real cost the global observers had, and the `observe` helper's own source says
  `TODO: This probably doesn't belong in bevy_ui_widgets`, so it may move in 0.20.
- **Layer folders** (`domain/`, `application/`...): see [Architecture](#architecture).
- **The `dirs` or `directories` crate** for the data folder, as `bevy-persistent` and `bevy_pkv`
  do. `data_dir` is fifteen lines with six tests; a crate is one more thing to move each Bevy
  release.
- **Splitting `overlays/saves.rs`** (678 lines) into save and load files. Its two halves share
  every helper (`save_list`, `save_row`, `save_name`, `save_details`, `age_label`), and step 4
  and step 8 take it down by about 40 lines. Split when the next list feature lands.
- **Moving `autosave_game` into `save.rs`.** It runs only in `Pause::Off`, so `save.rs` would
  import `screens`, breaking rule 2 of the dependency rule.
- **A `full_window()` node helper.** Ten nodes start with `width: percent(100), height:
  percent(100)`, but each adds different fields; a helper saves one line each and hides layout.
- **One shared test `TOLERANCE`.** Five test modules define `1e-5`; sharing it would couple
  test modules for a one-line constant.
- **Translation key constants everywhere, or nowhere.** Eight keys are constants (`*_QUESTION`,
  `*_TEXT`) and 34 are written inline. Five of the eight are used in more than one place; of
  the other three, two are named in doc comments and `NO_SAVES_TEXT` in nothing. A constant
  everywhere adds a name per key, and inline everywhere loses the shared spelling. The rule
  this suggests, for step 15 to write down: a constant when the key is used in more than one
  place, inline otherwise, which makes `NO_SAVES_TEXT` inline.
- **Gating `update_fullscreen_checkbox` so it stops painting every frame.** Turned down in the
  2026-09-23 review (recorded only in that session's notes, not in `docs/`): the checkbox would
  then need a second path to be painted when the panel opens, more code to save the cost of one
  entity per frame. Nothing has changed that.

## Docs already fixed

Done in the same pass as this plan, since they described the code wrongly:

- `CLAUDE.md`: the working agreements from the past sessions (Claude does pure moves; "implementa
  aí" means the whole sub-step; read the design before proposing moves; specs are history).
- `CONTRIBUTING.md`: overlays and pause despawn like screens; buttons as markers; `Time<Real>`
  for what moves while paused; floating UI from `floating_root`/`card`; named `*_Z_INDEX`;
  `write_atomic`; a Text section (keys, one key per place, no translated proper nouns); disk
  tests in their own folder; `Events` in the file layout; the spec workflow in the loop; a stale
  example (`tick_splash_timer` no longer exists).
- `docs/flow.md`: Close is `close_footer()` with the `overlay-close` key; Gameplay also ends by
  `confirm_quit`; Settings remembers three choices and has three gated systems; `font()` and the
  status line in the shared-code table.
- `docs/template.md`: rename `GAME_TITLE`, not a window `title`; change `CREDITS_NAMES` and the
  version; the trimmed-features line now keeps `jpeg` and `wav` (it compiled and ran with exactly
  `["2d", "ui", "audio", "jpeg", "wav"]`; without those two the menu art, slides and button
  sounds would not load); `menu_button("menu-new-game")` in the example.
- `README.md` and `justfile`: `just test` listed and described.
- The localization and UI style specs: status lines said "not yet tried" and "uncommitted";
  both are committed (`0cfc9ed`, `87df405`).

## Sources

- Bevy 0.19 release notes, observer run conditions: https://bevy.org/news/bevy-0-19/
- Bevy, games as plugins: https://bevy.org/learn/quick-start/getting-started/plugins/
- `bevy_new_2d` structure and design notes: https://github.com/TheBevyFlock/bevy_new_2d and
  https://github.com/TheBevyFlock/bevy_new_2d/blob/main/docs/design.md
- Bevy 0.19.1 examples: `examples/ui/widgets/standard_widgets.rs`, `examples/showcase/game_menu.rs`,
  `examples/scene/world_serialization.rs` (IoTaskPool writes)
- Layering Bevy games: https://morgenthum.dev/blog/introduce-layers-into-bevy/
- `input_just_pressed`: https://docs.rs/bevy/0.19.1/bevy/input/common_conditions/fn.input_just_pressed.html
- Steam Deck compatibility: https://partner.steamgames.com/doc/steamdeck/compat
- Xbox Accessibility Guidelines 106, 107, 114, 116:
  https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/107
- Game Accessibility Guidelines: https://gameaccessibilityguidelines.com/full-list/ and
  https://gameaccessibilityguidelines.com/allow-all-narrative-and-instructions-to-be-replayed/
- Autosave rotation: https://stepmodifications.org/wiki/Guide:Skyrim_INI/SaveGame and
  https://stepmodifications.org/wiki/Guide:Fallout4Prefs_INI/SaveGame
- Load without a warning reported as a bug: https://forums.factorio.com/viewtopic.php?p=517519
- Too-easy intro skip: https://bugs.scummvm.org/ticket/13706
- Microsoft pt-BR style guide:
  https://download.microsoft.com/download/8/e/3/8e349e32-9eb9-4b63-9909-7586b94a24dd/por-bra-StyleGuide.pdf
- Minecraft pt-BR and ja strings (1.21):
  https://raw.githubusercontent.com/InventivetalentDev/minecraft-assets/1.21/assets/minecraft/lang/pt_br.json
- Fluent best practices (one ID per context): https://mozilla-l10n.github.io/documentation/localization/dev_best_practices.html
- Language names in their own language: https://www.w3.org/International/questions/qa-navigation-select

## What changed while building it

- **Who wrote what.** The user wrote steps 1 to 4 and the first half of step 5. At their
  request Claude implemented the rest of step 5, steps 6 to 8 and 12 to 14, besides the moves
  (9 to 11) and the docs (15) it had been given.
- **Step 4.** The helper that writes a save and says whether it worked is `try_write_save`.
- **Step 6.** `EscAction` sits in `Helpers`, in a group with `esc_action` and next to its tests,
  rather than in a types section: it is only that function's answer.
- **Step 7.** `list_saves` became private once `saves_on_disk` was its only caller outside the
  tests, with a comment on why it still takes the folder.
- **Step 8.** `saves.rs` lost about 12 lines, not 35: the Save Game arms were one line each, and
  `unloadable_save_text` gained two tests. The user's names: `update_continue_button`,
  `SAVE_STATUS_FADE`, and `saves-none` kept.
- **Step 10.** What stays in `save.rs`'s old `Disk` section is now named `Save Files`.
  `docs/flow.md` still said `write_save` creates the folder, stale since step 2; fixed there.
- **Step 11.** Measured after step 8: `ui.rs` 73 lines, `button.rs` 310, `panel.rs` 222,
  `blur.rs` 157. `panel.rs` reaches the buttons through `ui`'s re-exports, like any caller.
- **Step 12.** The user picked `1.0.0` in `Cargo.toml` plus the `v1.0.0` tag.
- **Step 13.** The two tests that need a missing folder ask for one inside a fresh folder
  (`fresh_test_dir(...).join("saves")`), so it is missing by construction.
- **Step 14.** The whole table went in, and the casing rule is Microsoft's; CONTRIBUTING's Text
  section records it. "Português (Brasil)" still needs a look with `just run`.
- **Docs as they went.** Each step updated `docs/flow.md`, `docs/template.md` and CONTRIBUTING in
  its own commit, so step 15 was left with this section and the status line.
