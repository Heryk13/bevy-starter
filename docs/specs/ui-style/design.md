# UI style: a darker, calmer look, with a blur behind every panel

Status: **built** — all nine steps written together on 2026-10-02, committed in `87df405`.
Date: 2026-10-02
Bevy: 0.19.1

## Goal

The UI works but looks unfinished: 100 × 50 px grey squares for buttons, panels that are loose
text floating in the middle of a dark veil, a red "off" checkbox, and no sign on screen of which
game this is. This design gives every screen, overlay and dialog one coherent look, in one place
a game made from the template can recolor: a dark, elegant style with a single amber accent,
pill buttons, glass panels with the screen blurred behind them, and short, subtle motion.

## Decided

| Question | Decision |
| --- | --- |
| Look | Dark and elegant: graphite surfaces, one amber accent, thin borders, soft shadows |
| Blur | A real blur of whatever is behind a panel, done as a post-process on the screen camera |
| How | Two cameras: screens below, floating UI (overlays, pause, dialogs) above (approach B) |
| Buttons | Pills with a thin border; amber border and glow on hover; three kinds |
| Panels | Centered glass cards with a header, a divider and a footer |
| Motion | Subtle: colors ease in 0.12 s, panels rise and scale in 0.18 s, the blur ramps both ways |
| Keyboard navigation | Not in this design; its own spec later |
| Commits | Left uncommitted until the user says otherwise, as in the localization work |

## Out of scope

- **Keyboard and gamepad navigation** (`bevy_input_focus`, focus rings). Decided to be its own
  design, to keep this one about the look.
- **Exit animations.** A panel leaves with `DespawnOnExit`; animating it out would mean
  delaying the despawn, more structure than 0.18 s is worth. The blur still fades out, and
  that is what reads as "closing".
- **Fading a whole panel.** Bevy 0.19 has no group opacity in the UI (nothing in `bevy_ui`
  for it), so the entrance moves and scales the panel's root with `UiTransform` instead.
- **New art.** The menu background, music and sounds stay the placeholders they are.
- **A second font weight.** Everything stays Noto Sans JP Regular; hierarchy comes from size
  and color. A bold face would be another 4.5 MB.

## The blur

### The three approaches

| | How | For | Against |
| --- | --- | --- | --- |
| **B (chosen)** | Two cameras on the window. The screen camera gets a `Blur` component when something floats; a two-pass Gaussian post-process blurs its output, UI included, before the floating camera draws on top | Public, documented APIs; picking works across both cameras with no code; blurs anything any screen will ever draw | Three settings that must be right (below) |
| A | Render the screens to an `Image`, show it through a `UiMaterial` blur shader | Single camera for the window | Bevy does not pick UI on a camera that renders to an image: pointer events would have to be forwarded by hand, and the image resized with the window |
| C | A pre-blurred copy of the menu background behind panels | Trivial | Only works where a known image is behind; the menu's buttons vanish instead of blurring, gameplay gets nothing |

### Facts B rests on (read in the 0.19.1 source)

- Rendering is no longer a `RenderGraph`: each camera runs the `Core2d` schedule, and a pass is
  a render-world system added to it (`bevy_core_pipeline-0.19.1/src/schedule.rs:73-105`).
- `FullscreenMaterial` (`bevy_core_pipeline-0.19.1/src/fullscreen_material.rs:46-98`) is a
  ready full-screen pass: a component on the camera turns it on, removing it turns it off
  (:197-201), and its pipeline follows the view's texture format (:204-206). Its schedule and
  ordering can be overridden.
- The UI pass runs after the built-in post-processing (`bevy_ui_render-0.19.1/src/lib.rs:265-272`:
  `ui_pass.after(Core2dSystems::PostProcess).before(upscaling)`), and draws into the camera's
  main texture (`render_pass.rs:58`). So the blur passes are ordered
  `.after(ui_pass).before(upscaling)`, outside `Core2dSystems::PostProcess`, to blur the UI too.
- Cameras on one window share their main textures (`bevy_render-0.19.1/src/view/mod.rs:1192-1197`),
  and a later camera with `ClearColorConfig::None` loads instead of clearing (:1238-1242), so it
  draws on top of the blurred result; later cameras are blitted with alpha blending
  (`bevy_core_pipeline-0.19.1/src/upscaling/mod.rs:61-79`).
- UI picking tests every camera on the window and ranks each camera's hits by its order
  (`bevy_ui-0.19.1/src/picking_backend.rs:123-131`, :271-277), so the floating camera's panels
  win over the screens below, and a full-window backdrop blocks clicks to them.
- `bevy_post_process::gaussian_blur::gaussian_blur` is a public WGSL function
  (`bevy_post_process-0.19.1/src/gaussian_blur.wgsl:6`, :26) taking a blur diameter and a
  direction; our shader is a call to it, once per direction.

### The three settings B needs right

1. The screen camera carries `IsDefaultUiCamera`. Without it, every root without a
   `UiTargetCamera` goes to the highest-order camera, the floating one
   (`bevy_ui-0.19.1/src/ui_node.rs:2990-3008`).
2. The floating camera uses `ClearColorConfig::None`. Anything else wipes the screens.
3. Both cameras have the same HDR and MSAA settings, or they stop sharing textures. Both are
   left at their defaults.

## Structure

### `Floating`, the one marker

Every root that floats over a screen carries `Floating`. That is all a panel does to take part:

- an observer on `Add<Floating>` inserts `UiTargetCamera(floating camera)` on it;
- `update_blur` asks "is any `Floating` alive?" and moves the screen camera's blur strength
  toward full or zero.

`overlay_root()`, the pause menu and `dialog()` carry it. `blur.rs` knows nothing about overlays,
pause or dialogs, and a future floating thing needs only the marker.

### `src/blur.rs`, a shared file

At the top of `src/` because three features float (overlays, the pause menu, dialogs).

| Item | What it is |
| --- | --- |
| `FloatingCamera` | marker on the second `Camera2d`, order 1, `ClearColorConfig::None` |
| `Floating` | the marker above |
| `BlurHorizontal`, `BlurVertical` | the two `FullscreenMaterial`s, each a uniform with the blur diameter; same shader, a different direction |
| `BlurStrength` | the current 0.0 to 1.0 on the screen camera, eased by `update_blur`; at 0 both materials are removed, so no pass runs |
| `assets/shaders/blur.wgsl` | calls `gaussian_blur` with the material's diameter and direction |

### Where the style lives

The palette, the type sizes and the button kinds stay in `src/ui.rs`, the file that already
holds the palette and is the one every screen imports. A game made from the template recolors
by editing one `Config` section.

## Style

### Palette

| Constant | Color | Use |
| --- | --- | --- |
| `SURFACE` | `#16171B` at 85% | panel cards |
| `BUTTON_FILL` | white at 4% | button at rest |
| `BUTTON_FILL_HOVER` | white at 9% | button under the pointer |
| `BUTTON_FILL_PRESSED` | black at 20% | button held down |
| `BORDER` | white at 12% | buttons, panels, fields |
| `ACCENT` | `#F2B544` | hover border and glow, checked controls, focused field |
| `DANGER` | `#E05C5C` | Delete, and Yes on questions that destroy something |
| `TEXT` | `#EDEDF0` | main text |
| `TEXT_MUTED` | `#9EA0A8` | secondary text: a save's details, "No saves yet" |
| `SCREEN_DIM` | black at 35% | over the blur, under a card |

The fullscreen checkbox loses its red "off": off is `BORDER` on `BUTTON_FILL`, on is `ACCENT`.

### Type

| Constant | Size | Use |
| --- | --- | --- |
| `TITLE_FONT_SIZE_PX` | 44 | the game's name on the menu |
| `HEADING_FONT_SIZE_PX` | 26 | a panel's title |
| `BODY_FONT_SIZE_PX` | 18 | buttons and text |
| `SMALL_FONT_SIZE_PX` | 14 | details, the version, the save status line |

### Buttons

```
 rest      ╭────────────────────────╮   white 4% fill, white 12% border
           │  New Game              │
           ╰────────────────────────╯
 hover     ╭━━━━━━━━━━━━━━━━━━━━━━━━╮   amber border, 9% fill,
           ┃  New Game              ┃   soft amber glow (BoxShadow)
           ╰━━━━━━━━━━━━━━━━━━━━━━━━╯
 pressed   darker fill, no glow
```

- `BorderRadius::MAX`, a 1 px border, 44 px tall. The width comes from the caller instead of a
  fixed 100 px: 260 in the menu and pause columns, 140 in dialogs, the list's width for saves.
- The label sits 20 px from the left in the menu and pause columns, centered elsewhere.
- `ButtonKind` is a component on the button:
  - `Primary`, the default;
  - `Quiet`, no border until hovered, for Close and No;
  - `Danger`, red border and glow on hover.
- `button(key)` stays as the short form of a Primary button. The kind is a component rather than
  three functions because `paint_buttons` has to know it to pick the hover colors.

## Layout

### Menu

```
 ┌──────────────────────────────────────────────────────┐
 │░░░░░░░                                               │
 │░ BEVY STARTER ░        (background art)              │
 │░ ╭ New Game    ╮                                     │
 │░ ╭ Continue    ╮                                     │
 │░ ╭ Load Game   ╮                                     │
 │░ ╭ Settings    ╮                                     │
 │░ ╭ Credits     ╮                                     │
 │░ ╭ Quit        ╮                                     │
 │░░░░░░░                                       v0.1.0  │
 └──────────────────────────────────────────────────────┘
```

- The column stays on the left, its padding grows from 32 to 80 px, with the game's name above
  it at `TITLE_FONT_SIZE_PX`. The name is a new `GAME_TITLE` constant, also used for the window
  title (a leftover of the save game design). It is a proper noun, so it is never translated.
- Buttons 260 px wide, 10 px apart. The scrim widens a little to cover the title.
- The version in the bottom-right corner, small and muted, from `env!("CARGO_PKG_VERSION")`.

### Panel cards

One `card(width)` widget in `src/ui.rs` draws every card, so the overlays, the pause menu and
the dialog look alike and enter alike: 32 px padding, 16 px corners, `SURFACE`, a 1 px `BORDER`
and a large soft shadow. `overlay_root()` becomes the full-window `SCREEN_DIM` layer plus a
centered 520 px card. A card has:

- a header: the title at `HEADING_FONT_SIZE_PX`, then a thin divider;
- its content;
- a footer: Close (`Quiet`), aligned right.

**Settings** becomes a two-column grid:

```
  Fullscreen        [■]
  Volume            ━━━━━━━●━━━━━
  Language          ● English  ○ Português  ○ 日本語
```

The checkbox gets 4 px corners; the slider gets thinner, with the part before the thumb filled in
amber; the radios are rings, filled amber when checked.

**Save Game / Load Game:**

- The name field becomes a box: white 6% fill, 8 px corners, a `BORDER` that turns `ACCENT`
  while the field has focus.
- Each save is a two-line row: the name at `BODY_FONT_SIZE_PX`, its details underneath at
  `SMALL_FONT_SIZE_PX` in `TEXT_MUTED` ("1h 01m played · 3d ago"). `save_label` splits into the
  name and the details, and the `save-label` key goes.
- Delete is a small `Danger` button at the right of the row.
- Incompatible and unreadable files show in `TEXT_MUTED`, not as buttons.

**Credits:** the title, "Made with Bevy" and the names, centered in the card.

### Pause

The same card, 340 px wide: "Paused" and the six 260 px buttons. The save status line sits under
them at `SMALL_FONT_SIZE_PX`, amber for a save that worked and `DANGER` for one that failed. The
gameplay behind is blurred.

### Dialog

A small card, 400 px, the question centered, the answers aligned right: No (`Quiet`) then Yes.
`dialog(question, yes_kind, on_yes)` takes the Yes button's kind: `Danger` for deleting a save,
quitting without saving and overwriting a save; `Primary` otherwise. The panel under a dialog
dims; the blur stays.

## Motion

| What | How | Time |
| --- | --- | --- |
| Button colors | `paint_buttons` eases each button toward its target colors instead of jumping | 0.12 s |
| Hover glow | the amber shadow grows and fades with the color | 0.12 s |
| A card entering | rises 12 px and scales from 96% to 100%, `SmoothStep` like `fade.rs` | 0.18 s |
| Blur and dim | ramp from 0 to full when something floats, back to 0 when nothing does | 0.18 s |

Everything counts `Time<Real>`: virtual time is frozen while paused, and the pause menu's own
motion would freeze with it.

## Files

| File | Change |
| --- | --- |
| `src/blur.rs` | new, see "Structure" |
| `assets/shaders/blur.wgsl` | new |
| `src/main.rs` | `mod blur;`, its plugin; `IsDefaultUiCamera` on the screen camera; `GAME_TITLE` for the window |
| `src/ui.rs` | palette, type sizes, `ButtonKind`, the pill button, eased `paint_buttons`, `card(width)` with its entrance, the dialog built on it |
| `src/overlays.rs` | `overlay_root()` becomes the dim layer and a `card()`, with `Floating` |
| `src/overlays/settings.rs` | the grid, the restyled checkbox, slider and radios |
| `src/overlays/saves.rs` | two-line rows, the name field box, `Danger` Delete |
| `src/overlays/credits.rs` | the card's header |
| `src/screens/menu.rs` | the title, the version, the column |
| `src/screens/gameplay/pause.rs` | the card, the status line's colors, `Floating` |
| `assets/locales/*/main.ftl` | `save-label` goes |
| `README.md`, `docs/flow.md`, `docs/template.md` | the look, the two cameras, `Floating`, where to recolor |

## Tests

Shaders and looks are not something `cargo test` can see; `just run` is how those are checked.
The logic behind them is pure functions, tested in their files:

| Function | Where | Guarantees |
| --- | --- | --- |
| `button_colors(kind, hover, pressed)` | `ui.rs` | rest, hover and pressed for each kind; pressed wins over hover |
| `approach(current, target, step)` | `blur.rs` | moves toward the target and never past it, from either side |
| `entrance(t)` | `ui.rs` | starts lowered and small, ends exactly in place, never overshoots |
| `save_details` | `saves.rs` | the existing play time and age cases, in English and Japanese |

## Implementation order

Each step passes `just check` and has a *Try*:

1. **Two cameras and `Floating`.** Nothing changes on screen.
   *Try:* every button, overlay, dialog and the pause menu still clicks as before.
2. **The blur.** Shader, materials, `update_blur`.
   *Try:* opening Settings blurs the menu, buttons included; closing it unblurs smoothly; the
   pause menu blurs gameplay.
3. **Palette, pill buttons, `ButtonKind`**, without motion yet.
   *Try:* every button is a pill, Delete is red on hover.
4. **Button motion**: eased colors and glow.
5. **Cards**: overlays, pause and dialog, with the entrance.
   *Try:* every panel is a card that rises in; a dialog over Load Game dims the card under it.
6. **Menu**: title, version, column.
7. **Settings controls**: checkbox, filled slider, radios.
8. **Saves**: two-line rows, the name field box.
   *Try:* a save named "ação 東京" lists on two lines in all three languages.
9. **Docs.**

## Open questions

None: the approach and the five sections were approved in conversation on 2026-10-02.

## What changed while building it

1. **Our own blur, not Bevy's `gaussian_blur`.** `FullscreenMaterial` binds a default sampler,
   which filters nearest, and `gaussian_blur` relies on linear filtering to read two texels per
   sample. `blur.wgsl` instead reads whole texels with `textureLoad` over three standard
   deviations, which needs no filtering at all. One shader serves both passes; the direction is
   in each material's uniform.
2. **The dim is per panel, not in the shader.** Each `floating_root` carries its own `SCREEN_DIM`
   background, brought in by `animate_entrances` with its card. A shader dim would only darken
   the screens, so an overlay opened over the pause menu would sit beside an undimmed pause card;
   per panel, the card underneath darkens too.
3. **`approach` lives in `fade.rs`**, not `blur.rs`: the blur and the buttons both ease with it,
   and `fade.rs` is the shared file about timing.
4. **The entrance is the `Fade` type**, with an infinite hold (`ENTRANCE`), so `entrance(t)`
   became `entrance_transform(progress)`, the pure part that places the card.
5. **`BlurStrength` is the screen camera's marker.** `main.rs` spawns the screen camera with it
   and `IsDefaultUiCamera`; `update_blur` finds the camera by it.
6. **Three button functions, not a width argument.** `button(key, kind)` (centered label, min
   120 px), `menu_button(key)` (Primary, label left, as wide as its column) and
   `row_button(content)` (any bundle, for list rows). Widths come from the parent column instead
   of each call. The localization's `text_button` is gone.
7. **`save_label` became `save_name` and `save_details`**, and the `save-label` key became
   `save-details`, holding only the play time and the age.
8. **New pieces the sections implied:** `label(key)`, `heading(key)`, `font(size)` and
   `FIELD_FILL` in `ui.rs`; `overlay_card()` and `close_footer()` in `overlays.rs`;
   `setting_row` and `slider_track` in `settings.rs`; `paint_save_name_field`, which reads
   `InputFocus`; a `credits-title` key, so the Credits card does not borrow the menu's.
9. **The New Save button sits beside the name field**, in one row, instead of under it.
10. **The ICU4X warning on Japanese text needed `icu_provider`'s `logging` feature.** Without it,
    debug builds `eprintln!` the warning directly (`icu_provider-2.3.1/src/lib.rs:192-203`), and
    the `LogPlugin` filter in `main.rs` never sees it. Found while running this work; not part of
    the design, but in the same diff.
