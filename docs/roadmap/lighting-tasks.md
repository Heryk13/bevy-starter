# Lighting: tasks

Roadmap row 9, on `3d/base`. The decisions and their reasons live in the spec,
`docs/specs/lighting/design.md`; this file only breaks its implementation order into tasks
small enough for one sitting each, with the facts each task needs.

## How to read this

- **Step** = one commit. Its title is the commit message.
- **Who**: *Claude* writes each step in pieces and the user reads and runs it; *you* is a
  `just run`, which only the user's eyes can do; *both* is the review at the end of a step
  (the checklist in `CLAUDE.md`: every name, the order of sections and systems, `just check`).
- **Facts**: the names that cannot be guessed, checked in the Bevy 0.20.0 source and in this
  repository on 2026-11-20.
- **Where**: every step is written on `3d/base`, and its look is tried on `3d/player`, by
  walking or flying (F7) into the rooms: `3d/base`'s still camera never enters one. So each
  step is synced down before its `just run`.
- **Blender** is `D:\blender\blender.exe` (5.2) on the user's machine, not on `PATH`. Run
  from a script (`blender -b art/playroom.blend --python ...`) it edits the level without
  opening it. Saving leaves an `art/playroom.blend1` backup: delete it, it is not committed.
- Mark a task done by changing its `[ ]` to `[x]`.

## Departures from the spec, to record in its "What changed while building it"

- **The falloff is the margin over the size, not twice it** (found in the first `just run`,
  from the user's photos): `light_probe.wesl` weighs an axis by
  `saturate((1 - 2|p|) / (2 * falloff))`, so each face fades over the falloff's share of
  the whole size, not half of it as `LightProbe`'s docs say. Twice the margin let the
  fade into the 1.8 m corridor and put the floors halfway into it, half lit by the sky; only
  where two boxes overlapped, their weights past 1.0, was the corridor dark.
- **The boxes sit on the walls' inner faces, and grow into them** (found in the second
  `just run`): drawn on the outer faces and grown past them, a box covered the outside of
  its walls whole, which went dark in the sun. Now the empty is drawn on the inner faces
  (z up to 3.0, the ceiling's underside), and `INTERIOR_BLEND_M` is 0.1 m, half the
  level's 0.2 m walls, so the fade happens inside the wall: the inside is all interior,
  the outside all sky. At a door the change is sharper, over those 0.1 m.
- **The eye needs its plugin and a target** (found reading `auto_exposure.wesl` for 2.1):
  `AutoExposurePlugin` is not in Bevy's default plugins, so `playroom.rs` adds it. And the
  spec's "daylight stays where it was tuned" does not hold alone: the eye aims every view's
  average at Bevy's 1.0, about two stops brighter than the fixed exposure leaves daylight.
  A flat compensation curve moves that aim by `PLAYROOM_CAMERA_EYE_TARGET_EV` (-2.0); it is
  an asset, so the camera kit names it by a `uuid_handle!` constant and `make_eye_curve`
  makes it at startup. The eye's limit is `range`: a view darker than its start is
  measured at it, so `PLAYROOM_CAMERA_EYE_DARK_LIMIT_EV` (3.0) is the most it brightens
  one past daylight. `PLAYROOM_CAMERA_EYE_BRIGHT_LIMIT_EV` keeps Bevy's 8 stops the other way.
- **Sky portals, a step of their own** (asked for after step 3's `just run`): at the
  corridor's open end the dark began on a flat plane, where the user wanted the light to
  come in and fade. A probe fades every face alike, so a long fade at the opening would
  have faded the side walls too. Engines light such openings with portal lights: an empty
  of `kind` `"sky_portal"` gets a sky-blue light. With three kinds read, `kind` became
  `marked`, which parses the JSON and checks the kind for every system. The docs moved to
  step 5.
- **The portal is a `RectLight`, after trying both** (decided in step 4's `just run`): a
  wide `SpotLight` with shadows came first, since a `RectLight` casts none in Bevy 0.20. On
  the opening, the walls just inside were unlit and the light a torch's round patch; set
  2 m back, it was better, but the user asked to see a `RectLight` and found it much
  better: the light comes in through the whole opening. So the portal is an Image empty
  scaled to the opening, its light a unit `RectLight` the empty's scale sizes (Bevy
  applies a `RectLight`'s scale), at `π × SKY_LUMINANCE_NITS × area` lumens, the sky the
  interiors keep out. It stands `INTERIOR_BLEND_M` outside the opening: on it, the strip
  of floor where the interior fades out was behind it, unlit, a dark line. The cost:
  `area_light_luts` turned on in `Cargo.toml`, and no shadows, so a portal lights through
  walls within its range; if Bevy gives it shadows, nothing in the level changes.
- **A thinner fade, the portal in its middle, a darker eye** (from the photos of step 4's
  `just run`): at an opening the interior's fade lies on the floor, and with a portal there
  any width of it is a step: the light on the opening left the strip unlit, a dark line,
  and set back the whole margin it lit the strip on top of the sky, a bright one.
  `INTERIOR_BLEND_M` went from 0.1 to 0.02 m, too thin for a step to show, and the portal
  stands half of it out. `SKY_LUMINANCE_NITS` went from 5 000 to 3 000, nearer the sky
  Bevy draws with the sun low, and `PLAYROOM_CAMERA_EYE_DARK_LIMIT_EV` from 3.0 to 1.5:
  the insides were still too light.
- **The house dark too, lit at its door and windows** (asked for after the corridor's
  portal): `House_Interior`'s `sky_light` went from 0.3 to 0.01, and the open front door
  and the eight windows got a portal each, placed from the walls' meshes. Its rooms share
  one box, and the portals cast no shadows, so a window lights the next room a little
  through the wall between them.

## The level's objects at a glance

Blender coordinates, Z up (the glTF's and Bevy's Y); Blender's Y is Bevy's -Z.

| Empty in `art/playroom.blend` | Custom properties | Covers (inner faces of the walls) | Step |
|-------------------------------|-------------------|-----------------------------------|------|
| `DarkRoom_Interior` | `kind` `"interior"` | x 33.1 to 42.9, y -24.9 to -18.1 | 1 |
| `DarkCorridor_A_Interior` | `kind` `"interior"` | x 31.9 to 49.9, y -28.9 to -27.1 | 1 |
| `DarkCorridor_B_Interior` | `kind` `"interior"` | x 48.1 to 49.9, y -28.9 to -21.1 | 1 |
| `DarkCorridor_C_Interior` | `kind` `"interior"` | x 49.9 to 61.9, y -22.9 to -21.1 | 1 |
| `House_Interior` | `kind` `"interior"`, `sky_light` 0.01 (0.3 at first) | x 34.1 to 49.9, y 18.1 to 29.9 | 1 |
| `DarkRoom_LightSpot` (already there) | `kind` `"light_point"` | at (38, -21.5, 2.7) | 3 |
| `DarkCorridor_A_Portal` | `kind` `"sky_portal"` | the open end at x 31.9, y -28.9 to -27.1, z 0 to 3.0; its Z to +x | 4 |
| `House_Door_Portal` and `House_Window_{S1,S2,N1,N2,W1,W2,E1,E2}_Portal` | `kind` `"sky_portal"` | the open front door and the eight windows, each on its wall's inner face, its Z into the house, its X along the wall, its Y up | 4 |

Every box runs from z 0 to 3.0, the ceiling's underside. Each is a Cube empty of display
size 1, its location the box's centre and its scale half its size.

## Step 1: `feat: keep the sky out of the level's interiors`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 1.1 The five interior empties in `art/playroom.blend` | Claude | Done while writing the spec, from the walls' meshes |
| [x] | 1.2 `assets/gameplay/playroom.glb` re-exported with custom properties | Claude | `export_scene.gltf(export_format='GLB', export_extras=True)`. Before the empties, the default export was byte for byte the committed `.glb`, so the new one differs by the empties and the extras alone |
| [x] | 1.3 `serde_json` as a dependency, the version already in `Cargo.lock` | Claude | `1.0.151`, brought today by `bevy_gltf`; a line in `Cargo.toml` with the reason |
| [x] | 1.4 `playroom/lighting.rs` and its plugin, added from `playroom.rs`; its `Config`: `BLENDER_CUBE_SIZE_M`, `INTERIOR_SKY_LIGHT`, `SKY_LUMINANCE_NITS`, `INTERIOR_BLEND_M` | Claude | `mod lighting;` next to `still_camera` and `sun_glare` |
| [x] | 1.5 The helpers, with tests: the probe's size and falloff from the empty's size, and `kind` and `sky_light` read from the extras' JSON (a bad `sky_light` → `None`) | Claude | `LightProbe { falloff: Vec3 }`, a share of the box on each axis: a fade of `b` metres on each face of a probe `s` wide is `b / s` (see the departures). `GltfExtras { value: String }` from `bevy::gltf` |
| [x] | 1.6 The dark map, made once: a resource holding a `solid_color` white `EnvironmentMapLight` | Claude | `EnvironmentMapLight::solid_color(&mut Assets<Image>, Color)`, then `intensity` per box; both `diffuse_map` and `specular_map` are the same 1×1 cubemap |
| [x] | 1.7 `spawn_interiors`: each `Added<GltfExtras>` with `kind` `"interior"` gets a child with `LightProbe` and the map at `sky_light × SKY_LUMINANCE_NITS`; a bad `sky_light` warns with the empty's `Name` | Claude | `bevy::light::{LightProbe, EnvironmentMapLight}`. At most 8 probes reach a view (`MAX_VIEW_LIGHT_PROBES`) |
| [x] | 1.8 `just run` on `3d/player`: the room and the corridor go black, the house darker, outside unchanged | you | Black is expected: the exposure is still the sun's until step 2 |
| [x] | 1.9 The step's review | both | |

## Step 2: `feat: let the eye get used to the dark`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 2.1 Read `auto_exposure.wesl` and settle the eye's limit: `range` or a compensation curve; name its constant in `playroom.rs`'s camera kit group | Claude | `AutoExposure { range, filter, speed_brighten, speed_darken, exponential_transition_distance, metering_mask, compensation_curve }` in `bevy::post_process::auto_exposure`; the curve is an `AutoExposureCompensationCurve` asset, `from_curve(..)`. Its correction is added to the camera's `Exposure` |
| [x] | 2.2 `AutoExposure` in `PlayroomCamera`'s `#[require]` | Claude | It requires `Hdr`, already there through `AtmosphereSettings`. Not on WebGL2 |
| [x] | 2.3 `AmbientLight` at `brightness` 0.0 in the same `#[require]` | Claude | `bevy::light::AmbientLight { color, brightness, affects_lightmapped_meshes }`; on a camera it replaces `GlobalAmbientLight` for that view only |
| [x] | 2.4 `just run` on `3d/player`: tune `INTERIOR_SKY_LIGHT`, the house's `sky_light` and the eye's limit until "dark, but you see a little"; look out of a window from inside, at the sun's glare | you | |
| [x] | 2.5 The step's review | both | |

## Step 3: `feat: light the level's lamps`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 3.1 `lighting.rs`'s lamp group in `Config`: `LAMP_INTENSITY_LM`, `LAMP_RANGE_M` | Claude | |
| [x] | 3.2 `spawn_lamps`: each `Added<GltfExtras>` with `kind` `"light_point"` gets a child `PointLight` with shadows | Claude | `PointLight { intensity, range, radius, shadow_maps_enabled, .. }`; its shadow is a cubemap, six passes |
| [x] | 3.3 `just run` on `3d/player`: tune the lamp | you | |
| [x] | 3.4 The step's review | both | |

## Step 4: `feat: let the sky's light in at openings`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 4.1 `DarkCorridor_A_Portal` in `art/playroom.blend`, an Image empty scaled to the corridor's open end (3.0 by 1.8), and `playroom.glb` re-exported | Claude | Its Z, where the light shines, turned 90° about Y to +x. In the glTF, Blender's Z is the node's +Y and its X and Y the node's X and Z: checked in the `.glb`, rotation +Y to +x, scale (3, 1, 1.8) |
| [x] | 4.2 `lighting.rs`'s portal group in `Config`: `SKY_PORTAL_COLOR`, `SKY_PORTAL_RANGE_M`; `area_light_luts` in `Cargo.toml` | Claude | `RectLight { color, intensity, range, width, height }` needs the feature, not in Bevy's `3d` set |
| [x] | 4.3 `spawn_sky_portals`, `sky_portal_lumens`, and `marked` in place of `kind`, with tests | Claude | A `RectLight` shines along its `-Z`: `looking_to(Vec3::Y, Vec3::Z)` turns it to the empty's Z, its X to the empty's X and its Y to the empty's glTF Z. Bevy multiplies `width` and `height` by the global scale of those axes (`render/light.rs`) |
| [x] | 4.4 `just run` on `3d/player`: the light comes in at the corridor's end and fades; tune it | you | |
| [x] | 4.5 The step's review | both | |

## Step 5: `docs: mark the lighting spec built`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 5.1 `docs/template.md`: the three `kind`s, `sky_light`, the Cube empty's size, the portal's arrow, and the Custom Properties export option | Claude | |
| [x] | 5.2 README's `3d/base` row, the roadmap's row 9, the spec's status line and its "What changed while building it" | Claude | The README row stays: its Branches table lives on `main`, and the row still holds. The roadmap is `main`'s too, so row 9 is a commit there |
