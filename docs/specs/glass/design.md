# Glass: a blurred copy behind every panel, cheap enough for any game

Status: **designed** — written and approved 2026-11-08, not started.
Date: 2026-11-08
Bevy: 0.20.0

Row 0 of `docs/roadmap/roadmap.md`, first of its three specs (glass, look, pieces). The
roadmap's "The look, as agreed so far (row 0)" holds the measurements and the reasons behind
most of what follows; this design does not repeat them, it builds on them.

## Goal

Today a menu over the game blurs the whole screen with a full-resolution Gaussian
(`src/ui/blur.rs`, `assets/shaders/blur.wesl`) and draws its card as a flat, nearly opaque
surface. This design replaces that blur with the technique the Liquid Glass look needs: a
blurred copy of what is behind each layer of floating UI, made on a small image with a dual
Kawase blur, which every panel of that layer reads inside its own rounded rectangle. It changes
how the existing pause menu, overlays and dialogs are drawn, not what they contain: the colours,
the icon font and the restyled screens are the next spec's.

Done looks like this: `just run`, New Game, walk a little, Esc. The frozen level is blurred and
dimmed behind the pause menu, as today, and the pause card is a thicker glass over it, so the
card still stands out. "Quit without saving?" over the pause menu blurs the pause menu too.

## Decided

| Question | Decision |
| --- | --- |
| What a modal blurs | The whole screen behind it, as today, plus `SCREEN_DIM`. A modal is anything built from `floating_root()`: the pause menu, every overlay, every dialog |
| The modal's card | Glass one level more blurred than the backdrop, so it stands out from a background that is already blurred: the roadmap's "thicker glass" |
| Stacked modals | One copy per layer: each modal blurs everything below it, the modals under it included. The dialog over the pause menu blurs the game and the pause menu |
| The blur | A dual Kawase on a copy halved until it reaches a fixed height, the same at every resolution, so the number of levels comes from the screen's size. That height is tuned by eye so the backdrop matches today's blur at 1080p (Claude's call, see below) |
| Where it blurs | Inside a list of regions, each the rectangle of a group of glass nodes plus a margin of the blur's radius. Glass nodes that touch or overlap share one region. A modal's backdrop makes its region the whole screen |
| How glass comes in | With its node's opacity over `ENTRANCE`, the Kawase at full strength from the first frame. `BLUR_RAMP_SECS` goes |
| Blurring once | While nothing behind a layer moves, its copy is made once and reused, told by explicit signals, never by comparing images |
| Reduce transparency | A setting in this spec: no copies are made, the backdrop is the dim alone and every card a solid fill |
| Corner panels | The technique supports them (regions smaller than the screen), but no committed code uses one yet: the first is the HUD of the third spec. A throwaway test panel proves them on `3d/player` |
| Where it is built | Tried uncommitted on `3d/player`, committed on `main`, merged down (roadmap, "Tried on `3d/player`, committed on `main`") |

## Must not

The user named four defects that rule out a build even when it technically works. Each step
checks the ones it can affect, by eye, with `just run`:

- **Flicker.** The edges of what is behind must not shimmer when the camera moves. Every
  halving averages the pixels it merges (the Kawase descent's bilinear reads do), never skips
  them. Checked on the test panel on `3d/player`, walking and turning.
- **A hitch.** No frame may stall when a modal opens. Textures are made when the window
  appears or resizes, never when a layer opens, and the shaders' pipelines are queued early
  enough that the first pause finds them compiled: Bevy compiles pipelines in the background,
  so a pass whose pipeline is not ready is skipped, and the first pause would show without
  glass for a few frames.
- **Wrong colour.** The blurred copy keeps the colours of the image it copies: the same texture
  format as the camera's image (HDR in `3d/player`'s gameplay, not on `main`), and edges clamped
  so the screen's border does not darken. The only darkening is `SCREEN_DIM`, on purpose.
- **Text that is hard to read.** With today's palette, a card over the brightest background the
  template has (the 3D level's sky) still reads as easily as today's. The next spec retunes the
  colours; this one must not hand it an unreadable card.

A blocky look from the small copy stretched back up was named as tolerable.

## Out of scope

- **The look** (spec 2): the Liquid Glass colours, the white accent, the Lucide font, keycaps,
  capsules, the specular rim and the darkened inner edges, and the main menu, which has no card
  (its buttons sit on the screen) and so gets its glass when the next spec restyles it.
- **The pieces** (spec 3): the HUD, the popover, the bar, the list, the alert, and with them the
  first committed corner panel.
- **Exit animations**, as in the `ui-style` spec: a layer leaves with `DespawnOnExit`.
- **Refraction**, left out by the roadmap.
- **Stopping the game's camera under the pause menu.** It is worth doing (the paused game gets
  lighter than the running one), but the camera is the level's, in `3d/base`'s `playroom.rs`, so
  it is a commit on `3d/base` after this spec merges down, not part of this spec.

## Facts this rests on (read in the 0.20.0 source)

- **A UI material gets what glass needs.** `bevy_ui_render`'s `UiVertexOutput`
  (`src/ui_vertex_output.wesl`) carries the node's `size` in pixels, its four corner radii
  (`border_radius_x`, `border_radius_y`) and `@builtin(position)`, the fragment's position on
  the screen. A fragment can therefore read the layer's copy at its own screen coordinates and
  cut the rounded corners itself.
- **The UI draws in `ui_pass`**, a system in the `Core2d` and `Core3d` schedules, ordered
  `.after(Core2dSystems::PostProcess).before(upscaling)` (`bevy_ui_render/src/lib.rs`). A system
  that makes a layer's copy runs in the same schedule, before that camera's `ui_pass`.
- **`FullscreenMaterial` writes back to the camera's own image**, which is why today's blur uses
  it and why the copies cannot: they write to textures of their own, sized to each level, and
  each pass limits where it draws to the regions (a scissor rectangle or a viewport). The passes
  are written by hand, as render systems.
- **There is no group opacity in the UI**: nothing in `bevy_ui` 0.20 fades a node with its
  children, as the `ui-style` spec already found in 0.19. "Glass comes in with its node's
  opacity" is the glass material's own alpha, the backdrop's and the card's. The card's content
  keeps today's entrance, rising and scaling in with `UiTransform`.
- **Cameras that share the window share one image only while their format and MSAA match**, as
  `match_ui_cameras_to_playroom_camera` in `3d/base` documents. The floating camera already relies
  on that. Any camera this design adds must too.

## Structure

### Layers

A layer is one of the named `*_Z_INDEX` roots that floats: `PAUSE_Z_INDEX` (50),
`OVERLAY_Z_INDEX` (100), `DIALOG_Z_INDEX` (200). `VEIL_Z_INDEX` is not glass. Each open layer
needs its copy taken after everything below it has drawn and before it draws itself.

Today one floating camera draws every `Floating` root. A copy per layer means that drawing is
split by layer, so a copy can be taken between two of them. The natural way, and the builder's
call, is one camera per layer, ordered by its `*_Z_INDEX`, each clearing nothing and drawing over
the image the ones below left, as the floating camera does today; a root reaches its layer's
camera through `UiTargetCamera`, as `target_floating_camera` sends it to the floating camera now.

### The copy

For each open layer, a chain of passes over the image as it stands when that layer is about to
draw:

1. **Descent**: halve the image until it reaches the target height, averaging as it goes (the
   dual Kawase's down pass, five bilinear reads).
2. **Ascent**: the Kawase's up pass, back up some of the levels (eight bilinear reads). The
   backdrop reads the ascent from the target height; the card's ascent starts one level deeper,
   so it is blurred more. The descent is shared, and the extra level is a tiny image.

In a dual Kawase the strength is how deep the descent goes, so the target height sets both the
look and the cost. Claude's call: it is a named constant tuned by eye so the backdrop looks like
today's blur (a Gaussian of sigma 9 px) at 1080p. Since sigma 9 at 1080p is a wide blur, the
target is expected to land around 135 to 270 px, two or three halvings below 1080p. The roadmap
first wrote "about 540 px", which is a single halving and much weaker than today's blur; the
roadmap's 4K estimate still holds, since its cost is the first halving, which every depth pays.
The levels' count is the number of halvings from the screen's height to the target, so the blur
covers the same share of the screen at every resolution.

The textures are made at startup and remade when the window resizes or the camera's image
format changes (entering `3d/player`'s gameplay switches it to HDR), never when a layer opens.

### Regions

Each pass draws only inside its layer's regions. A region is the screen rectangle of a group of
glass nodes of that layer, plus a margin of the blur's radius, so the blur's edges read real
neighbours. Nodes whose rectangles (with margins) touch or overlap are merged into one region;
nodes apart keep their own. The levels' count comes from the screen, never from a region, or a
small panel would blur less than a large one. A modal's backdrop covers the screen, so its
layer's one region is the whole screen; only the test panel exercises smaller ones until the
third spec.

Turning glass nodes into regions is a pure function, rectangles in, rectangles out, tested at
the bottom of its file.

### The material

One UI material draws every glass node. For a fragment it reads the layer's copy at its screen
position, from the level the node asks for (backdrop or card), mixes in the node's tint, cuts the
rounded corners from `size` and the radii, and multiplies the result's alpha by the node's
entrance progress. `floating_root()` gets it for the backdrop, carrying today's `SCREEN_DIM` as
its tint; `card()` gets it for the card, carrying today's `SURFACE`.

The layer's copy is taken before the layer draws, so it holds no `SCREEN_DIM`. Claude's call:
the card's sample gets the backdrop's dim too, before its own tint, so the card reads as the dimmed
scene seen through thicker glass, never as a brighter hole in the dim. It keeps the text over the
3D sky as readable as the dimmed backdrop makes it, which matters more once the next spec lowers
`SURFACE`'s alpha. It is one line of the shader, to revisit at `just run` if the card stops
standing out.

### Coming in

`animate_entrances` in `src/ui/panel.rs` already moves the dim and the card over `ENTRANCE`. It
moves the glass material's alpha the same way, so the backdrop and the card fade from the sharp
game into glass while the card rises, and the GPU's alpha blending is the mix. `update_blur`,
`BlurStrength` and `BLUR_RAMP_SECS` go.

### Blurring once

A layer over a frozen background makes its copy once, when it opens, and reuses it until it
closes or the window resizes. Frozen means an explicit signal, not a measurement: under
`Pause::On` the game is frozen, a layer over another layer is over a frozen one, and the main
menu's background is a still image. Anything not known to be frozen is copied every frame, which
is always correct, only slower.

### Reduce transparency

A checkbox in the settings overlay, saved in `settings.ron` next to `fullscreen`, built like the
fullscreen checkbox (`src/overlays/settings.rs`). While it is on, no copy is made and no pass
runs: the backdrop is `SCREEN_DIM` alone and every card a solid fill. Its label is a key in every
`assets/locales/<code>/main.ftl`; the words follow `CONTRIBUTING.md`'s Text rules (button and
label casing copies the English).

### Files

`src/ui/blur.rs` and `assets/shaders/blur.wesl` are replaced. The new code is the glass's: the
layers' cameras, the copies, the regions and the material. It may outgrow one file; it is split
by part, `screens.rs`-style, when a part makes sense on its own. The file names are the user's
call when the step that creates them comes.

## The test panel (never committed)

On `3d/player`, while each step is tried: one glass node in a corner of the screen during
gameplay, with the level moving behind it, the one case blurred every frame. It is measured as
the roadmap's spike measured, vsync off (`PresentMode::AutoNoVsync`) and
`RenderDiagnosticsPlugin` + `LogDiagnosticsPlugin`, reading each pass's `elapsed_gpu`. Before the
step is carried to `main`, the test panel and the diagnostics are thrown away with `git restore`.

## Tests

- **Regions**: the pure function from glass rectangles to regions. Apart stays apart; touching
  or overlapping merges; the margin is added once to a merged region, not once per node; a region
  never leaves the screen.
- **Levels**: the pure function from the screen's height to the number of levels, for 720p,
  1080p, 1440p and 4K, written over the target-height constant, so the deepest level lands
  within a halving of the target at each, and 4K takes one level more than 1080p.
- **Settings**: `settings.ron` without the new field still loads, with reduce transparency off.
- The look itself is not tested by `just check`; each step asks the user to `just run`.

## Implementation order

One commit per step, each tried on `3d/player` and committed on `main` as above. Each step
updates `docs/flow.md` and `docs/template.md` where it changes what they describe ("Two cameras
and the blur" in `flow.md` is rewritten by step 1 and kept true by the steps after).

1. **One camera per layer.** The floating camera splits into one camera per `*_Z_INDEX` layer,
   and each root reaches its layer's camera. Today's Gaussian stays on the screen camera, so
   nothing looks different yet: this step only makes room for a copy between two layers.
2. **The copy and the backdrop.** The copy (descent and ascent) for each open layer, and the
   material on `floating_root()`'s backdrop, its region the whole screen. Replaces the Gaussian:
   `blur.wesl`, `BlurHorizontal` and `BlurVertical` go. The dialog over the pause menu blurs it.
   The card is unchanged. Coming after step 1, no commit on `main` ever draws a blurred game over
   a pause menu it should have shown.
3. **The card's glass.** `card()` gets the material, reading one level deeper.
4. **Coming in.** The glass's alpha follows `ENTRANCE`; `update_blur`, `BlurStrength` and
   `BLUR_RAMP_SECS` go.
5. **Blurring once.** Frozen layers copy once.
6. **Regions, one per layer.** Passes draw inside a region: the rectangle around every glass node
   of the layer, with its margin. The test panel shows the gain.
7. **Grouping.** The pure function that merges touching nodes and keeps apart ones apart, with its
   tests.
8. **Reduce transparency.** The setting, its text keys, and the material and passes honouring it.

After the merge down, on `3d/base`: the game's camera stops drawing under the pause menu.

## What changed while building it

Nothing yet.
