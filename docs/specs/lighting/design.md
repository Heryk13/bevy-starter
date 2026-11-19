# Lighting: dark places really dark

Status: **built** 2026-11-20, on `3d/base`.
Date: 2026-11-20
Bevy: 0.20.0
Roadmap: row 9 of `docs/roadmap/roadmap.md`, from its "Lighting, as agreed" section.

## Goal

Inside `DarkRoom` and `DarkCorridor` it is dark: you can still make out a little, the way an
eye sees once it gets used to a dark room. `DarkRoom_LightSpot`, on the room's ceiling,
becomes a lamp. The house, which has doors and windows, is darker than outside and lighter
than the corridor. Outdoors looks as it does today.

## Why it is light today

The level has three lights:

| Light | What it does | Does a wall stop it? |
| --- | --- | --- |
| The sun (`DirectionalLight`, with shadows) | Light from one direction | Yes: its shadow already keeps it out of the rooms |
| The sky (`AtmosphereEnvironmentMapLight` on the camera) | A picture of the sky that lights every surface by the way it faces | **No**: the shader knows the surface's direction, never that a ceiling covers it. SSAO only darkens creases, up to 5 m |
| Ambient light (`GlobalAmbientLight`, Bevy's default of 80) | Added to everything | No, but next to the sun it is too faint to see |

The exposure is also fixed at EV 13.5, a sunny day's. With it a dark room is black, and a
lamp inside one barely shows next to the sun.

## Approaches

| Approach | How | Cost per frame | Why not |
| --- | --- | --- | --- |
| **Interior boxes** (chosen) | A `LightProbe` with a dark `EnvironmentMapLight` around each interior. Bevy's shader lets a reflection probe that fully covers a surface replace the camera's sky there, and blends the two over the box's edge: Bevy's own comment says it is there "for nice transitions between the interior of a building and the outdoor environment map" | Almost none: up to 8 probes a view, one lookup each, and the dark map is a 1×1 cubemap made once | The light inside is even, with no light bouncing off the lamp, and each box is a box, never the room's shape |
| Irradiance volumes or lightmaps, baked in Blender | Blender's EEVEE bakes the light, bounces included, and a tool (`export-blender-gi`) turns it into a `.ktx2` Bevy reads | Low | A tool outside cargo, a bake again after every change to the level, and light that never moves. A game that wants bounced light adds it later, on top of this |
| Solari (ray tracing) | Bevy's real-time global illumination | Heavy | Experimental, needs a GPU with ray tracing, and in 0.20 it takes no `SpotLight` or `PointLight`, so the lamp would not light |

`AutoExposure` comes with every approach: without it, the exposure that suits the sun leaves
any interior black.

## Decided

| Question | Decision |
| --- | --- |
| Branch | `3d/base`: every 3D game has a level with insides. The still camera of `3d/base` never enters a room, so the look is tried on `3d/player`, by walking or flying (F7) in, after the sync merges it down |
| Where the interiors come from | The level, not the code: an empty in `art/playroom.blend` with the custom property `kind` set to `"interior"` marks one. The level already names what each special object is this way (`kind` is `"door"` on the doors, `"ladder"` on the ladders, `"light_point"` on `DarkRoom_LightSpot`), so the interiors follow it, and their names are free. Whoever makes a level marks its insides in Blender; the code never lists rooms. The empty's display is a **Cube** of display size 1, so what Blender draws is the box: such a cube is 2 m a side before its scale (`BLENDER_CUBE_SIZE_M`), and Bevy's light probe is 1 m |
| A room that is not a box | Several boxes: the corridor is an S of three straight stretches, so it has three, `DarkCorridor_A_Interior` to `_C_`. One box around all of it would darken the outside between its arms. Five boxes in all, under the 8 a view takes |
| The level's custom properties | Exported from now on (Include → Custom Properties in Blender's glTF exporter, off by default). Every `kind` reaches the game as `GltfExtras`, read by nothing yet: the doors and ladders rows will read theirs. Blender writes them into the `.glb` as glTF extras, and Bevy hands them over as `GltfExtras`, a JSON string read with `serde_json`, which bevy already brings into the tree. With the option on and no other change, the export was checked to differ from the old `.glb` only by the new empties and the extras |
| How dark each interior is | A custom property `sky_light` on the empty: how much of the sky gets in, from 0.0 (none) to 1.0 (as outside). The house, with windows, gets 0.3 to start; an empty without one gets `INTERIOR_SKY_LIGHT`. A `sky_light` that is not a number or outside 0.0 to 1.0 falls back to the default with a warning naming the empty |
| The dark map | `EnvironmentMapLight::solid_color(Color::WHITE)` with `intensity` at `sky_light × SKY_LUMINANCE_NITS`. `SKY_LUMINANCE_NITS` is what `sky_light` 1.0 means, the sky's brightness in cd/m², tuned once so a box at 1.0 looks like outside. A solid colour, not the atmosphere: an `AtmosphereEnvironmentMapLight` on a probe would render and filter a cubemap per box every frame. One white map serves every box |
| The box's edge | The empty is drawn on the walls' outer faces, and the code grows the probe by `INTERIOR_BLEND_M` on every side. The probe's falloff fades it over that margin, so inside the walls the box is whole and only an open door shows the fade. Bevy's falloff is a share of the box's size, so it is worked out per box (a pure function, tested), as is the probe's scale: the empty's size plus the two margins. Tuning the margin never means redrawing a box in Blender. The outside of a wall, and the ground at a door, are inside the margin and get a little darker: thin enough to miss, tuned by eye |
| Boxes that overlap | Allowed: the corridor's three do at its two turns. Bevy blends overlapping probes by their weights |
| Ambient light | Off on the playroom's cameras: `AmbientLight` with `brightness` 0.0 in the camera kit. Outside the sky does its job, and inside a dark room, with the eye adapted, the default 80 would light the room evenly and undo the boxes. A component on the camera, not a change to the global resource, so nothing has to put it back when gameplay ends |
| The eye | `AutoExposure` in the camera kit, on top of the fixed `Exposure`: Bevy adds its correction to the camera's own exposure, so daylight stays where it was tuned. It requires `Hdr`, which the kit already has. "Dark, but you see a little" is how far it brightens a dark view: its `range` and a compensation curve (`AutoExposureCompensationCurve`) that leaves a dark view darker than average. Which of the two holds the knob is checked at step 2, against the shader, and named there. How fast it adapts uses Bevy's speeds (3 stops a second to brighter, 1 to darker) until the eye says otherwise |
| The lamp | An empty with `kind` `"light_point"`, as `DarkRoom_LightSpot` already has, gets a `PointLight` with shadows as a child: a bulb that lights every way, the ceiling included, which is what the level asks for. A point light's shadow is a cubemap, six passes where a spot's is one; with one lamp that is cheap. More lamps are more empties in Blender. `LAMP_INTENSITY_LM` and `LAMP_RANGE_M` are the knobs; every lamp shares them |
| Lights made in Blender | Not used: Bevy imports glTF's punctual lights, but without shadows, and their brightness would be tuned by re-exporting. A game that wants them turns on Blender's export of Punctual Lights |
| The code | `src/screens/gameplay/playroom/lighting.rs`: a system that reads entities with `Added<GltfExtras>` during gameplay, and spawns the probe or the lamp under each whose `kind` is one of the two. The camera kit's two additions stay in `playroom.rs`, with the rest of the kit |
| Release builds | Everything ships: it is the game's light, not a dev tool |

## The knobs the user tunes in `just run`

| Constant | What it changes |
| --- | --- |
| `INTERIOR_SKY_LIGHT` | How dark an interior is when its empty says nothing |
| `sky_light` on `House_Interior` (in Blender) | How dark the house is |
| `SKY_LUMINANCE_NITS` | What "as outside" is: tuned once, then left alone |
| `INTERIOR_BLEND_M` | How wide the fade at a door is |
| the eye's limit (named at step 2) | How much you see once the eye adapts |
| `LAMP_INTENSITY_LM`, `LAMP_RANGE_M` | The lamp |

## Risks, checked on the way

- **The glare through a window:** `AutoExposure` corrects the whole picture, the sun's glare
  included, which skips only the fixed `Exposure`. Looking out from a dark room, the glare is
  brighter, as it is for a real eye. If it reads wrong, it is noted at step 2.
- **Shadows leaking:** the sun's shadow on thin walls and roofs may let light through at a
  seam. If it shows, the fix is the sun's cascades, not the boxes.

## Out of scope

- **Light that bounces** (baked irradiance volumes, lightmaps, Solari): later, per game.
- **A lamp switch**: the lamp is always on. Turning it on and off belongs to the doors and
  props rows.
- **Lamps that differ from each other**: one set of lamp constants until a level needs two.

## Implementation order

One commit each, on `3d/base`, then synced down.

1. **The interiors.** The five `kind` `"interior"` empties in `art/playroom.blend` (done
   while writing this spec, with Blender run from a script) and `playroom.glb` re-exported
   with its custom properties. In code, `lighting.rs` with the probes, the helpers for the
   probe's size and falloff and the `sky_light` reader, with tests. With the fixed exposure
   the rooms go black, which is expected until step 2.
2. **The eye.** `AutoExposure` and the ambient light at zero in the camera kit, the eye's
   limit named and tuned.
3. **The lamp.** `"light_point"` empties become point lights with shadows.
4. **Sky portals** (added while building, see "What changed while building it").
   `"sky_portal"` empties become wide spot lights with shadows, along their arrow.
5. **Docs.** `docs/template.md`: the three `kind`s, `sky_light`, the cube's size, the
   portal's arrow, and the Custom Properties export option, which Blender leaves off.
   README's `3d/base` row, the roadmap's row 9, this status line.

## What changed while building it

The tasks file, `docs/roadmap/lighting-tasks.md`, tells each of these in full.

- **The falloff is the margin over the size.** `light_probe.wesl` fades each face over the
  falloff's share of the whole size, not half of it as `LightProbe`'s docs say. Twice the
  margin let the sky into the corridor and onto the floors.
- **The boxes sit on the walls' inner faces**, up to the ceiling's underside, and grow into
  the walls. On the outer faces they darkened the outside of their walls in the sun. The
  margin ended at 0.02 m: at an opening the fade lies on the floor, and any width of it
  showed as a line.
- **The eye needs its plugin and a target.** `AutoExposurePlugin` is not in Bevy's default
  plugins, and alone the eye aims every view at Bevy's 1.0, about two stops brighter than
  daylight was tuned. A flat compensation curve, an asset named by a `uuid_handle!`
  constant, aims it at `PLAYROOM_CAMERA_EYE_TARGET_EV`; `range` is the eye's dark limit.
- **Sky portals, a step of their own, and a `RectLight`.** The corridor's dark began on a
  flat plane where the user wanted light coming in. A box fades every face alike, so
  openings got a mark of their own, `"sky_portal"`. A wide `SpotLight` with shadows came
  first; the user saw a `RectLight` the opening's size and kept it, though it casts no
  shadows in Bevy 0.20 and needs `area_light_luts`. The portal is an Image empty scaled to
  the opening, as bright as the sky over its area, half the margin outside it. With three
  kinds, `kind` became `marked`. The docs moved to step 5.
- **Darker than planned.** `INTERIOR_SKY_LIGHT` ended at 0.005, the eye's dark limit at
  0.5 stops, `SKY_LUMINANCE_NITS` at 3 000: far from its openings an interior is dark, and
  a portal is the light at the end of the tunnel. The house went from `sky_light` 0.3 to
  0.01, with a portal at its open door and each of its eight windows.
- **One commit for the code, not four.** The steps were tuned together in the same files,
  so splitting them would have made commits that never ran.
- **README's `3d/base` row is unchanged**: the Branches table lives on `main`, and the row
  still holds. The roadmap's row 9 is marked on `main`.
