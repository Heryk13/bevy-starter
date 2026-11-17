# Look: every screen the template has, in Liquid Glass

Status: **approved** — written 2026-11-17, approved 2026-11-18, not built yet. Its tasks are
in `docs/roadmap/look-tasks.md`.
Date: 2026-11-17
Bevy: 0.20.0

Row 0 of `docs/roadmap/roadmap.md`, second of its three specs (glass, look, pieces). The
roadmap's "The look, as agreed so far (row 0)" holds the reasons behind the palette, the white
accent and the icon font; this design does not repeat them, it builds on them. The screens as
approved are in `docs/roadmap/ui/look-screens.html` (open it in a browser: the buttons answer
the pointer).

## Goal

The glass spec changed how a card is drawn and left what is on it alone: amber on hover,
bordered pills with a glow, checkboxes and radios, a grey secondary text tuned for an opaque
card. This design repaints all of it for glass and redoes each screen the template already has:
the menu, the pause menu, Settings, Save Game, Load Game, Credits and the dialogs. It adds one
thing that is not paint: every save shows a picture of the game at the moment it was saved.

Done looks like this: `just run`. The menu is the game's name over a column of plain greyish
words, and the word under the pointer sits on a white bar that fades out to the right. New
Game, walk a little, Esc: the pause card is a clear dark glass with a 26 px corner and a bright
rim at its top left, its buttons capsules that turn white with dark letters under the pointer.
Save: the list shows each save beside a small picture of where it was made.

## Decided

Decided by the user on the mockup unless marked *(Claude's call)*, which the user saw in the
mockup and did not object to.

| Question | Decision |
| --- | --- |
| The menu | No card, no capsules, no icons. The words alone, greyish; under the pointer a white bar, solid on the left and fading out to the right, and the word turns dark. Title, scrim and version stay |
| Cards | The thicker, colourless glass of a modal: 26 px corner, a specular rim bright at the top left and dark at the bottom right, the fill darker toward the bottom *(Claude's call, from the roadmap)* |
| Buttons | Every button is a capsule with no border and no glow. Under the pointer it takes the white fill and the dark ink *(Claude's call)* |
| The main action | White already at rest: New Save, and a Yes that destroys nothing, like the "Save" of the alert approved in the first mockup *(Claude's call)* |
| What destroys | Red ink at rest, red fill with white ink under the pointer: Delete, and a Yes that destroys something |
| Dialogs | The two answers centred under the question, not at the right |
| Settings | The two checkboxes become switches, the three language radios one segmented control, the volume slider white *(Claude's call)* |
| Load Game's Delete | A round button holding a trash icon. The word goes, and its key with it |
| Overwriting a save | Asks first. Already built (`ask_to_overwrite_save`); it only takes the new look |
| A save's picture | Each row of Save Game and Load Game shows a small picture of the screen when it was saved. A save with none (older, or the capture failed) shows an empty frame |
| Where the picture comes from | A screenshot of the window, taken the frame the pause is asked for, before the pause menu exists. Chosen over a second camera drawing into a texture (each genre would supply one, `main` has none) and over reading back the glass's copy (the hardest plumbing of the three) |
| Icons | Only where one replaces a word: the trash. The pause menu and the Settings labels stay words, like the menu *(Claude's call: the user asked for the menu without icons and did not answer for the pause)* |
| The pause menu | Keeps its card and its column, in capsules *(Claude's call: asked twice, not answered)* |
| Type sizes | The game's own stay: 44, 26, 18, 14. The 13 px of the first mockup is the dev tools' size |
| Where it is built | Tried uncommitted on `3d/player`, committed on `main`, merged down (roadmap, "Tried on `3d/player`, committed on `main`"), each step with `just tools/sync main "<message>"` |

## Must not

- **Text that is hard to read.** The new card is thinner than today's (about 56% against 85%),
  and the secondary text is white. Both must read over the brightest things the template has:
  the 3D level's sky behind a card, and the menu's artwork, which is nearly all white, behind
  the menu's words. The menu's scrim stays for that reason. Each step that touches a colour
  is looked at on `3d/player` against the sky, and under Reduce Transparency, where a card
  must still be a solid, readable fill.
- **A picture with the menu in it.** A save's picture shows the game, never the pause card, an
  overlay or the blur.
- **A hitch.** Saving must not stall a frame the player can feel while playing: the autosave
  runs mid-game. Opening the lists must not stall either with the handful of saves a normal
  player keeps.
- **A flickering bar.** A pointer crossing the menu's column leaves a short trail, as the
  buttons do today (`BUTTON_EASE_SECS`), never a blink.

## Out of scope

- **Keycaps.** The roadmap lists them under this spec, but no existing screen shows a key, and
  a widget nobody calls is dead code, which fails `just check`. They come with the first
  screen that shows one: the Controls page of row 1, or the keys panel of the Pieces spec.
- **The pieces only the dev tools use** (popover, HUD, bar, list, alert with an icon) and
  **the regions**: the Pieces spec.
- **Moving through the menus with the keyboard or a gamepad.** Row 1. The paint a button takes
  under the pointer is the one a focused button will take, so nothing here is thrown away.
- **Fading a card's text and buttons with its entrance.** Today only the glass and the card's
  place ease in; that stays.
- **New art for the menu**, and a real "cover" for it: the `ponytail:` note in `menu.rs` stays.
- **A language picker that holds any number of languages.** The segmented control fits three
  or four names; see Settings below.

## Facts this rests on (read in the 0.20.0 source and in this repository)

- **Screenshots.** `Screenshot::primary_window()` (`bevy_render::view::window::screenshot`)
  is a component: spawned on a new entity, it captures that frame, and an observer on the
  entity receives `ScreenshotCaptured { entity, image: Image }` some frames later. The entity
  is despawned afterwards. A window screenshot is what the window shows, after tonemapping,
  so `3d/player`'s `Hdr` camera needs nothing special.
- **The pause is asked in one place**: `apply_esc_action` in `src/overlays.rs` sets
  `NextState<Pause>`. The pause menu spawns on `OnEnter(Pause::On)`, which runs in the next
  frame's state transition, so a screenshot spawned beside the `set` captures a frame with no
  card. A card's text is drawn at full strength from its first frame, so `OnEnter` would be a
  frame too late.
- **Saves.** `write_save` (`src/save.rs`) is called from three places: `write_new_save` and
  `overwrite_save` in `src/overlays/saves.rs`, both under the pause menu, and `autosave_game`
  in `src/screens/gameplay.rs`, mid-game. `list_saves` reads only `.ron` files, so a picture
  beside a save is never taken for one. `delete_save` removes the one file.
  `write_atomic` (`src/disk.rs`) takes text and names its temporary file `.ron.tmp`.
- **Encoding without a new crate.** `Image::try_into_dynamic()` returns the `image` crate's
  `DynamicImage`, whose `crop_imm`, `thumbnail_exact`, `to_rgb8` and `write_to` are plain
  method calls. `write_to` wants the `image` crate's format, which
  `bevy::image::ImageFormat::Jpeg.as_image_crate_format()` hands over, so the code never names
  the `image` crate and `Cargo.toml` gains nothing. JPEG is already built (the `jpeg` feature,
  for the menu's background); PNG too, through `common_api`.
- **Decoding from the data folder.** The asset server reads only `assets/`. `Image::from_buffer(
  bytes, ImageType, CompressedImageFormats, is_srgb, ImageSampler, RenderAssetUsages)` turns a
  file's bytes into an `Image` for `Assets<Image>`.
- **Fonts.** `TextFont::font` is a `FontSource`, which a font handle converts into.
  `use_ui_font` (`src/i18n.rs`) replaces the default font from bytes built into the binary
  (`include_bytes!`), which is why `font()` and every widget are plain functions with no asset
  server. `uuid_handle!` (`bevy_asset`) makes a handle known at compile time.
- **Gradients and shadows.** `BackgroundGradient` (already used by the menu's scrim),
  `BorderGradient` and `TextShadow { offset, color }` exist in `bevy_ui`. A `TextShadow` is
  an offset copy of the text with no blur.
- **A node's material is drawn over its border.** `bevy_ui_render`'s `stack_z_offsets` put
  `BORDER` at 0.01, `BORDER_GRADIENT` at 0.03 and `MATERIAL` at 0.05, and the material's quad
  is the whole node, border included. So the card's `BorderColor` is painted over by its
  glass today, and a gradient border would be too. `UiMaterial::stack_z_offset()` can be
  overridden; `BOX_SHADOW` sits at -0.1.
- **The first window screenshot of a run** queues its own pipeline on the frame it is taken,
  and Bevy compiles pipelines in the background (`screenshot.rs`), so that first capture may
  skip drawing the window for a frame.
- **Buttons today.** `paint_buttons` (`src/ui/button.rs`) eases `ButtonMotion` and writes the
  fill, the border and the glow from the pure `button_colors(kind, hover, press)`. It never
  touches a label. `menu_button` and `row_button` are `ButtonKind::Primary`.
  `play_button_sounds` asks only for `Button` and `Hovered`.
- **Icons.** Lucide's font maps each icon to a private-use code point, listed in the release's
  `lucide.css`. `sport-shoe`, which row 7 needs, exists from 1.70.0.

## Structure

### Palette (`src/ui.rs`, `Config`)

Start values, taken from the mockup and tuned by eye in the build against the sky:

| Constant | Today | Becomes |
| --- | --- | --- |
| `ACCENT` | amber `#F2B544` | white `#FFFFFF` |
| `INK` (new) | | `#111114`: text and icons on a white fill |
| `DANGER` | `#E05C5C` | `#FF5F57` |
| `TEXT` | `#EDEDF0` | white at 92% |
| `TEXT_MUTED` | grey `#9EA0A8` | white at 74%, always with a soft dark `TextShadow` |
| `BORDER` | white at 12% | white at 16%: the hairline under a heading, around a field |
| `FIELD_FILL` | white at 6% | white at 8% |
| `SURFACE` | `#16171B` at 85% | `#2C2C32` at about 56% |

Muted text has more than three users today (the menu's version, the pause's status, Credits,
Settings, the save lists), each writing `TextColor(TEXT_MUTED)` itself. The shadow makes that two components that
must travel together, so `ui.rs` gains one widget for them, and `muted_label` in `saves.rs`
uses it.

The shadow is for muted text that stays light. Text that turns dark under the pointer (a save
row's details, the menu's words) would smear over its own shadow, which has no blur: a row's
details carry `TEXT_MUTED` alone, as in the mockup, and the menu's words fade their shadow
out with their own paint.

A constant lands in the step of its first user, since `just check` fails on dead code: `INK`
in step 3, each icon with the button or the frame that shows it.

### Cards (`src/ui/panel.rs`, `assets/shaders/glass.wesl`)

- `CARD_RADIUS_PX` 16 → 26.
- The rim: the card's `BorderColor` becomes a `BorderGradient` running from the top left to
  the bottom right, bright white to almost nothing to a dark edge. For it to show,
  `GlassMaterial` overrides `stack_z_offset()` to sit under the border and over the shadow:
  no shader is involved.
- The darker bottom: the card is already drawn by `glass.wesl`, so the shade is one more field
  of `GlassMaterial` (how far the tint darkens from the top of the node to its bottom, 0 for
  a backdrop) and one `mix` in the shader. It applies under Reduce Transparency too.
- `dialog()`: `JustifyContent::End` → `Center`.

### Buttons (`src/ui/button.rs`)

`ButtonKind` says what a button is for; `button_colors` turns it into a fill and an ink.

| Kind | For | At rest | Under the pointer |
| --- | --- | --- | --- |
| `Primary` | the main action: New Save, a Yes that destroys nothing | `ACCENT` fill, `INK` | a little dimmer |
| `Choice` (new) | one among equals: `menu_button`, `row_button` | white at 10%, `TEXT` | `ACCENT` fill, `INK` |
| `Quiet` | the way out: Close, No | no fill, `TEXT` | white at 18%, `TEXT` |
| `Danger` | what destroys | white at 10%, `DANGER` ink | `DANGER` fill, white ink |

- Pressing darkens whatever fill the button has. The border and the glow go:
  `ButtonColors` is a fill and an ink, and `button_frame` loses its border.
- **The ink reaches the label.** `paint_buttons` writes the ink's colour to every text under
  the button and keeps each text's own alpha, so a row's muted details stay muted on the white
  fill without a marker component.
- `Primary` keeps its name and changes meaning: today it is "the usual choice", which is what
  `Choice` now is. `menu_button` and `row_button` move to `Choice`; `button("save-new",
  ButtonKind::Primary)` stays and becomes the white one. The names are Claude's call.
- `row_button` takes a fixed corner (18 px) instead of the pill's: a row is tall, and a pill's
  ends would eat its picture.

### The menu (`src/screens/menu.rs`)

The menu's words are its own widget, in `menu.rs`: nothing else looks like them, and a part
with one user stays with it. Each is a `Button` with `Hovered`, so `play_button_sounds` covers
it, and no `ButtonKind`, so `paint_buttons` leaves it alone. A system in `menu.rs` eases it as
`paint_buttons` does and writes a `BackgroundGradient` (white, solid up to 30% of the width,
clear at the end) at the eased strength, and the word's colour from greyish (white at 62%,
with the muted shadow) to `INK`. The strength and the colour come from a pure function.

The column is 300 px wide (260 today) and its items touch (no gap), so the pointer never falls
between two of them. The words line up with the title, 80 px from the left, as in the mockup
(today a button's border and padding push them to about 101): the column starts at 60 and
each item pads its word by 20.

### Settings (`src/overlays/settings.rs`)

The controls stay in `settings.rs`: one user each.

- **Switch**, replacing `checkbox()` and `set_checkbox`: a 46 × 26 capsule and a 20 px knob.
  Off, the track is white at 16% and the knob white on the left; on, the track is `ACCENT`
  and the knob `INK` on the right.
- **Segmented control**, replacing `language_radio` and `RadioMark`: one capsule track, one
  segment per `Language::ALL`, the chosen one filled `ACCENT` with `INK` text, the others
  muted, in the small type size.
- **Slider**: a 6 px track, the fill `ACCENT`, an 18 px white thumb.
- `LABEL_WIDTH_PX` 140 → 190, so "Reduce Transparency" stays on one line.
- **Width.** Three names in a row beside a 190 px label may not fit `OVERLAY_WIDTH_PX` (520)
  in every language; the build measures it in English, pt-BR and Japanese and widens the
  overlays to what fits (about 560), which Save Game and Load Game share.
- A segmented control holds three or four names. A game with more languages needs a list
  instead; that ceiling gets a `ponytail:` comment where the control is built.

### Icons (`assets/fonts/`, `src/ui.rs`, `src/ui/button.rs`)

Lucide's `.ttf` (1.70.0 or later, the version written beside it) and its ISC licence go in
`assets/fonts/`, next to Noto Sans JP and `OFL.txt`. The font is built into the binary and
registered under a handle known at compile time, the way `use_ui_font` installs the UI font,
so an icon is a plain function like `label()`: a `Text` holding one code point, in the icon
font, 22 px beside 18 px text. Each icon used is a named constant in `ui.rs`'s `Config`, its
code point read from that release's `lucide.css`. This spec names two: the trash and the
empty picture.

They live in `ui.rs` although Load Game is their only user today: the font is part of the
palette, and every row after this one draws from it. That exception is written where they
are defined.

`button.rs` gains a round button holding one icon, 44 px across, of any `ButtonKind`.

### Save Game and Load Game (`src/overlays/saves.rs`)

- The name field becomes a capsule as tall as a button.
- A row is its picture (112 × 63, a 10 px corner) beside the name over the details.
- Load Game's Delete is the round trash button, `Danger`. `save-delete` leaves the three
  `main.ftl`.
- `SAVE_LIST_MAX_HEIGHT_PX` grows so about four of the taller rows show.
- What the lists hold does not change, and the code wins where the mockup differs: a save
  that cannot be loaded keeps its Delete, now the trash, since it is its only way out of the
  list; Save Game goes on leaving the autosaves out; the name field gets no placeholder.

### A save's picture (`src/save.rs`, `src/overlays.rs`, `src/screens/gameplay.rs`, `src/disk.rs`)

- **The file.** `<save>.jpg` beside `<save>.ron`: 320 × 180, cropped to 16:9 around the
  window's centre and stored larger than it is shown, so a scaled UI stays sharp. Written
  through a temporary file and a rename like the save itself; `write_atomic` learns to write
  bytes and to name its temporary file after the file it writes.
- **Saving under the pause.** `apply_esc_action` spawns the screenshot when it asks for
  `Pause::On`, and an observer keeps the captured `Image` in a resource: the last picture of
  the game with nothing over it. `write_new_save` and `overwrite_save` write it beside the
  save. With no picture kept, an overwritten save loses its old one, which would show another
  moment.
- **Never a stale picture.** The kept picture is cleared when a pause is asked and when
  gameplay is left, so a save never takes the picture of an earlier pause or of another game.
  A capture can be lost (Bevy drops a second screenshot of the same window in one frame, and a
  very fast Esc then Save beats its arrival): that save has no picture, which beats a wrong
  one.
- **The first capture.** If the first screenshot of a run blinks the window, a throwaway one
  is taken under the loading screen, where nothing is seen, as `init_kawase_pipelines` warms
  the glass's pipelines. Checked in the build before it is added.
- **Autosaving.** `autosave_game` writes the save at once, as today, and spawns a screenshot
  whose observer writes the picture beside it when it arrives.
- **Order.** The save is written first and marks the session saved; the picture is a second
  write, and its failure is only logged. A save with no picture is a save.
- **Deleting** removes the picture too; a missing one is not an error.
- **Showing.** Opening a list reads each save's picture from disk into `Assets<Image>`; the
  handles go with the overlay. The files are small, but every autosave is kept, so a player
  with hundreds of saves pays for hundreds of reads on the main thread: that ceiling gets a
  `ponytail:` comment naming the way up (read them in a task, or as the list scrolls).
- **The cost of saving.** Cropping, shrinking and encoding 320 × 180 pixels happens on the
  main thread, in the observer. If the build feels it during an autosave on `3d/player`, it
  moves to a task; it is measured before it is moved.

### Files

| File | What changes |
| --- | --- |
| `src/ui.rs` | the palette, `INK`, the muted text widget, the icon font, its constants and `icon()` |
| `src/ui/button.rs` | `ButtonKind::Choice`, fill and ink, the label painted, the round icon button; border and glow go |
| `src/ui/panel.rs` | the card's corner and rim, the dialog's answers centred |
| `src/ui/blur.rs`, `assets/shaders/glass.wesl` | `GlassMaterial`'s shade |
| `src/screens/menu.rs` | the words and their bar |
| `src/overlays/settings.rs` | switch, segmented control, slider, label width |
| `src/overlays.rs` | `OVERLAY_WIDTH_PX`; the screenshot when the pause is asked |
| `src/overlays/saves.rs` | field, rows with pictures, the trash |
| `src/save.rs`, `src/disk.rs`, `src/screens/gameplay.rs` | the picture's file: path, write, delete, the autosave's |
| `assets/fonts/` | Lucide and its licence |
| `assets/locales/*/main.ftl` | `save-delete` goes |

## Tests

Pure functions, each with its tests beside it:

- `button_colors`: each kind at rest, hovered and pressed; the ink is dark exactly where the
  fill is white.
- The menu word's paint: invisible bar and greyish word at rest, full bar and `INK` hovered.
- The picture's path from a save's path, and that `list_saves` does not list a picture.
- The 16:9 crop of a window wider than 16:9, taller, and exactly 16:9.
- On disk (`fresh_test_dir`): bytes written atomically read back whole; deleting a save removes
  its picture; deleting a save that has none succeeds.
- The translation test already fails while one `main.ftl` keeps `save-delete`.

What no test sees, and `just run` must: every colour over the sky and over the menu's art,
the rim, the bar's trail, the pictures.

## Implementation order

One commit per step, each tried on `3d/player`, then `just tools/sync main "<message>"`. Each
step updates the docs it touches (`README.md`, `docs/flow.md`, `docs/template.md`).

1. `feat: repaint the palette for glass` — `ui.rs` `Config`, the muted text widget and its
   users. `docs/template.md` stops calling `ACCENT` amber.
2. `feat: round the cards and light their rim` — corner, `BorderGradient` and the material's
   place in the stack, the shade in `GlassMaterial` and `glass.wesl`. Looked at over the sky
   and under Reduce Transparency.
3. `feat: paint the buttons as capsules` — `INK`, `ButtonKind::Choice`, fill and ink, the label
   painted, border and glow gone, the dialog's answers centred. The pause menu, Credits and
   the dialogs are finished by this step.
4. `feat: draw the menu as words on a bar` — the menu's own widget and paint.
5. `feat: turn the settings into switches and segments` — switch, segmented control, slider,
   label and overlay widths, checked in the three languages.
6. `feat: add the icon font and the trash button` — Lucide, `icon()`, the round button, the
   name field as a capsule, `save-delete` gone.
7. `feat: keep a picture with every save` — the screenshot, the file, the autosave's, the
   delete, the first capture checked for a blink. Nothing shows it yet; the files are checked
   in the data folder.
8. `feat: show each save's picture in the lists` — rows with pictures, the empty frame and its
   icon, the list's height.
9. `docs: mark the look spec built` — this spec's status and "What changed while building
   it", the roadmap's row 0, `README.md`'s look sentence.

## What changed while building it

Nothing yet.
