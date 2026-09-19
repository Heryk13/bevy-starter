# Contributing

How every change in this project is made. Follow the loop for each feature, screen or fix.

## The loop

1. **Plan**: find where the change belongs. Code is organized by feature, not by who wrote it:
   each screen or feature is a plugin in its own file (e.g. `src/screens/menu.rs`).
2. **Write** following the [code conventions](#code-conventions).
3. **Test** the logic (see [Tests](#tests)).
4. **Fix and check**: `just fix`, then `just check`.
5. **Run**: `just run` and try the change in the game.
6. **Docs**: update `README.md` if features, commands, requirements or the project structure changed.
7. **Commit** (see [Commits](#commits)).

## Code conventions

### File layout

Every `.rs` file follows the same order, skipping the sections it doesn't have:

1. imports: external crates first, a blank line, then crate-local ones (`use super::...`)
2. child modules (`mod ...;`)
3. `Config`: tunable constants
4. `Plugin`: the `plugin` function, a summary of what the file does
5. types: `State`, `Components`, `Resources`
6. `Systems`: in the order they run (spawn, update, ...)
7. `Helpers`: pure functions used by the systems, next to the tests that cover them
8. `Tests`

### Splitting files

- Split by feature, never by kind of code: `player.rs` and `enemy.rs`,
  not `components.rs` and `systems.rs`.
- Keep a feature in one file while it reads as one thing.
  Split when a part makes sense on its own, usually around 300 to 400 lines
  or when you find yourself scrolling to look for "the part about X".
  That part becomes its own file.
- A split module follows the `screens.rs` pattern: `player.rs` next to a `player/` folder
  with the parts (`player/movement.rs`), never `mod.rs`.
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
- State that belongs to an entity is a component on it, not a global resource.
- Exactly one entity: `Single<...>`. Zero or more: `Query<...>`.
- Ask for the minimum access: `&` to read, `&mut` only to write.
- When a system reads what another writes, make the order explicit (`.after(...)` or `.chain()`).
- Tunable values are named constants in the `Config` section, never magic numbers.
- Dev-only code goes behind `#[cfg(feature = "dev")]`.

### Naming

- Systems: verb + noun (`spawn_splash_screen`, `tick_splash_timer`).
- Constants: `SCREAMING_SNAKE_CASE`, with the unit in the name (`LOGO_HOLD_SECS`).

## Tests

- Logic lives in pure functions: plain values in, plain values out.
  Systems stay thin: read from the ECS, call the function, write the result.
- Tests sit at the bottom of the same file, in `#[cfg(test)] mod tests` under `// ==== Tests ====`.
- Compare floats with a small tolerance instead of `assert_eq!`.

## Commits

- [Conventional Commits](https://www.conventionalcommits.org/), in English, imperative, short title:
  `feat: add main menu`, `fix: ...`, `refactor: ...`, `test: ...`, `style: ...`, `docs: ...`, `build: ...`.
- One change per commit.
- `git commit --amend` only before pushing.

## Dependencies

- New crates are fine when they save real work, but check first if Bevy or the standard library already does it.
- `just release` checks every dependency license against `about.toml`.
  Review a new license before adding it to the list.
