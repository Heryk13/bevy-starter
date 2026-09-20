# Menu, Settings, Credits and Gameplay screens

Status: **awaiting approval** — no code written yet.
Date: 2026-09-20
Bevy: 0.19.1

## Goal

Today the splash screen transitions to `Screen::Menu`, which has no plugin: the game
shows a black window and the player can only alt-F4 out of it. This design closes the
flow with a real menu and the screens it reaches, so every path a player can take leads
somewhere and comes back.

## Decided

| Question | Decision |
| --- | --- |
| Buttons | Play, Settings, Credits, Quit |
| Play leads to | `Screen::Gameplay`, an empty screen (background colour only) |
| Settings controls | Fullscreen on/off and master volume |
| Settings persistence | None. Both reset to the default on every run |
| Navigation | Mouse only, plus Esc to go back |

## Out of scope

Each of these is a separate design cycle, not a "while we are here":

- Saving preferences to disk (serialization, per-OS paths, corrupt-file handling)
- Keyboard and gamepad navigation (focus state, selected-item highlight)
- Any actual gameplay inside `Screen::Gameplay`
- Audio assets. The volume setting moves a number nothing plays yet

## Screen flow

```
Splash --> Menu --> Play ------> Gameplay --+
             |                              |
             |----> Settings ---------------+---> Esc / Back returns to Menu
             |----> Credits ----------------+
             |
             +----> Quit -----> AppExit
```

`Splash` already skips to `Menu` on any key or click; that stays as it is.

## Structure

**Recommended: keep `Screen` flat.** Every new screen becomes one more variant of the
existing enum and one more file under `src/screens/`, shaped exactly like `splash.rs`:

```rust
enum Screen {
    #[default]
    Splash,
    Menu,
    Settings,
    Credits,
    Gameplay,
}
```

Why this and not a sub-state for the menu pages: the machinery the project already
uses — `DespawnOnExit(Screen::X)` and `.run_if(in_state(Screen::X))` — works per
`Screen` variant. A flat enum introduces zero new concepts, and each screen file stays
readable on its own.

The alternative, which Bevy's own `examples/showcase/game_menu.rs` uses, is a second
state (`MenuState`) holding the menu pages. It pays off when menu pages must coexist
with another screen — a pause menu drawn over a running game — or when "back" needs a
stack instead of a fixed destination. Neither is true yet.

**Known ceiling:** with a flat enum, "back" is hardcoded to `Screen::Menu`. The day a
pause menu opens Settings from inside gameplay, Settings will return to the main menu
instead of the game. The fix at that point is either a `PreviousScreen` resource or the
sub-state above. This gets a `ponytail:` comment in the code naming the ceiling.

## Files

| File | Change |
| --- | --- |
| `src/screens.rs` | 3 new `Screen` variants, register 3 plugins, shared Esc system |
| `src/screens/menu.rs` | new — title and the 4 buttons |
| `src/screens/settings.rs` | new — fullscreen toggle, volume, Back |
| `src/screens/credits.rs` | new — static text, Back |
| `src/screens/gameplay.rs` | new — background colour only |
| `src/ui.rs` | new — button widget, palette, interaction colouring |
| `src/main.rs` | one line: `mod ui;` |
| `README.md` | one new bullet under Features; three sections move out |
| `docs/template.md` | new — Starting a new game, Project structure, Assets |
| `docs/screens.md` | new — the screen flow and what each screen does |

## `src/ui.rs`

Nine buttons across three screens (four in the menu, four in settings, one in credits)
need the same bundle and the same hover/press colours. Writing that nine times is the
real risk in this change, so it lives in one module:

- `Config`: the palette (normal, hovered, pressed) and button sizing constants
- `pub fn button(text: &str) -> impl Bundle` — a `Button` node with its `Text` child
- One system painting `BackgroundColor` from `Interaction`, for every button in the
  app, with no `run_if`: it is generic over any button, on any screen
- `pub fn button_color(interaction: &Interaction) -> Color` — the pure function the
  system calls, and what the tests cover

This is the one place the design bends `CONTRIBUTING.md`'s "split by feature, never by
kind of code": `ui.rs` is a kind. The justification is three consumers and a system
that must exist exactly once. Flagged here on purpose rather than smuggled in — see
"Needs your call" below.

## Button actions

Each button carries a marker component naming what it does, and one small system per
screen reads it:

```rust
#[derive(Component)]
enum MenuAction {
    Play,
    Settings,
    Credits,
    Quit,
}
```

The system queries `(&Interaction, &MenuAction), Changed<Interaction>`, matches on
`Interaction::Pressed`, and sets `NextState<Screen>` — or writes `AppExit` for Quit.
This mirrors `examples/ui/widgets/button.rs` from Bevy 0.19.1 and keeps the file in the
plain-systems style `splash.rs` already uses. Observers (`On<Pointer<Click>>`) are the
other option; they are idiomatic in 0.19 too, but they spread the behaviour across
closures at spawn time instead of one readable system.

## Settings behaviour

**Fullscreen.** The window is the state, so no resource is added: the system takes
`Single<&mut Window>` and flips `window.mode`. Borderless rather than exclusive
fullscreen — no resolution switching and instant alt-tab:

```rust
WindowMode::Windowed  <->  WindowMode::BorderlessFullscreen(MonitorSelection::Current)
```

**Volume.** Bevy already owns this state as the `GlobalVolume` resource
(`GlobalVolume { volume: Volume }`, with `Volume::Linear(f32)`). Two buttons, `-` and
`+`, step it by `VOLUME_STEP` clamped to `0.0..=1.0`, and a label shows the current
percentage. No new resource.

Both APIs were verified against the local 0.19.1 source rather than from memory:
`bevy_window-0.19.1/src/window.rs` and `bevy_audio-0.19.1/src/volume.rs`. Note that
`AppExit` is a *message* in 0.19: the writer is `MessageWriter<AppExit>`, not
`EventWriter`.

No font asset is needed. `default_font` arrives through the `ui` feature chain
(`ui -> default_platform -> default_font`), so `Text` renders with Bevy's built-in font.

## Esc

"Esc goes back to the menu" is one behaviour, not three, so it is one system in
`screens.rs` rather than a copy in each screen file. It runs in `Settings`, `Credits`
and `Gameplay`. It does nothing in `Splash`, which already skips on any key, and
nothing in `Menu`, where Esc would mean quitting and the Quit button covers that.

## Tests

The rule stands: logic in pure functions, systems stay thin. What gets covered:

| Function | Cases |
| --- | --- |
| `ui::button_color` | one per `Interaction` variant |
| `settings::toggled_window_mode` | windowed to borderless, borderless to windowed |
| `settings::stepped_volume` | step up, step down, clamp at 0.0, clamp at 1.0 |

Floats compared with a tolerance, as in `splash.rs`.

## Needs your call

1. **Flat `Screen` enum** vs a `MenuState` sub-state for the menu pages. Recommendation
   above: flat, upgrade when a pause menu exists.
2. **`src/ui.rs` as a module organized by kind.** The alternative is duplicating the
   button bundle in three files, which is worse. Approving this means accepting one
   documented exception to the "split by feature" rule.
3. **Volume as `-`/`+` steps** vs three presets (Low/Medium/High, which is what Bevy's
   `game_menu.rs` example does). Steps are finer; presets are fewer clicks and fewer
   states to reason about.
4. **Volume is inaudible** until the game has sound. If you want to hear it work, the
   cheap route is a click sound on button press — one `.ogg` asset and one observer.
   That is an addition to this scope, not part of it.

## Documentation split

The `README.md` is the front door: what this is, what you need installed, and the
commands to run it. Everything that describes how the game is *built* goes under
`docs/`, so the README stops growing every time a screen is added.

| Where | What | Why |
| --- | --- | --- |
| `README.md` | Features, Requirements, Usage, Cargo features, Releasing | Read on day one and used every day |
| `docs/template.md` | Starting a new game, Project structure, Assets | Read once, when the template is turned into a game |
| `docs/screens.md` | Screen flow, what each screen does, how to add one | Grows with every feature |
| `docs/specs/<name>/` | One folder per implementation: `design.md`, then `plan.md` | History of why, not how-to |

Moving Project structure out of the README is the point of the split: it is the one
section that gains a line for every file the game adds.

Step 6 of the loop in `CONTRIBUTING.md` currently says to update the `README.md` when
the project structure changes. After the split that sentence points at the wrong file,
so it moves in the same commit: features and commands stay with the README, project
structure points at `docs/template.md`.

This split is independent of the menu itself and can land as its own commit before
anything else, which is why it is step 0 below.

## Implementation order

Each step ends green (`just check`) and is its own commit:

0. `README.md` and `docs/template.md` — the documentation split above
1. `src/ui.rs` — button widget and colour system, with its tests
2. `src/screens/gameplay.rs` plus the `Gameplay` variant — the simplest screen, proves
   the pattern end to end
3. `src/screens/menu.rs` — the four buttons, with Play and Quit working
4. `src/screens/credits.rs` — static text and Back
5. `src/screens/settings.rs` — fullscreen and volume, with its tests
6. Shared Esc system in `screens.rs`
7. `docs/screens.md` — the finished flow, plus one new bullet under Features in the
   `README.md` and the new files in the project structure inside `docs/template.md`
