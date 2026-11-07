# Glass: tasks

Roadmap row 0, first of its three specs, built on `main`. The decisions and their reasons live
in the spec, `docs/specs/glass/design.md`; this file only breaks its implementation order into
tasks small enough for one sitting each, with the facts each task needs.

## How to read this

- **Step** = one commit. Its title is the commit message.
- **Who**: *you* writes the Rust and the shaders; *Claude* does pure moves, docs, doc comments
  and separators, and hands over the lines of a render API you have never seen, saying why;
  *both* is a review together (the checklist in `CLAUDE.md`: every name, the order of sections
  and systems, `just check`).
- **Facts**: the names you cannot guess (types, methods, modules), checked in the Bevy 0.20.0
  source. Assembling them is the task.
- **Where**: every step is written and tried uncommitted on `3d/player`, then carried to `main`
  (`git stash`, `git switch main`, `git stash pop`), checked there too (`just check`, `just run`),
  committed, and merged down. Step 9 alone is `3d/base`'s.
- A task is done when `just check` passes, except where it says the next task makes it pass.
- Mark a task done by changing its `[ ]` to `[x]`.

## Two departures from the spec, to record in its "What changed while building it"

- **"Coming in" is not a step of its own.** Step 2 removes the Gaussian, and with it the only
  thing `update_blur` drives, so the backdrop's alpha follows `ENTRANCE` from step 2 and the
  card's from step 3; otherwise two commits on `main` would pop the blur in. The spec's step 4
  is folded into them, and its steps 5 to 8 become steps 4 to 7 here.
- **The copies are always `Rgba16Float`**, whatever the camera's format. Sampling an sRGB image
  returns linear colour, so a 16-bit float copy keeps the colours of either image ("Wrong colour"
  in Must not), and its textures need no remaking when gameplay switches the cameras to HDR. The
  copies are small, so the wider format costs next to nothing.

## The layers at a glance

| Layer | `GlobalZIndex` | Camera `order` |
|-------|----------------|----------------|
| screens and the game | — | 0, as `main.rs` spawns it |
| pause menu | `PAUSE_Z_INDEX` (50) | 50 |
| overlays | `OVERLAY_Z_INDEX` (100) | 100 |
| dialogs | `DIALOG_Z_INDEX` (200) | 200 |

A layer camera's `order` is its `GlobalZIndex`, so the order follows from the constants already
listed in `docs/flow.md` and is never restated.

## Step 1: `refactor: draw each floating layer with its own camera`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 1.1 `GlassLayers` resource in `src/ui/blur.rs`: the list of floating layers | you | `#[derive(Resource, Default)] struct GlassLayers(Vec<GlobalZIndex>)`, `pub(crate)` |
| [ ] | 1.2 `pause.rs`, `overlays.rs` and `panel.rs` register their layer from their plugin | you | `app.init_resource::<GlassLayers>()` (does nothing if it already exists), then `app.world_mut().resource_mut::<GlassLayers>().0.push(PAUSE_Z_INDEX)` |
| [ ] | 1.3 `spawn_floating_camera` becomes `spawn_layer_cameras`: one camera per registered layer, as the floating camera is today, with a `LayerCamera(GlobalZIndex)` marker | you | `GlobalZIndex(pub i32)`; `order: z.0 as isize`; keep `clear_color: ClearColorConfig::None` and `RenderLayers::none()`, for the reasons their comments give |
| [ ] | 1.4 `target_floating_camera` sends a new `Floating` root to the camera of its own layer | you | `On<Add<Floating>>` runs once the whole bundle is in, so the root's `GlobalZIndex` is there: `Query<&GlobalZIndex>`, then the `LayerCamera` with the same value |
| [ ] | 1.5 `docs/flow.md` "Two cameras and the blur" and `docs/template.md`: one camera per layer | Claude | |
| [ ] | 1.6 `just run` on `3d/player`, then on `main`: pause; Settings and Load over the pause menu; "Quit without saving?" over it; clicks reach only the top layer; everything looks as before; review; commit | both | `playroom.rs` already gives every `Camera2d` `Hdr` and `Msaa::Off`, the new cameras included |

## Step 2: `feat: blur behind each layer with a dual Kawase`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 2.1 Config: `GLASS_TARGET_HEIGHT_PX` (start at 270, tuned in 2.12) and `KAWASE_OFFSET` (1.0) | you | |
| [ ] | 2.2 `fn levels_for_height(screen_height_px: u32) -> u32`: how many halvings bring the screen down to the target | you | stop at the first halving at or below the target; `saturating_sub`-style care at the small end, so a tiny window gives 1, never 0 |
| [ ] | 2.3 Tests for 2.2, at 720, 1080, 1440 and 2160 px tall, written over the constant | you | `deepest_level_lands_within_a_halving_of_the_target` (`target / 2 < height >> levels <= target`); `a_4k_screen_takes_one_level_more_than_1080p` (a Rust name cannot start with a digit) |
| [ ] | 2.4 The copy textures: per layer, one `Image` per descent level (one more than the backdrop's, for the card in step 3) and one per ascent level, all `Rgba16Float`, held in a `GlassCopies` resource | you | `Image::new_target_texture(width, height, TextureFormat::Rgba16Float, None)`; `ResMut<Assets<Image>>`; made at `Startup` and remade on `MessageReader<WindowResized>` (`bevy::window`) |
| [ ] | 2.5 Hand `GlassCopies` and `LayerCamera` to the render world | Claude hands the lines | `#[derive(ExtractResource)]` + `ExtractResourcePlugin` (`bevy::render::extract_resource`); `LayerCamera` gets `ExtractComponent` with `#[extract_app(RenderApp)]`, as `BlurHorizontal` has today |
| [ ] | 2.6 `assets/shaders/kawase_down.wesl` and `kawase_up.wesl` | you | the fullscreen vertex is `bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput`, as `blur.wesl` imports it; `textureSample` with a filtering sampler, not `textureLoad`: the bilinear reads are the trick. Formulas below |
| [ ] | 2.7 The two pipelines, queued at `RenderStartup` so they are compiled before the first pause ("A hitch" in Must not) | Claude hands the lines | mirror `init_pipeline` in `bevy_core_pipeline/src/fullscreen_material.rs`: `FullscreenShader::to_vertex_state()`, a layout of `texture_2d` (filterable) + `sampler(Filtering)` + `uniform_buffer`, one `ColorTargetState { format: TextureFormat::Rgba16Float, blend: None, .. }`, `PipelineCache::queue_render_pipeline` |
| [ ] | 2.8 The render system that runs the chain for each active layer camera, in `Core2d`, `.before(ui_pass)`, so it reads what the cameras below left and the layer's own UI is not in it yet | Claude hands the lines, you write the loop over the levels | mirror `fullscreen_material_system`: `ViewQuery<(&ViewTarget, &LayerCamera)>`, `RenderContext`, `Res<RenderAssets<GpuImage>>`; the source of the first halving is `view_target.main_texture_view()`; one `diagnostics.pass_span` per pass, so the test panel can measure it |
| [ ] | 2.9 A layer camera with no `Floating` root is not active, so a closed layer runs no pass | you | `Camera { is_active, .. }`; a system after the roots spawn and despawn |
| [ ] | 2.10 `GlassMaterial` and `assets/shaders/glass.wesl`: reads the layer's copy at the fragment's screen position, lays the tint over it, and fades by the node's alpha | you | `#[derive(AsBindGroup, Asset, TypePath, Clone)]` + `impl UiMaterial` (`bevy::ui_render`), `UiMaterialPlugin::<GlassMaterial>`; `#[uniform(0)]` tint and alpha, `#[texture(1)] #[sampler(2)]` the copy; in the shader `@group(0) @binding(0) var<uniform> view: View;` from `bevy_render::view::View`, and the screen position is `in.position.xy / view.viewport.zw`. The UI material pipeline already alpha-blends |
| [ ] | 2.11 The backdrop: the observer from 1.4 also gives the root `MaterialNode(GlassMaterial { tint: SCREEN_DIM, alpha: 0.0, copy })`, and `animate_entrances` moves its alpha where it moved the dim's. `BlurHorizontal`, `BlurVertical`, `BlurStrength`, `update_blur`, `BLUR_RAMP_SECS`, `BLUR_SIGMA_PX` and `blur.wesl` go | you | each root gets its own material (`Assets<GlassMaterial>::add`), since each fades on its own; `Assets::get_mut` in `animate_entrances`; `MaterialNode` is `bevy::ui_render::prelude::MaterialNode`. The backdrop has no corners to cut |
| [ ] | 2.12 The test panel on `3d/player`, never committed: a glass node in a corner during gameplay, on a test layer of its own; tune `GLASS_TARGET_HEIGHT_PX` by eye against today's blur (`git stash` it, look, pop it); measure `elapsed_gpu` | both | vsync off and the diagnostics plugins, as in the roadmap's spike |
| [ ] | 2.13 `docs/flow.md` (the cameras section, the shared-files table) and `docs/template.md` (the shader files) | Claude | |
| [ ] | 2.14 `just run` on both branches, against Must not: no flicker walking past the test panel, no hitch on the first pause, colours unchanged, text readable over the sky; the dialog over the pause menu blurs the pause menu; review; throw the test panel away; commit | both | |

**The Kawase formulas** (Marius Bjørge's dual filter, SIGGRAPH 2015), with `t` the *source*
texture's texel size (`1.0 / vec2<f32>(textureDimensions(source))`) times `KAWASE_OFFSET`:

- **Down**, writing a texture half the size: `(4 * s(uv) + s(uv - t) + s(uv + t) +
  s(uv + vec2(t.x, -t.y)) + s(uv - vec2(t.x, -t.y))) / 8`.
- **Up**, writing a texture twice the size: the four reads at `uv ± vec2(2 * t.x, 0)` and
  `uv ± vec2(0, 2 * t.y)` weigh 1, the four at `uv ± t` and `uv ± vec2(t.x, -t.y)` weigh 2,
  all over 12.

The ascent stops at half resolution (level 1), which the material samples bilinearly; stopping
a level lower is tried with the test panel and kept only if it passes Must not.

## Step 3: `feat: make the cards glass`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 3.1 The card's chain: the descent's extra level and an ascent of its own from there | you | the descent is shared with the backdrop; same shaders, same pipelines |
| [ ] | 3.2 A system gives each new `Card` a `GlassMaterial` reading its layer's card copy, tinted with `SURFACE` over the backdrop's dim, with its corners cut | you | `Query<(Entity, &ChildOf), Added<Card>>`, the layer from the parent's `GlobalZIndex`; the corners from `in.size`, `in.border_radius_x`, `in.border_radius_y` and `in.uv`: copy `sd_rounded_box` and the `antialias` it uses from `bevy_ui_render/src/ui.wesl` into `glass.wesl`, since a material cannot import them |
| [ ] | 3.3 `animate_entrances` fades the card's material with the root's | you | |
| [ ] | 3.4 `docs/flow.md` | Claude | |
| [ ] | 3.5 `just run` on both branches: the card stands out from the backdrop, its text reads over the sky; review; commit | both | |

## Step 4: `perf: blur a frozen layer once`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 4.1 A layer is frozen while the game is paused or the screen is the menu, and once every layer below it has finished its entrance | you | `Res<State<Pause>>`, `Res<State<Screen>>`; the entrances are the `Entrance` components of the roots below |
| [ ] | 4.2 A frozen layer whose copy is made skips its chain until it closes or the window resizes | you | an extracted component on the layer camera saying whether to copy this frame; a resize remakes the textures and so copies again |
| [ ] | 4.3 `docs/flow.md` | Claude | |
| [ ] | 4.4 `just run` with the diagnostics on `3d/player`: under the pause menu the passes log once and then repeat their last value (the spike's tell for a pass that did not run); review; commit | both | |

## Step 5: `perf: blur only inside each layer's region`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 5.1 Each layer's region: the rectangle around its glass nodes, plus the margin, kept inside the screen | you | `ComputedNode::size()` and `UiGlobalTransform` (`bevy::ui`) give a node's size and centre in physical pixels; the margin follows from the levels' count (each level's reads reach two of its texels), as a derived constant |
| [ ] | 5.2 Hand the regions to the render world; every pass draws only inside its region, scaled down to that level | Claude hands the lines | `render_pass.set_scissor_rect(x, y, width, height)`, rounded outwards at each level |
| [ ] | 5.3 `just run` with the test panel: the passes' `elapsed_gpu` drops with the panel's share of the screen; no smeared edge on the panel; review; commit | both | |

## Step 6: `perf: merge glass regions that touch`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 6.1 `fn merge_regions(rects: &[URect], margin_px: u32, screen: UVec2) -> Vec<URect>` | you | `URect` has `union`, `intersect` and `is_empty`; merge until no two touch |
| [ ] | 6.2 Tests: `rects_apart_stay_apart`, `touching_rects_merge`, `overlapping_rects_merge`, `margin_is_added_once_to_a_merged_region`, `region_never_leaves_the_screen` | you | |
| [ ] | 6.3 The render system draws each pass once per region | you | |
| [ ] | 6.4 `just run` with two test panels in opposite corners, then side by side; review; commit | both | |

## Step 7: `feat: add a reduce transparency setting`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 7.1 `reduce_transparency: bool` in `SettingsFile`, defaulting to off for an older `settings.ron` | you | `#[serde(default)]` on the field |
| [ ] | 7.2 Test: `settings_without_reduce_transparency_load_with_it_off` | you | `ron::from_str` on a string without the field |
| [ ] | 7.3 The checkbox, built like the fullscreen one | you | `FullscreenToggle`, `toggle_fullscreen`, `update_fullscreen_checkbox` in `src/overlays/settings.rs` |
| [ ] | 7.4 The key `settings-reduce-transparency` in every `main.ftl` | you | en `Reduce Transparency`, pt-BR `Reduzir Transparência`, ja `透明度を下げる` (macOS's own words) |
| [ ] | 7.5 A `ReduceTransparency(bool)` resource in `blur.rs` that settings writes; while on, no layer copies and every card is solid | you | the material's alpha uniform already exists; a solid card is `SURFACE` at alpha 1 |
| [ ] | 7.6 `docs/flow.md` | Claude | |
| [ ] | 7.7 `just run`: the box saves and comes back after a restart; with it on, the passes stop logging; review; commit | both | |

## Step 8: `docs: document the glass`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 8.1 `README.md` (line 23, "a real blur of the screen behind them"), `docs/template.md`, the spec's status to built and its "What changed while building it", the roadmap's row 0 | Claude | |
| [ ] | 8.2 Commit; merge `main` → `3d/base` → `3d/player` and `main` → `2d/base`; push | both | |

## Step 9 (on `3d/base`): `perf: stop drawing the level under the pause menu`

| | Task | Who | Facts |
|---|------|-----|-------|
| [ ] | 9.1 The playroom's camera stops drawing under `Pause::On`, once the pause layer's copy is made, and draws again when the game resumes | you | `Camera { is_active, .. }` on the `Camera3d` in `playroom.rs`; the screen camera keeps drawing, so the copy below the pause layer is still there |
| [ ] | 9.2 `just run`: the paused game looks the same; its `frame_time` is lower than the running game's; review; commit; merge `3d/base` → `3d/player`; push | both | |
