# Glass: a blurred copy behind every panel, cheap enough for any game

Status: **built on `main`** — written and approved 2026-11-08; steps 1 to 4 and 7 built by 2026-11-16. The regions (steps 5 and 6) are put off to the Pieces spec, and step 9, on `3d/base`, is still to build (see "What changed while building it").
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

Built on `main` in steps 1 to 4 and 7 of `docs/roadmap/glass-tasks.md`, between 2026-11-08 and
2026-11-16. The step numbers below are that file's: its steps 4 to 7 are this spec's 5 to 8,
since "Coming in" was folded into the steps before it.

- **Measured in 2.12, at 1920 × 1080, two levels.** A pass going down took about 0.009 ms and
  one coming up about 0.013 ms on the GPU, about 0.03 ms for the whole chain, against the
  0.16 ms of the Gaussian it replaced. Against the Must not list: smooth, no flicker walking
  past the test panel, and the strength matched the old blur, so `GLASS_TARGET_HEIGHT_PX`
  stayed at 270.
- **"Coming in" is not a step of its own.** Step 2 removes the Gaussian, and with it the only
  thing `update_blur` drives, so the backdrop's alpha follows `ENTRANCE` from step 2 and the
  card's from step 3; otherwise two commits on `main` would pop the blur in. The spec's step 4
  is folded into them, and its steps 5 to 8 become steps 4 to 7 here.
- **The copies are always `Rgba16Float`**, whatever the camera's format. Sampling an sRGB image
  returns linear colour, so a 16-bit float copy keeps the colours of either image ("Wrong colour"
  in Must not), and its textures need no remaking when gameplay switches the cameras to HDR. The
  copies are small, so the wider format costs next to nothing.
- **The copies are a component on each layer camera, not a `GlassCopies` resource** (decided
  while building 2.4). They belong to one camera, and `CONTRIBUTING.md` puts state that belongs
  to an entity on it: the render system of 2.8 then gets a camera's copies in the same query as
  the camera, with no lookup by `GlobalZIndex`, and 2.5 needs only `ExtractComponent`.
- **`KAWASE_OFFSET` is `KAWASE_OFFSET_TEXELS`** (decided while building 2.1). `CONTRIBUTING.md`
  puts the unit in a constant's name, and an offset in texels of the image a pass reads is easy
  to mistake for one in screen pixels, which would widen the blur at every level.
- **`LayerCamera` is not extracted** (decided in 2.14's review). Only layer cameras carry
  `GlassCopies`, so the render system's `ViewQuery` on it alone already skips every other
  camera, and 2.5's second `ExtractComponent` had nothing left to read it.
- **The glass material's pipeline is not compiled ahead** (found in 2.14's review, left for
  later). Only the two Kawase pipelines are queued at `RenderStartup`; Bevy specializes the
  `GlassMaterial` pipeline the first frame a glass shows and skips drawing it until it is
  ready, so the first pause may bring its glass in a few frames late. Not seen while testing,
  so it waits for a hitch someone can see.
- **The level lists stay inside `generate_level_images`** (decided in step 3). The review of
  2.14 meant them to become a pure, tested function in 3.1, which changed them for the card.
  They are three ranges over `levels`, one line each: a function around them would be more
  code than what it protects.
- **A resize points the open glass at the new copies** (found while testing 2.12). A resize
  remakes every `GlassCopies` with new handles, and a root's material kept the old ones, which
  nothing writes any more: the glass of an open panel froze, stretched, until it was reopened,
  and a panel that never closes (a HUD, the prop editor) would stay frozen. An observer on
  `Insert<GlassCopies>` now hands every open root of that layer the new copy, and the choice of
  which image the glass reads is `GlassCopies::backdrop_copy`, shared by both places (named
  `glass_copy` until 3.2 gave the card a copy of its own, `card_copy`). Since 3.2 the same
  observer, `retarget_open_glass`, hands each root's cards the new card copy too.
- **A card gets its glass from an observer, not a system** (decided while building 3.2). An
  observer on `Add<Card>` runs as the card is spawned, where a system on `Added<Card>` would
  wait for the next `Update` and could show the card a frame without its glass; and it matches
  `target_layer_camera`, which does the same for the roots.
- **The card's two tints are one colour** (decided while building 3.2). A material replaces what
  is under it, so the card's glass would lose the dim its root lays over the backdrop, and the
  shader mixes in a single tint. The pure `lay_over(top, bottom)` in `blur.rs` gives the one
  colour that looks like `SURFACE` laid over `SCREEN_DIM`, tested against laying the two in turn.
- **The corners are cut with one radius, not Bevy's `sd_rounded_box`** (decided while building
  3.2). Bevy's takes a radius per corner and ellipses, and calls two more functions a material
  cannot import: some 40 lines copied for a card whose four corners share one round radius.
  `glass.wesl` has the two-line box of a single radius, the top left corner's, and a comment
  there says what to copy the day a node's corners differ.
- **The card's flat fill went after 3.3, not in 3.2** (decided while building 3.2). The card's
  glass starts at alpha 0 and 3.3 is what fades it in, so taking `BackgroundColor(SURFACE)` off
  first would have left the card without a fill in between.
- **The features say when the background is still; the glass does not ask** (decided while
  building 4.1). Task 4.1 had `blur.rs` read `State<Pause>` and `State<Screen>`, which would
  make `ui` depend on the screens for the first time, and a game with a still moment of its own
  (an inventory that stops the game) would have to edit the glass. Instead `blur.rs` holds a
  `FrozenBackground` resource, off by default, and hands out `freeze_background` and
  `unfreeze_background`: `pause.rs` runs them on entering and leaving `Pause::On`, as it
  registers its layer in `GlassLayers`.
- **The menu is still only once its veil is gone** (found while building 4.1). The opening veil
  fades for 0.8 s over the menu, so an overlay opened under it would keep a blur of the menu
  half covered. `menu.rs` sets `FrozenBackground` from whether a `Veil` exists, each frame.
- **A layer waits for its own entrance too** (decided while building 4.1). Its own root does not
  reach its copy, but the 0.18 s it takes cover what is still settling under it: the button
  that opened it eases out of its hover for 0.12 s.
- **A change under a frozen layer is not seen until it closes** (accepted in 4.1). Changing
  the language in Settings rewrites the menu's words behind it, and the frozen blur keeps the
  old ones: blurred words cannot be read, and closing the overlay shows the new ones.
- **The passes remember the copy they made; the game does not tell them to copy** (decided
  while building 4.2). Task 4.2 had an extracted component saying whether to copy this frame,
  set by the main world. But only the passes know whether a copy was really made: a pipeline
  still compiling or an image not on the GPU yet ends the frame without one, and a main world
  that had already counted it would leave the layer frozen on an empty copy. So the main
  world only says whether the layer is frozen (`FrozenLayer`), and `blur_layer_backdrops`
  keeps, per layer camera, the copy it last finished: a frozen layer whose copy is still that
  one is skipped. A resize hands out new images, so it blurs again with no signal of its own.
- **`FrozenLayer` holds a `bool` and never leaves the camera** (decided while building 4.1).
  Bevy's extraction does not take a component off the render world when it leaves the main
  one, so a marker inserted and removed would freeze the layer there for good.
- **Measured in 4.4 by the age of each pass, not by a repeated value.** A throwaway system
  logged, once a second, how long ago each Kawase pass was last timed. Under the pause menu the
  passes ran for the 0.2 s of the entrance and then stopped: the age grew a second per line
  (0.8, 1.9, 2.9 ... 7.9 s) until the menu closed, and opening it again ran them for another
  0.2 s. Each pass took about 0.003 ms going down and 0.009 ms coming up on the GPU.
- **Steps 5 and 6, the regions, are put off to the Pieces spec** (decided on 2026-11-16, before
  starting 5.1). Every floating root is a modal whose backdrop covers the screen, so each
  layer's region would be the whole screen and the two steps would save nothing; only a test
  panel could exercise them. The whole chain measured about 0.03 ms a frame on the full
  screen. They come with the first glass panel smaller than the screen, the dev tools' own,
  which blurs every frame over a running game and gives them something to measure. Their
  tasks stay below as written, for that spec to pick up.
- **Reduce Transparency is a `flat` uniform of the glass, not another kind of node** (decided
  while building 7.5). The alpha the material already had makes a card solid, as 7.5 said, but
  not a backdrop: its shader lays the dim over the blurred copy, and with no copy the dim has
  to be drawn alone, over the sharp screen. One `flat` uniform does both: the shader draws the
  tint alone, as opaque as the tint's alpha, so the dim stays see-through and the card's tint
  is the surface made opaque. The entrance needs no second path, since the same alpha fades it.
- **Turning Reduce Transparency on forgets the copies kept** (found while building 7.5). A
  layer frozen before the setting went on would otherwise, when it goes off, show the copy from
  back then: the passes had stopped, so nothing had remade it.
- **The two checkboxes share `checkbox()` and `set_checkbox`** (decided while building 7.3).
  Task 7.3 built the new one like the fullscreen one; with two, the bundle and the painting
  moved into helpers of `settings.rs` instead of being written twice.
