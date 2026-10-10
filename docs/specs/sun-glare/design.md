# Sun glare: the sun as a camera sees it

Status: **approved**, step 1 built.
Date: 2026-10-10
Bevy: 0.20.0
Branch: `3d/base`

## Goal

The playroom's sun is a crisp disk with a soft halo. Looking at the real sun, nobody sees its
shape: a camera shows a blinding white glow, far larger than the disk, with rays of light
spreading from it. This design draws that glare over the sun, hidden by whatever stands in
front of it, as the leaves of a tree cut it in a photo.

## Why it cannot be tuned in

- `Bloom` is what spreads a bright spot, but its first downsample always runs a firefly
  filter (a Karis average) that "limits luma to [0, 1]": the sun counts barely brighter than
  the sky beside it, so no `SunDisk` intensity or size grows its halo
  (`bevy_post_process/src/bloom/bloom.wesl`, `downsampling_pipeline.rs`).
- A stronger `Bloom` hazes the whole view, and a hazier sky (a larger Mie term) widens the
  bright sky around the sun, not the glare. Both were tried in the playroom's step 1.
- Bevy has no lens flare or starburst.

So the glare is something drawn, not a setting.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Where it lives | `3d/base`, every 3D game inherits it | The user's call. It reverses the playroom's "a lens effect is a game's look", recorded there |
| What it draws | A wide glow and rays in a star. No flare ghosts | The two that make the sun read as blinding. Ghosts are a second feature, and the most work |
| How it is drawn | A square mesh in the world, far along the sun's direction from the still camera, always facing it | In the world, the depth buffer hides it behind any wall, in part when the sun sits on a corner. A UI image over everything would shine through the level |
| Its material | `StandardMaterial`, `unlit`, `AlphaMode::Add`, `fog_enabled: false`, its texture tinted by a colour above 1.0 | Unlit output skips the exposure (`pbr_functions.wesl`), so the colour reaches the screen as given. Additive blending only adds light, never darkens the sky. A colour past white feeds `Bloom` |
| Its shadow | `NotShadowCaster` | A square standing in the sun's path would shade the level |
| Its texture | Generated at startup from a pure function, one value per pixel: brightness from the centre out | No file and no licence to check, tuned by constants, and the function is tested |
| The function's shape | A glow falling off with the distance from the centre, plus `SUN_GLARE_RAY_COUNT` narrow rays, all reaching exactly zero at the edge | A texture that is not zero at its edge draws the square's outline in the sky |
| Its size | An angle on screen in `Config` (`SUN_GLARE_DEGREES`), turned into metres from its distance | The glare looks the same size wherever the square stands, and the distance can change freely |
| Its distance | A `Config` constant, short of the camera's far plane (1000 m by default) and past the level | Past the level so nothing behind it is hidden, inside the far plane so it is drawn at all |
| Where it follows the camera | One system in `Update` sets its position and rotation from the still camera's transform each frame | The still camera never moves on `3d/base`, but the branches below move theirs |
| Where the code sits | `src/screens/gameplay/playroom/sun_glare.rs`, a plugin added by `playroom.rs`, reading `SUN_POSITION` from it | A part that makes sense on its own, with its own systems, helper and tests. Split the `screens.rs` way, `playroom.rs` next to `playroom/` |

## Risks

- **The atmosphere's aerial perspective may tint the square.** At hundreds of metres Bevy
  hazes what it draws towards the sky colour, which could wash the glare out. If so, the
  distance comes down, or the square is drawn without it.
- **The sun's disk under it.** With the glare drawn, `SUN_DISK_SCALE` may go back to 1.0.
  Decided by looking at the game.

## Not in this design

Flare ghosts along the line from the sun to the centre of the screen, dimming the glare as
less of the sun is visible, glare from any light but the sun, and settings for the player.

## Implementation order

One commit per step. `just check` passes after each one, and `just run` shows it.

1. **Draw the glare.** `sun_glare.rs`: the texture built from the pure function at startup,
   the square spawned on entering gameplay with `DespawnOnExit(Screen::Gameplay)`, and the
   system that keeps it on the sun facing the camera. Tests on the function: brightest at
   the centre, zero at the edge, brighter along a ray than between two at the same distance.
   Tune the constants by looking at the game.
   `feat: draw a glare over the sun`
2. **Docs.** `docs/template.md` (where the glare lives), the playroom spec's "What changed",
   this spec's status.
   `docs: describe the sun glare`

## What changed while building it

- **The texture is built on entering gameplay, not at startup.** It takes a fraction of a
  millisecond, and building it where the square is spawned keeps the whole glare in one
  system, its texture freed with it when gameplay ends.
- **The glare is placed in `PostUpdate`, before transform propagation, from the camera's
  `Transform`.** On the first frame the camera's `GlobalTransform` is still at the origin,
  so placing from it, or in `Update`, would show a huge square there for a frame.
- **The glare is three layers, not a glow and rays.** A single `1/(1+r²)` glow left a small
  crisp dot under long, even rays. The function now adds a wide core that blows out into a
  white ball (a Gaussian), a wide halo, and rays that fade fast (both exponentials), each
  with its own constants, tuned by eye against a photo of the sun.
- **The sun's disk stays twice its real size.** Bevy draws it with an edge one pixel wide
  (`fwidth`) and nothing softens it, so at the real size it sat hidden inside the core's
  white ball. That was tried and looked worse than a disk showing through, kept at twice
  the real size by `SUN_DISK_SCALE`.
