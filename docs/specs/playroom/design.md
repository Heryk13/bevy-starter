# The playroom: a 3D level every 3D game starts from

Status: **approved**, not built yet.
Date: 2026-10-08
Bevy: 0.19.1
Branch: `3d/base`

## Goal

`Screen::Gameplay` is still a coloured background. On `3d/base` it becomes a place: entering
gameplay shows `assets/gameplay/playroom.glb`, a test level for character controllers
(stairs, ramps from 15° to 60°, jump gaps, a maze, a house), lit, in front of a camera that
stands still.

That is all `3d/base` does. Nothing moves and nothing can be controlled: no player, no input,
no physics. Whoever walks the level, and how, is the business of the branches cut from here
(`3d/first-person`, and the variants cut from it in turn), so they start from a level on
screen without inheriting a body or a camera they would have to take apart.

## Decided

| Question | Decision | Why |
| --- | --- | --- |
| What the base holds | The level, its light and a still camera. No physics, no player, no input | The user's call: the base is the map in front of you and nothing else. Physics arrives with the first branch that has something to move |
| Loading the glb | The loading screen loads it, for both destinations | `docs/flow.md` already says the loading screen "is where the gameplay's own assets will be loaded later". Entering gameplay with the level loaded avoids a blank frame and keeps the bar honest |
| Sky | Bevy's own `Atmosphere::earth`, with a `ScatteringMedium::earth` handle, as in Bevy's `examples/3d/atmosphere.rs` | A physically based sky: the blue, the paler horizon and the haze over distance come from how sunlight scatters, not from a colour picked by hand. It ships with Bevy, so it moves to each new release with it. The crates were worse: `bevy_atmosphere` stopped at an older Bevy (last release May 2025) now that Bevy has its own, `bevy_sky_gradient` is a stylized gradient, and an HDRI skybox needs a large `.ktx2` whose painted sun never matches the light |
| Sun | A `DirectionalLight` at `lux::RAW_SUNLIGHT` with shadows, and a `SunDisk` so it shows in the sky. It never moves: its angle is a constant in `Config`, settled by looking at the game | The illuminance the atmosphere is made for. A moving sun, or a day cycle, is a later design |
| Ambient light | `AtmosphereEnvironmentMapLight` on the camera, and no `AmbientLight` | The sky itself lights what the sun does not reach, blue from above, as outdoors. A flat ambient colour would be a second, wrong sky |
| Camera | A `Camera3d` at a fixed spot looking at the origin, in `Config`. It carries `AtmosphereSettings` (which brings `Hdr`), `Exposure { ev100: 13.0 }`, `Tonemapping::AcesFitted` and `Bloom::NATURAL` | Something has to show the level, and a branch cut from here replaces it with its own. Real sunlight is far too bright for the default exposure, and bloom gives the sun its glow. The values come from Bevy's example, as a start to tune |
| Shadows over the whole level | Bevy's default cascades: 4 of them, the first ending at 10 m, shadows out to 150 m from the camera, on a 2048 px shadow map | Made for large outdoor scenes, and nearly all of the level is within 150 m of a camera near its centre. Only the far end of the shooting range lies beyond, where a missing shadow goes unseen. Tuned only if `just run` shows a problem |
| Contact shadows | `ScreenSpaceAmbientOcclusion` on the camera | It darkens creases, stair edges and where objects meet the floor. On a level of flat prototype colours it is what most makes it look real |
| Anti-aliasing | `Smaa` on the camera, with `Msaa::Off` | SSAO needs `Msaa::Off`. SMAA smooths edges without TAA's trails when a camera moves, which the branches below will do |
| Where the code lives | `src/screens/gameplay/playroom.rs`, registered by `gameplay.rs` like `pause.rs` | Split by feature, following the `gameplay.rs` + `gameplay/` pattern |

## Risks to check while building

- **The UI camera over the 3D camera.** The screen camera is a `Camera2d` that clears the
  window and carries the blur (`ui/blur.rs`, a `Core2d` fullscreen pass). The `Camera3d` has to
  draw first (a lower `order`), and the screen camera must not clear over it during gameplay.
  Step 1 checks the playroom shows, and that the pause menu still blurs it. `Hdr` makes this
  likelier to need work: an `Hdr` camera renders into a texture of its own, which the 2D
  camera's blur pass may never see.
- **Cost.** SSAO is the expensive piece. If the frame rate drops noticeably on a mid-range
  GPU, lower its quality first, and write down under "What changed" what was given up.
  Settings to switch it off are a later design.
- **Sunlight through the house windows.** `M_Glass` is `BLEND` with an alpha of 0.2, and Bevy
  only leaves a blended surface out of the shadow map below an alpha of 0.05
  (`PREMULTIPLIED_ALPHA_CUTOFF` in `pbr_prepass_functions.wgsl`), so the glass likely shades
  like a wall and no sun reaches inside. If so, `NotShadowCaster` on `House_Windows_Glass`.

## Not in this design

Physics, a player, input of any kind, a camera that moves, and anything the playroom's props
suggest (doors, levers, pickups, physics crates). Those belong to the branches cut from here.
Also left for later: light shafts (god rays), which the user has a plan of their own for, a
moving sun or a day cycle, and graphics settings for the player.

## Implementation order

One commit per step. `just check` passes after each one, and `just run` shows it.

1. **Show the playroom.** `GameplayAssets` in `loading.rs`, loaded for both destinations and
   counted by the bar. `playroom.rs` spawns the scene, the sky, the sun and the still `Camera3d`, all
   with `DespawnOnExit(Screen::Gameplay)`, in place of the coloured background. Settle the
   camera order and clearing, and check the pause blur.
   `feat: show the playroom in gameplay`
2. **Make it look real.** SSAO, with SMAA in place of MSAA. Tune its values by looking at the
   game.
   `feat: give the playroom contact shadows`
3. **Docs.** `docs/flow.md` (Loading, Gameplay), `README.md` (features), `docs/template.md`
   (structure, and the Bevy features a 3D game keeps), this spec's status.
   `docs: document the playroom`

## What changed while building it

- **Step 1 is two commits.** The level and the still camera went in first, the sky, the sun and
  the camera's exposure after, so the camera order was settled before `Hdr` could hide why.
- **The camera order needed settling, the clearing did not.** Both cameras at the default
  order of 0 made Bevy warn of an ambiguity and the level flicker, so the still camera has
  `STILL_CAMERA_ORDER` (-1). The screen camera never wipes the level: Bevy 0.19 gives cameras
  with the same window, format and MSAA one shared texture, cleared only by the first camera
  each frame. That changes once the still camera has `Hdr` and `Msaa::Off`, so both steps
  check it again.
- **Bevy's default tonemapping, not `AcesFitted`.** Bevy documents `AcesFitted` as "not
  neutral": bright greens and reds turn orange, bright blues magenta. Every 3D branch inherits
  this camera, so its look belongs to each game, not to the base, and a neutral curve shows
  what the light is doing when judging it by eye. `TonyMcMapface` is that curve and Bevy's
  default, so the camera carries no `Tonemapping` at all. `AgX` looked the same here.
- **The still camera looks up, not at the origin.** It stands at about 41 m and looks at a
  point high over the level, so the level fills the bottom of the view and the sky, the sun
  and the shadows falling towards the camera can all be judged at once.
- **The project moved to Bevy 0.20 partway through step 1**, merged down from `main`.
- **The UI cameras take `Hdr` during gameplay.** `Bloom` brings `Hdr`, which moved the still
  camera onto an image of its own, and the screen camera's opaque clear then hid the level
  (Bevy alpha-blends every camera after the first onto the window). Clearing it transparent
  would have shown the level but left the blur blind to it. Giving the screen camera `Hdr`
  too froze the view instead: the floating camera never clears, and alone on its own image
  it kept showing the last frame they had shared. So `give_ui_cameras_hdr` gives every
  `Camera2d` `Hdr` on entering gameplay, and `take_ui_cameras_hdr` takes it back on leaving,
  and the three cameras share one image again. Step 2's `Msaa::Off` splits them the same way,
  so the UI cameras will need it too. Rendering the level into a `ViewportNode` was the
  fallback, kept for if this stops holding.
- **The sun's disk is wider than the real one.** A directional light without a `SunDisk` is
  drawn with `SunDisk::EARTH` anyway, so the plain preset changed nothing on screen. At its
  real size the sun was an 8 px dot, so `SUN_DISK_SCALE` draws it five times as wide, as
  games often do. It changes only the disk, not the light or the shadows.
- **The sun's halo stays soft.** `Bloom::NATURAL` gives it one, but no brighter disk makes
  it larger: `Bloom`'s first downsample always runs a firefly filter (a Karis average) that
  "limits luma to [0, 1]", so the sun counts barely brighter than the sky beside it. A
  stronger `Bloom` would haze the whole view, and a lens flare is a feature of its own; both
  are a game's choice of look, not the base's, so the base keeps the soft halo. A hazier sky
  (the Earth preset's Mie term scaled up) was tried and dropped: it widens the bright sky
  around the sun, not the blinding glare and rays a camera shows, which are lens effects. A
  sun glare drawn over the sun's place on screen is left to a design of its own.
- **The sky lights the shadows at less than its real strength.** At the physical 1.0 the
  shadows read too light for the exposure the sunlit side needs, so
  `STILL_CAMERA_SKY_LIGHT_INTENSITY` lowers only the sky's share. Step 2's SSAO darkens
  creases and contacts, so the value is worth revisiting once it is in.
