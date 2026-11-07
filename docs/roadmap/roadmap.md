# Roadmap

Everything left for the template's 2.0 (1.0 is tagged `v1.0.0`), in the order it gets built.
Agreed on 2026-11-07.

## How to read this

- **One row, one spec.** When a row starts, it gets its design in
  `docs/specs/<feature>/design.md` on the branch it belongs to, and a tasks file next to this
  one (`<feature>-tasks.md`). A row marked "base in `main`" is two or three specs: the shared
  part first, then one per genre.
- **Branch** follows the tree in `CONTRIBUTING.md` (Branches): a change goes into the highest
  branch every user of it lives under, then merges down. `main` is also the CLI's *Empty* game,
  so whatever lands there must make sense with no player and no props.
- **Status**: *next*, *designed* (spec written), *building*, *built* (spec's status is built).
  Update it here when it changes.
- **Order** is deliberate: the look and the key bindings first, since everything after is
  built on them; then the tools that help test the rest; multiplayer and its chats late,
  because they reach into everything before them; the release last.
- **"As agreed" sections** below hold what was decided for a row before its spec exists, so
  the spec starts from them instead of from memory.

## The rows

| #  | Item | Branch | Status | Spec / tasks | Notes |
|----|------|--------|--------|--------------|-------|
| 0  | **Liquid Glass UI**: the whole template's look, the game's own UI (menu, pause, overlays, dialogs) and the dev tools' alike; glass panels that blur what is behind each one, the Lucide icon font, the shared pieces (popover, HUD, bar, list, buttons, alert) | `main` | glass designed | `glass/design.md`, `glass-tasks.md` | agreed so far: "The look" below, its spike measured; three specs (glass, look, pieces); first, so every tool is born with the look; supersedes the `ui-style` look |
| 1  | **Key bindings**: a Controls page in Settings to change the keys and gamepad buttons | base in `main`, then each branch registers its own actions | | | agreed so far: below; right after the look, so every later row reads a bound key, never a fixed `KeyCode` |
| 2  | **Dev tools base**: keys panel (F1), FPS (F2), slow motion (F4), `just profile` (`trace_chrome`) | `main` | designed; to revise for rows 0 and 1 before building | `dev-tools/design.md`, `dev-tools-tasks.md` | waits on rows 0 and 1 |
| 3  | **Photo mode** (F3): freeze, hide the dev UI, free camera, E takes a photo | `main` | | | agreed so far: below |
| 4  | **File explorer** (Numpad 0): the photos and videos, like the Windows explorer | `main` | | | agreed so far: below; was photo mode's gallery |
| 5  | **Dev tools 3D**: physics debug view (F6), fly (F7), photo mode's 3D free camera | `3d/player` | | | agreed so far: below; `hide_body_debug_collider` goes |
| 6  | **2D player**: a square that walks, no animation, and photo mode's 2D free camera | `2d/player` (new) | | | agreed so far: below; the CLI asks without or with the player module |
| 7  | **Speed + UI** (F8) | base in `main`, then `2d/player` and `3d/player` | | | agreed so far: below |
| 8  | **Screen recording** (F9): a countdown, then a blinking REC while it records, sound included | `main` | | | agreed so far: below |
| 9  | **Lighting**: dark places really dark | `3d/base` | | | agreed so far: below; test in `DarkRoom`, `DarkCorridor`, `DarkRoom_LightSpot`; the sky light lights interiors evenly today |
| 10 | **Character controller**: stairs, run, jump, crouch | `3d/player` | | | agreed so far: below; the capsule catches on the `Arena2F_Stair_*` steps; avian 0.8's `move_and_slide` is worth reading first |
| 11 | **Ladders**: teleport first, then real climbing | `3d/player` | | | agreed so far: below; test on `FallTower_Ladder`, `Pool_Ladder`, `ClimbWall_2m` to `_6m` |
| 12 | **Doors**: press E to open | `3d/player` | | | agreed so far: below; test on `DarkRoom_Door`, `Door_Bedroom1/2`, `Door_Front` (each with a `_Knob`) |
| 13 | **Prop editor** (F10): spawn, delete, move, rotate; the map file | base in `main`, then 2D and 3D | | | agreed so far: below; after doors, since a door is a kind of prop |
| 14 | **Trailer scenes**: named copies of the map, the player included, picked from a window F10 opens | base in `main`, then 2D and 3D | | | agreed so far: below; uses the props, so it comes after them |
| 15 | **Third-person camera**: over the shoulder, V swaps shoulders | `3d/player` | | | agreed so far: below; brings the CLI's camera question, marker removal and `just check-combos` |
| 16 | **Isometric camera**: fixed, in perspective | `3d/player` | | | agreed so far: below; marker `// camera: isometric` |
| 17 | **Camera switch** (F5) | `3d/player` | | | agreed so far: below; `camera_switch.rs`, only when a game has two cameras or more |
| 18 | **Multiplayer**: co-op up to 10, one player hosts, the rest join by IP (LAN or Radmin) | to decide | | | agreed so far: below; last, since it reaches into spawn, input, save, pause and scenes; its spec decides how the CLI removes it |
| 19 | **Text chat** for multiplayer games | with multiplayer | | | agreed so far: below |
| 20 | **Voice chat (VoIP)** for multiplayer games | with multiplayer | | | agreed so far: below; a spike first, like recording's sound |
| 21 | **Release 2.0**: tag `v2.0.0`, README and its branch table, a new `create-bevy-starter` with the new questions (2D player, cameras, multiplayer) | `main`, then every branch; the CLI's own repository | | | last: it ships everything above |

## Keys

Taken and reserved dev keys, so two rows never claim the same one. These are the default
bindings: row 1 makes every one of them rebindable. The keys panel (F1) lists every one that
exists in the build, with the key it is bound to.

| Key | Tool | Row |
|-----|------|-----|
| F1 | keys panel | 2 |
| F2 | FPS | 2 |
| F3 | photo mode | 3 |
| F4 | slow motion | 2 |
| F5 | camera switch (a game key, not a dev key) | 17 |
| F6 | physics debug view | 5 |
| F7 | fly | 5 |
| F8 | speed | 7 |
| F9 | screen recording | 8 |
| F10 | prop editor | 13 |
| Numpad 0 | file explorer | 4 |
| Numpad − / + | the slow motion or speed window, whichever is open | 2, 7 |

F11 (fullscreen in many games) and F12 (Steam's screenshot key) are best left alone.

## The look, as agreed so far (row 0)

Seen in browser mockups on 2026-11-07. The user rejected a first flat version as looking
machine-made, liked a macOS vibrancy version, and liked a Liquid Glass version better still.
The chosen mockup is kept next to this file: `ui/mockup.html` (opens in any browser; its
icons load Lucide from unpkg) and `ui/mockup.png`, the same page as one image. Every block
names its roadmap row.

- **Everything gets this look**, the game's own UI as much as the dev tools: the menu, the
  pause menu, the overlays (settings, saves), the dialogs. The template keeps one face. Its
  colours, sizes and pieces stay in `ui.rs` and its `Config`, as the Bevy patterns ask; the
  built `ui-style` spec is history and keeps its text, the new spec replaces its look.

- **Apple's Liquid Glass** (macOS 26 onwards): a clear glass that blurs only what is behind
  each panel, lightly (more saturation than blur), a bright specular rim along the top-left
  edge, darkened inner edges at the bottom for depth and legibility, one corner radius for
  every panel (26 px), and capsules for every control: buttons, bars, pills, segmented
  controls, search fields.
- **Keycaps look like real keys**, white with dark text: a white top face (`#FFFFFF` to
  `#F3F3F6`) sitting on a visible grey skirt below it (`#D8D8DE` to `#B9B9C2`, about 5 px), a
  6–7 px corner and a soft shadow under the key. Picked from twelve variants (flat capsule,
  dark Mac key, Magic Keyboard, mechanical, glass, white glass, tilted, real proportions,
  pillow, pressed-when-on). Two nodes in Bevy: the skirt, and the face inside it.
- **The accent is white** (`ACCENT` = `#FFFFFF`), chosen side by side against Apple's blue,
  indigo, teal, pink and green: the amber read badly over the glass, a light yellow over the
  game's light sky. With a white accent, what is selected or primary stands out by its fill
  (a white capsule with dark text), not by a colour; `DANGER` red stays for what deletes.
  Everything inside a white fill takes the dark ink, its icons too. Large numbers such as the
  countdown are solid white. `docs/template.md` still calls `ACCENT` the amber, which row 0
  corrects when it lands.
- **Modals use a thicker, colourless glass** over a dimmed game: the clear glass of the
  panels let the blue sky tint an alert. Corner panels keep the clear glass.
- **Legibility over glass wins over macOS's faint greys:** secondary text (`TEXT_MUTED`) is
  white at about 74%, not 55%, with a soft dark shadow under it; a toolbar's controls (back
  and forward, the path, the search field) are full white.
- Each tool copies a real macOS piece: the keys panel a popover with white keycaps; slow
  motion and speed the new volume slider capsule (icon, value, a bar of steps; an hourglass
  and a sport shoe); photo mode's hint a glass bar and "Photo saved" a notification; the file
  explorer a Finder window (shaded traffic lights, a sidebar floating inside the window,
  columns, Quick Look, context menu); recording the Cmd+Shift+5 bar, its settings clicked in
  place, a round countdown and a REC pill; the prop editor a Finder icon view with a segmented
  control; scenes an inspector list with + / − buttons; the unsaved-scene dialog a macOS alert.
- **Cost:** today's blur blurs the whole screen when a menu opens. Glass behind each panel needs
  a new piece: a UI material showing a blurred copy of the screen inside each panel's rectangle.
  The specular rim can be a gradient border. Refraction (the glass bending what is behind it)
  is left out: it needs a far more complex shader than the look is worth.
- **The glass must stay cheap**, since this is a template for games and its own UI should
  never cost the game frames. Agreed on 2026-11-07, measured in the spike below on 2026-11-08:
  - **One blurred copy per layer**, which every panel of that layer reads inside its own
    rectangle: ten panels side by side cost what one does, plus a texture read each. The
    layers are the named `*_Z_INDEX` roots (overlays, the pause menu, dialogs), and each
    layer's copy is taken just below it, the layers under it included. Agreed on 2026-11-08,
    so glass shows what is really behind it, as on macOS: the "Quit without saving?" dialog
    shows the pause menu blurred under it, not the game as if the menu were not there. One
    copy of the game for every layer was turned down for that. It costs one more chain per
    open layer, almost always one, and a stacked layer is mostly over a frozen game, so its
    copy is taken once too.
  - **Blurred on a smaller copy, with a dual Kawase blur** instead of today's Gaussian: a blur
    throws the detail away anyway, so the smaller copy looks the same. The copy is halved until
    it reaches a fixed height, one halving more at 4K than at 1080p, so it is always about the
    same size and the blur costs the same on every screen. Since a Kawase's strength is its
    number of levels, the same rule also gives the blur the same look at every resolution.
    The height is tuned in `docs/specs/glass/design.md` to match today's blur; it was first
    written here as "about 540 px", which is a single halving below 1080p and far too weak.
  - **The glass comes in with its panel's opacity**, not with a growing blur: the Kawase runs at
    full strength from the first frame, and the panel fades in over `ENTRANCE`, so the GPU's
    alpha blending mixes the sharp game behind with the blurred glass. `BLUR_RAMP_SECS` goes,
    and the entrance and the glass can no longer drift apart. Shrinking the Kawase's sample
    offset instead was turned down: at no offset the copy is still small, stretched and
    blurred, so the animation would start with a jump.
  - **Only where glass is**, agreed on 2026-11-08: every pass draws inside a list of regions
    instead of the whole screen, so a lone window open while playing (a HUD, the prop editor)
    blurs about its own area, not the screen's. That is the one glass blurred every frame,
    and at 4K a 400 × 600 window is 3% of the screen: about 0.7 ms on a weak GPU becomes about
    0.03 ms (worked out, not measured). A full-screen blur is a region the size of the screen.
    - Each region is its windows' rectangle **plus a margin of the blur's radius**: a blur
      reads its neighbours, and a crop without them leaves the glass's edges smeared.
    - **The number of levels comes from the screen, never from the region**, or a small
      window would get fewer levels and a weaker blur than a large one.
    - **The textures stay screen-sized**, made once and remade only when the window resizes;
      a region only limits where a pass draws. Panels keep reading the copy at their screen
      coordinates.
    - **Windows that touch or overlap share one region**, so no pixel is blurred twice and
      no margin is paid twice; windows far apart get one region each, so the space between
      them is not blurred. Turned down: one chain of textures and passes per window, which
      blurs overlaps twice, pays a margin per window, and remakes its textures whenever a
      window changes size, the entrance's growing card included.
    - Built in two steps: first the list always holds one region, the rectangle around every
      window, to learn the Kawase without the grouping; then the grouping, a pure function from
      rectangles to rectangles with its own tests. Nothing of the first step is thrown away.
  - **Only while glass is on screen**: with no panel open the passes do not run, as
    `update_blur` already does today.
  - **Blurred once while nothing behind moves**: the pause menu and photo mode freeze the
    world, and the main menu's background is a still image (`menu/background.jpg`), so the copy
    is made once and reused until the window resizes. Under the pause menu the game's camera
    can even stop drawing, which makes the paused game lighter than the running one.
  - **A "Reduce transparency" setting** (macOS has the same), which draws every panel as a
    solid dark fill with no blur: the way out for a weak PC, and an accessibility option.
  - The dev tools cost nothing in a released game: they live behind `feature = "dev"`. The only
    glass seen *while playing* is a HUD left open, which is the case to measure.
  - Estimate, not measured: under 0.1 ms per frame on a common GPU, against the 16.6 ms of a
    frame at 60 fps.
- **Spike before the spec, done on 2026-11-08** (on `3d/player`, never committed, thrown away
  with `git restore`). It measured today's blur in the 3D level with vsync off
  (`PresentMode::AutoNoVsync`) and `RenderDiagnosticsPlugin` + `LogDiagnosticsPlugin`, which
  log each render pass's GPU time:
  - **The FPS overlay cannot see the blur.** Opening the pause menu did not move the 100 fps:
    the frame is about 10 ms, of which the GPU works about 2 ms, so the game is held back by
    the CPU and the blur fits in the GPU's idle time. The pause also stops the physics, which
    takes CPU time away while the blur adds GPU time. Compare GPU milliseconds per pass, never
    frames per second.
  - **Today's full-screen Gaussian costs about 0.16 ms** of GPU (0.08 ms per pass) at about
    1920 × 1009, against about 2 ms for the whole GPU frame (irradiance 0.40, SSAO 0.29,
    atmosphere 0.23).
  - A pass that stops running keeps logging its last value, so a number repeated to the
    last digit across seconds is a pass that did not run.
  - Worked out, not measured: at 4K on a GPU ten times weaker, today's Gaussian would need a
    sigma of 18 to look the same and cost about 14 ms, a frame lost. On the smaller copy it
    drops below 1 ms, most of it the first halving, which reads the whole 4K image once and
    which any blur pays. After that, a Gaussian on the small copy would cost about 0.2 ms and
    the Kawase about 0.1 ms. The Kawase was chosen for that margin and to learn how it works.
  - Not built: the Kawase itself and the panels reading the copy. Whether a UI material can
    read the blurred copy inside its own rectangle is the spec's first step.
- **Icons: Lucide** (ISC), chosen over Phosphor side by side in the browser. Used as its icon
  font: the `.ttf` goes in `assets/fonts/` with its license file, next to Noto Sans JP's
  `OFL.txt`, and an icon is a `Text` with that font, sized by `TextFont` and coloured by
  `TextColor`. No crate, sharp at any size. Icons are larger than the text beside them: 19 px
  next to 13 px text, 32 px in a HUD (picked over 15 px, a thicker custom-built stroke, and
  Phosphor Bold), with the font's own stroke. Slow motion is `hourglass`, speed `sport-shoe`,
  which needs Lucide 1.70.0 or later (0.460 lacks it; every other icon used is in 1.70.0).
  Not SF Symbols: Apple's licence only allows them in apps for Apple platforms.
- **Three specs, not one**, agreed on 2026-11-08: the row is too large for one plan of one
  commit per step. In this order:
  1. **Glass**: the technique (the per-layer copies, the Kawase, the regions, the material
     panels read it with), proven on one plain panel.
  2. **Look**: the colours, the Lucide font, keycaps and capsules, and every existing screen,
     overlay and dialog redone in it.
  3. **Pieces**: what only the dev tools use (popover, HUD, bar, list, alert).
- **Tried on `3d/player`, committed on `main`**, agreed on 2026-11-08: on `main` nothing moves
  behind a panel (the menu's background is a still image, gameplay is empty), so each step is
  written and tried uncommitted on `3d/player`, where the level moves behind a window, then
  carried to `main` (`git stash`, `git switch main`, `git stash pop`) and committed there, and
  reaches the branches below by the usual merge down. The `src/ui/` files are the same on
  both branches, so the stash carries cleanly. Before each commit on `main`:
  - `just check` and `just run` on `main` too, since its cameras have no `Hdr` and keep
    their MSAA, which `3d/base`'s `match_ui_cameras_to_playroom_camera` changes in gameplay;
  - nothing of the step may lean on `3d/player`: a change that `playroom.rs` needs belongs
    to `3d/base` and is committed there, after the merge.

## Key bindings, as agreed so far (row 1)

- A Controls page in Settings lists the actions and the key each one is bound to, and lets
  the player change them. It is game UI, so it takes the Liquid Glass look (row 0) and its
  words are locale keys.
- Placed right after the look on purpose: every row after it reads the key bound to an action,
  never a `KeyCode` written in the code, so no row has to come back later to swap them.
- The base lives in `main`: the page, the bindings saved in the `settings.ron` that already
  exists, and a list of actions each feature registers its own into, the way each dev tool
  adds its line to the keys panel (row 2). Branches below register theirs (walk, jump, fly,
  swap shoulder, open door...).
- **Two tabs:** the game's controls, and a Dev tab with the dev tools' keys (F1 to F10,
  Numpad 0, Numpad − / +), rebindable the same way. The Dev tab exists only in dev builds:
  release builds leave the dev tools out, so the tab is behind `#[cfg(feature = "dev")]` too.
  Its words are English, like every dev tool's.
- Claude's note for the spec: once dev keys are rebindable, the keys panel (row 2) shows the
  bound key, not the default one.
- **Gamepads too:** every game action can be bound to a gamepad button or stick as well as a
  key, so this row designs bindings for both from the start (Bevy reads gamepads itself, no
  crate). Claude's note for the spec: playing with a gamepad also means moving through the
  menus with it, which the `ui-style` spec left out as "its own spec later" (focus and
  directional navigation, `bevy_input_focus`); that spec is this row's or a row of its own.

## Photo mode, as agreed (row 3)

Only a photo mode: the photos are browsed in the file explorer (row 4), not from here.

- F3 enters and leaves; Esc leaves too, instead of opening the pause menu. Gameplay only.
- The game freezes without the pause menu (not in a multiplayer game, where the world keeps
  running: row 18). Every dev window and the FPS hide.
- The cursor stays locked and moves the free camera; leaving puts the camera back where it was.
  In an Empty game there is no camera to move: it freezes and takes the photo as it is.
- The 3D free camera (row 5) moves like fly: WASD where it looks, Space rises, Shift sinks.
  The 2D one (row 6) pans with WASD and zooms with the mouse wheel. Both at a fixed speed, not
  row 7's multiplier: photos are taken close to the player.
- E takes a photo of the game alone (dev UI hidden for that one frame, back on
  `ScreenshotCaptured`), into the game's data folder, named by date and time. A short
  "photo saved" notice shows, and never lands in a photo.
- A hint line on screen: "E: take photo · F3: exit".
- Bevy 0.20's `EasyCameraMovementPlugin` only glides a camera to a target: the free camera is
  ours.

## File explorer, as agreed (row 4)

Numpad 0 opens and closes it. It shows the photos (row 3) and, once row 8 exists, the videos.
It works like the Windows explorer, with no buttons except "+ Folder":

- A list on the left (folders first), a big preview of the selected file on the right.
- One click selects, a double click opens the file in the system's default app (viewer,
  player) or enters a folder.
- The title is the path (`Gallery › trailer`), each part clickable to go back. `Gallery` is the
  root: the folder photos and videos land in, and every folder made from here lives inside it.
- The sidebar shows that as a tree: Recents, then `Gallery` with its folders indented under it,
  the folder being shown highlighted. Never the folders as Gallery's siblings.
- Right click on a file: Rename, Delete. Right click on empty space: New folder.
- Drag a file onto a folder to move it there (`Pointer<DragStart>`, `Pointer<DragDrop>`;
  the right button is `PointerButton::Secondary`).
- Renaming reuses the text field the save screens already have.

## Dev tools 3D, as agreed (row 5)

**Physics debug view (F6).** Shows and hides the outline of every collider, the player's
capsule included: from first person its line shows from inside, which is enough to measure
the hitbox. Starts off. `hide_body_debug_collider` is deleted.

**Fly (F7).** For going high and far, to judge how far the map can be seen (fog, clouds).

- Noclip: the player passes through walls, ceilings and floors.
- No gravity, for the player's body only; the rest of the world keeps it.
- WASD moves where the camera looks, up and down included; Space rises, Shift sinks.
- Speed is a fly base speed times row 7's speed multiplier, the same one walking uses. Until
  row 7 exists, the base speed alone.
- Turning it off gives gravity back and the player falls. Turned off inside a wall, avian
  pushes the body out as it can: a dev tool, no code for that case.

## 2D player, as agreed (row 6)

`2d/base` today: a room seen from above, drawn in tiles from an ASCII map (1024×640 px, fits the
1280×720 window), no input, no collisions, drawn by the screen camera that also draws the UI.

- **The body:** a solid-colour square one tile big (64 px on screen), spawned in the middle of
  the room. No animation.
- **Walking:** WASD, four directions and the diagonals (diagonals no faster), seen from above.
  The speed is a pixels-per-second constant that row 7's multiplier scales later.
- **Collisions:** none for now; the square walks through walls and crates.
- **Camera:** a dead zone. It stays still while the square is in the middle of the screen and
  follows once the square gets near an edge. With today's room it only moves once the square
  walks out through a wall; a bigger map uses it fully. Moving the camera never moves the UI.
- **Saved:** the position goes into `GameData::player`, as in 3D, so Continue resumes there.
- **Photo mode's 2D free camera:** WASD pans, the mouse wheel zooms in and out (the camera's
  orthographic scale). Fixed speed, as in 3D. Leaving puts position and zoom back.
- **The CLI:** picking 2D asks one more question, without or with the player module, which
  clones `2d/base` or `2d/player`. A change to `create-bevy-starter` that ships with this row.

## Speed + UI, as agreed (row 7)

- **F8** opens and closes a small window showing the player's current speed (m/s in 3D, px/s
  in 2D) and the multiplier under it (`3×`).
- `main` holds the window and the multiplier; each player writes its current speed and reads
  the multiplier. Fly (row 5) reads it too; photo mode's free cameras do not.
- Numpad − and + change the multiplier while the window is open, the same keys slow motion
  uses: so the two windows never stay open together, and opening one closes the other. Since
  closing slow motion puts time back at 1×, pressing F8 with slow motion open does too. This
  row writes the closing both ways, since the speed window does not exist yet when row 2 is
  built.
- Closing the window puts the multiplier back at 1×, as slow motion does with time, so nobody
  has to reopen it just to make things normal again.
- Fixed steps: 1×, 2×, 5×, 10× (Claude's call; the user left the values open).
- No CCD against tunneling: at 10× the walking capsule may pass through a thin wall between two
  physics steps. Left alone on purpose: whether a game needs CCD is its developer's call.

## Screen recording, as agreed so far (row 8)

- **The flow:**
  1. F9 opens a glass bar at the bottom and frees the cursor. No Options menu: one click on
     the sound turns it on or off, one click on the frame rate swaps 30 and 60, and only the
     timer opens a small list (none, 3, 5 or 10 s, Claude's guess for the values). The bar
     remembers the last choice.
  2. F9 again closes it without recording, to walk closer to where the shot starts.
  3. Space starts: the window goes, the cursor locks again, and a big number counts down in
     the middle of the screen while the player gets in position. While the window is open the
     game ignores Space, so it never jumps (row 10) or rises (fly, row 5) instead. F9 during
     the countdown cancels.
  4. At zero the recording starts: a red circle blinking next to "REC" in a corner.
  5. F9 stops it and writes the file; the indicator goes. The bar never turns into a Stop
     button: it is gone before recording starts, and the REC pill's stop icon is only a hint,
     since the cursor is locked while playing and nothing can be clicked.
- The video shows the game alone, its own UI included, never the dev UI (REC, FPS, dev
  windows): it must serve as a trailer. Unlike a photo, the dev UI cannot hide for one frame,
  so this row moves it to a camera of its own that the recording leaves out (Bevy's FPS
  overlay included). Row 2 keeps them on the screen camera until then.
- The videos land where the photos do, so the file explorer (row 4) lists and opens them.
- Gameplay only, as photo mode, and never inside photo mode: they are separate tools, so F9
  does nothing while photo mode is on.
- The video has the game's sound by default: a trailer needs it.
- Closing the game while recording stops it and writes the file whole.
- Slow motion while recording is wanted: the video comes out in slow motion, and the sound
  stays at normal speed, since slow motion never touches sound.
- No ffmpeg or anything installed outside cargo (likely `openh264`, compiled by cargo).
- **Risk, to try in a throwaway spike before the spec:** sound is the hard half. Bevy's audio
  goes through `rodio` (0.22) straight to the sound card, with no ready point to listen to what
  plays. Candidates: tapping the mix inside the game, or capturing what the sound card plays
  (`cpal`, 0.17, can record an output device on Windows, but it hears every program, not only
  the game). Then the sound needs an encoder cargo can build and a container that holds both.
- Bevy 0.20's `EasyScreenRecordPlugin` does not work on Windows (bevy#22132), and its `x264`
  needs a library installed on the system: the recorder is ours.

## Lighting, as agreed (row 9)

- The goal is that dark places *can* be dark: today the sky light
  (`AtmosphereEnvironmentMapLight`) lights every interior evenly, so `DarkRoom` and
  `DarkCorridor` never get dark whatever the values.
- How dark, and how bright `DarkRoom_LightSpot` is, the user tunes by eye in `just run`; the
  spec only names the constants to tune.
- How the sky light is kept out of interiors is the spec's research (Bevy 0.20's light probes
  are the first place to look).

## Character controller, as agreed so far (row 10)

- The feel the user likes: *Wardogs*, tactical but very dynamic. The details are decided when
  the row starts.
- Whatever key jumps, it must stay ignored while the recording window is open if it is Space
  (row 8).

## Ladders, as agreed so far (row 11)

- Climbing starts by walking into the ladder, no key: more dynamic.
- First version: walking into it teleports the player to the top. Then real climbing.
- **Climbing:** W goes up and S down, wherever the camera looks. Holding Shift climbs faster,
  both ways.
- **At the top** the player gets onto the platform by itself: for now a teleport onto it,
  the place where an arms animation goes later.
- **At the bottom** the player lets go by itself on reaching the floor.
- **Midway**, the jump key lets go of the ladder.
- **Going down from the top** catches the ladder both ways: walking backwards off the edge, or
  facing it.

## Doors, as agreed so far (row 12)

- Simple, like older games: near a door a prompt to press E shows; E opens it, E again closes
  it. No hand on the knob, no pushing: the door just moves.
- The prompt is the E key alone, no words: a white-outlined keycap just under the crosshair.
  It is the game's UI, not the editor's, so it does not take the dev tools' glass look. With
  no words it needs no locale key.
- The door swings on its hinge, not a jump from shut to open. Claude's guesses: always the same
  way for a given door, and a duration constant tuned by eye.
- For the spec: the doors are part of the playroom's static trimesh today, so a door that
  swings needs a collider of its own that turns with it.

## Prop editor, as agreed so far (row 13)

- For building the map and trailer scenes. In a multiplayer game it is shared, so friends
  build the map together (row 18).
- **F10** opens the props window and takes the camera off the player: a free camera, as in
  photo mode, so the map can be edited from anywhere, the way Unreal's editor works. It reuses
  photo mode's free cameras (rows 5 and 6).
- **Mouse, as in Unreal:** holding the right button turns the camera with the mouse; releasing
  it shows the cursor, to click and drag props.
- **WASD always flies**, right button held or not (Space and Shift for up and down), unlike
  Unreal: the editor has no other use for those keys.
- **The game keeps running** while the editor is open, so no code has to stop it. Claude's
  guess: props are static for now (no rigid body), so a prop put down stays where it was put.
- **The props window shows thumbnails**, as Unreal's does. In 2D the image is its own
  thumbnail; in 3D each model is rendered once into a small image by a camera of its own
  (Bevy cameras can render to an `Image`). Whether thumbnails are cached on disk is the spec's.
- **Three tabs:** all props, favourites, and the most recently used.
- **Favourites:** any prop can be marked and unmarked as a favourite (Claude's guess: a star
  on its thumbnail).
- **The full list** has a search bar (reusing the save screens' text field) and sorts by name
  or by file size.
- Claude's guess: favourites and recents are the developer's preferences, not the game's, so
  they live in a file in the data folder, not in `assets/`.
- Where every prop stands lives in a **map file**. Every move in the editor saves to it, so the
  game opens with each prop where it was left.
- The map file is part of the game, so it lives in `assets/` and is committed; dev builds
  write it, release builds only read it. Not the data folder, which is the player's.
- **Where props come from:** `assets/props/`. Anything put there shows in the props window
  live, without closing the game (dev builds already hot-reload assets; whether a folder
  loaded with `load_folder` picks up a new file is the spec's to check).
- **Dropping files:** a file dragged from the Windows explorer onto the game window is copied
  into `assets/props/` and shows up like any other (Bevy's `FileDragAndDrop::DroppedFile`).
- In 3D a prop is a model (glb); in 2D, an image. Which formats are accepted is the spec's.
- **Editing, with the mouse:** a click on a prop sticks it to the mouse; moving the mouse moves
  it; another click puts it down.
- **R** rotates the prop while it is stuck to the mouse.
- **Delete** removes the prop under the mouse, or the one stuck to it.
- Claude's guesses: in 3D a held prop slides over whatever surface is under the cursor (a ray
  from the camera); R turns it around the vertical axis in fixed steps (in 2D, in the plane);
  Esc while holding puts it back where it was picked up.

## Trailer scenes, as agreed so far (row 14)

- A scene is a copy of the map files (row 13), saved under a name to load back later, plus
  where the player stands and looks (later, the weather too).
- Scenes are picked from a window listing them: a window of its own in a corner of the screen,
  opened along with the props window by F10, so it needs no key.
- Its buttons: save scene, the chosen scene, delete scene, rename scene.
- **The map is the base the scenes sit on**, never overwritten by them:
  - Choosing a scene puts the map aside somewhere safe and shows the scene.
  - Unchoosing it brings the map back exactly as it was left. Nothing is lost, so no dialog.
- **With a scene chosen, edits go nowhere until "save scene" is clicked.** The map is not
  touched either. An indicator shows the chosen scene has unsaved changes (Claude's guess: an
  asterisk after its name, `scene 1 *`).
- Unchoosing a scene, or choosing another, while it has unsaved changes asks first, saying the
  scene is not saved (the `dialog()` the quit confirmation uses).

## Third-person camera, as agreed so far (row 15)

- Over the shoulder, shooter style. The user's reference: *Helldivers*.
- A key swaps shoulders: **V** (Claude's call; the user left the key open). A game key, not a
  dev key, so it is not on the keys panel.
- Claude's guess: the camera pulls in when a wall comes between it and the player, so it never
  shows the inside of a wall (a ray from the player to the camera).

## Isometric camera, as agreed so far (row 16)

- Completely fixed: one angle, no turning. The user's reference: *Project Zomboid*'s view, in
  3D here (Zomboid draws 2D sprites).
- **Perspective projection**, not orthographic: farther things get smaller, as in Diablo or
  Hades, which the user finds prettier than the flat classic isometric look.
- Claude's guess: WASD moves relative to the screen (W is up on the screen), since the camera
  never turns.

## Camera switch, as agreed (row 17)

- F5 goes to the next camera in order (first person → third person → isometric → first person),
  skipping the ones the game was made without. Only built when the game has two or more.
- The chosen camera is kept in the save, so Continue resumes with it.
- Unlike the other F-keys it is a game key, in release builds too, so it is not a dev key; it
  sits in the Keys table only so no dev tool claims F5.

## Multiplayer, as agreed so far (row 18)

- **No architecture fits every game**: a listen server (one player hosts and plays, 2 to ~8+),
  a dedicated server (a headless build, 16 to 100+), and peer-to-peer rollback (2 to 4,
  needs a deterministic simulation, which avian's physics makes hard).
- **First: co-op for up to 10 players, one of them hosting** (a listen server). The others join
  by the host's IP, over a LAN or Radmin VPN, which only makes the PCs see each other as one
  local network; it does not limit the player count.
- A listen server and a dedicated server share the same code (the host is a server with a
  player in it), so a dedicated server can come later as another option. Rollback is a
  separate world and is not planned.
- **Hosting:** in gameplay, the pause menu (Esc) has an "Open to LAN" switch, on and off.
- **Joining:** the main menu gets a "Find server" button opening a list of the games open on
  the network, found by themselves (a LAN broadcast). The list refreshes itself every few
  seconds, and a refresh button at its top does it at once.
- **Esc never freezes a shared game:** with LAN open (and on every client), the pause menu
  opens without pausing `Time<Virtual>`; the world keeps running and only that player stands
  still. Today's pause freezes time, so this row splits "the menu is open" from "time is
  frozen", which `docs/flow.md` describes as one state (`Pause`).
- **Saving, as in Valheim:** the host saves the map (the world) and their own character; a
  player who joined saves only their own character, on their own PC. Today one save holds
  both (`GameData`, with `GameData::player` inside), so this row splits a save into a world
  part and a character part.
- **The character is portable:** the same character joins any friend's game. Claude's guess:
  the character file remembers where it stood in each world (keyed by a world id), so
  rejoining a world puts it back there, and a first visit spawns it at the world's spawn.
- **Other players look like the player:** the same capsule, with no name above it. Players
  have no name at all.
- When the host leaves or closes the game, everyone else goes back to the menu with a notice
  that the host ended the game (Claude's guess, not contested).
- Claude's guesses: discovery needs the network to carry broadcasts, which Radmin VPN is made
  to emulate for LAN games, to confirm in the spike; and a "join by IP" field as the fallback
  when a game does not show up.
- **The crate: `lightyear`** (the user's call). Checked on crates.io on 2026-11-07: 0.31.0 is
  the latest and depends on Bevy `^0.20`. Its licence goes through `about.toml` before it is
  added.
- **Old saves stop loading, on purpose:** splitting a save into world and character changes its
  format, and a template has no players whose saves must survive. The spec says so, so nobody
  writes a migration by habit.
- **Probably an option, not a branch:** multiplayer combines with 2D, 3D and every camera, and
  git cannot add up sibling branches. Likely it lives in the branches as files removed by a
  marker (`// multiplayer`), as the cameras are, and the CLI asks for it. Its spec decides.
- **Dev tools work in multiplayer too, each for whoever used it**, except:
  - **Slow motion is off** in a multiplayer game: the host runs the world, and one player
    slowed down would fall further and further behind it.
  - **Photo mode does not freeze the world** in co-op: the free camera flies while the world
    keeps running.
  - **The prop editor is shared**, so friends can build the map together: an edit goes through
    the server to the map file on the host's PC, and every player sees it. This also gives the
    server a first, basic sync to build on. Claude's guess: a prop held by one player is
    locked for the others until it is put down.
  - **The server sends prop files:** a prop the host adds (dropped into the host's
    `assets/props/`) is sent to every friend, whose game saves it in their own
    `assets/props/`, so nobody keeps the folders in step by hand. Dev builds only.
- **Receiving files is a door into the PC, so the spec must close these** (the user agreed;
  at worst a hostile friend can crash the game, never touch the PC):
  - **The name:** only the file name is kept; a name holding `/`, `\` or `..` is refused, and
    the file is always written inside `assets/props/`.
  - **The kind:** only prop extensions (`.glb`, `.png`, the spec's list) are accepted. The game
    never runs a received file, only loads it as a model or an image.
  - **The size:** a cap per file in a `Config` constant (about 50 MB).
  - **A crafted file** that breaks Bevy's loader cannot be ruled out; at worst the game
    crashes loading it. Hence: only join games of people you trust.
  - **A half file** from a dropped connection: written with `write_atomic`, so a file is
    either whole or absent.
- Rough numbers for later: past 20 to 30 players a listen server runs short (the host's upload
  and one PC running both the server and a player's game), which is where a dedicated server
  starts to pay.

## Text chat, as agreed so far (row 19)

- Wanted, for multiplayer games. Its details are decided when the row starts. It reuses the
  text field the save screens have, and the Liquid Glass look.

## Voice chat, as agreed so far (row 20)

- Wanted, for multiplayer games. Its details are decided when the row starts.
- **Risk, to try in a throwaway spike first**, the same kind as recording's sound (row 8):
  the microphone can be read with `cpal` (built by cargo), but the voice needs an encoder
  cargo can build on its own; the usual one, Opus, is a C library, and the rule is that
  nothing gets installed outside cargo.

## Release 2.0, as agreed (row 21)

- Last, since it ships everything above: tag `v2.0.0` on `main` and every branch below it.
- `README.md`: the features, the commands (`just profile`), the branch table with the new
  branches (`2d/player`).
- A new `create-bevy-starter` on crates.io, from its own repository, asking the questions
  the rows added: the 2D player (row 5), the cameras (rows 15 to 17), multiplayer (row 18).
  Its `just check-combos` builds every combination before it ships.

## Rules that hold for every row

- The repository runs as cloned: nothing installed outside cargo.
- Whatever goes into `main` must make sense in an Empty game.
- Every dev tool's text is English in the code, not locale keys: release builds leave the dev
  tools out, so no player reads it (the user's call, 2026-11-07). Text the player reads stays
  a locale key.
- Speeding time up is out; it would only earn its place with rewind, which is out too.
- Every row must keep working with `create-bevy-starter`: options a game combines freely are
  files removed by markers, not branches.
