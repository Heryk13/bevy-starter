# Look: every screen the template has, in Liquid Glass

Status: **built** — written 2026-11-17, approved and built on `main` on 2026-11-18, in the
eight steps of `docs/roadmap/look-tasks.md` and the review that followed them. The keycaps were left for the first screen that
shows a key (see "Out of scope"), and the icons come from a crate, not a font file (see "What
changed while building it").
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

Built on `main` in steps 1 to 8 of `docs/roadmap/look-tasks.md`, on 2026-11-18, one commit a
step (`f032a72` to `bb91d9e`), each written and run on `3d/player` first. Claude wrote the
steps and the user read and ran each one. The eight commits were then read again by a
reviewer that had not written them, and what it found was mended in a ninth (`e3bfcd2`): its
entries close this list. The task numbers below are that file's.

- **No start value was changed by a `just run`.** The palette, the card's rim and shade, the
  buttons' fills and the widths all stayed as first written: the mockup's numbers, and for
  the two widths Claude could not measure (`OVERLAY_WIDTH_PX`, `LABEL_WIDTH_PX`), its estimate.
- **The pause menu's status line is not muted text** (found while building 1.4). The spec
  counts it among the users of `TEXT_MUTED`, but that colour is only what the line holds while
  it is empty: `show_save_status` paints it in `ACCENT` or `DANGER` as it writes it, and
  `fade_save_status` fades it by its alpha. A shadow there would stay behind as the words
  fade, a ghost of them, so the line keeps its `TextColor` alone and `pause.rs` is untouched.
- **The reason a save row's details carry no shadow is written in 3.4, not in 1.4** (decided
  while building 1.4). The reason is that they turn dark under the pointer, which is only
  true once `paint_buttons` paints a button's texts: a comment saying so two steps early
  would describe code that is not there.
- **Pressing darkens the fill a button has, in place of swapping it for a dark one** (found
  while building 3.2). Task 3.2 kept the mix toward `BUTTON_FILL_PRESSED`, a black at 20%:
  on a button that is white under the pointer, that would put the dark ink on a dark fill for
  as long as the press lasts. `button_colors` now mixes the fill toward black at the fill's
  own alpha, by `BUTTON_PRESS_SHADE`, the mockup's `brightness(.82)`.
- **A button resting where it was last painted is skipped** (decided while building 3.4).
  Painting now walks down to every text a button holds, so `paint_buttons` leaves alone a
  button whose `ButtonMotion` already sits on its targets, and paints a new one once.
- **The ink is written, not propagated** (checked while building 3.4). Bevy's own
  `Propagate<TextColor>` (`bevy::app`, what `bevy_feathers` paints its labels with) replaces
  the whole colour of every text below, the alpha that keeps a row's details muted included,
  and goes through commands on each change, where the ink changes every frame of an ease.
- **The menu's systems are named after its words** (decided while building 4.4). Once the menu
  holds no `menu_button`, which is now the pause menu's alone, `activate_menu_button` and
  `update_continue_button` would name a widget that file no longer has: they are
  `activate_menu_word` and `update_continue_word`.
- **A pressed word is painted as a hovered one** (decided while building 4.2). The mockup dims
  the bar a little while a word is pressed; the spec's pure function takes the hover alone,
  and a click leaves the menu or opens a card over it within a frame or two.
- **The bar is built anew while it eases, not changed in place** (checked while building 4.5).
  A `BackgroundGradient` is a list of gradients, each a list of stops, so changing it in place
  means reaching for the first two stops by their position; `bar_gradient(color)` builds the
  three stops again instead, the same function the word spawns with. A word resting is
  skipped, as a button is, so it runs for the 0.12 s of an ease.
- **A control already showing its state is left alone** (decided while building 5.1). The
  checkboxes and the radios were checked and painted again every frame, a command each. The
  switches and the segments spawn off, and their systems act only when `Has<Checked>`
  disagrees with the window, the setting or the language, which is the frame one changes.
- **The switch is announced as a checkbox** (checked while building 5.1). Bevy's `Checkbox`
  asks a toggle switch to override its `AccessibilityNode` with `Role::Switch`, and its own
  toggle switch does; but Bevy no longer re-exports `accesskit`, so the role would cost the
  template a crate of its own, pinned to Bevy's version. A screen reader says "checkbox,
  checked", which is true of it.
- **The knob snaps** (decided while building 5.1). The spec gives the switch two states and
  no ease, and a click on it is felt at once either way.
- **The slider's track and a switch that is off share `CONTROL_TRACK`**, a colour of
  `settings.rs` (decided while building 5.1). The spec's white at 16% is `BORDER`'s value,
  but `BORDER` is the hairline, and a control's empty track is not one.
- **`OVERLAY_WIDTH_PX` is 580, not measured** (5.5). Claude cannot see the game: the width is
  worked out from the label column, the three names' lengths and the paddings, with some
  room to spare, and the user's `just run` in the three languages is what confirms it.
  Confirmed in 5.7.
- **The icons come from the `lucide-icons` crate, not from a font file in `assets/fonts/`**
  (asked by the user before 6.1: "is there no library for it?"). The crate (1.74.0, Lucide's
  own version number) holds the font's bytes, `LUCIDE_FONT_BYTES`, and every icon as a
  variant of an `Icon` enum that turns into its character, so no file is fetched, no licence
  is copied in, and no code point is read out of `lucide.css` by hand, where a wrong digit
  draws another icon. It depends on nothing, Bevy included, so it holds no upgrade back; it
  has no build script; its licence, `MIT AND ISC`, is two the `about.toml` already accepts,
  and `just release` writes it into the notices with the rest. What it costs: it is a third
  party's, not Lucide's own, and it builds all its icons into the game, about 1 MB, as the
  file would have. An icon is therefore `icon(Icon::Trash2)`, not a constant in `ui.rs`.
- **The name field is at least a button tall, not exactly** (decided while building 6.6). An
  `EditableText` is its own text, so nothing centers it in a taller box: its padding does,
  and `min_height: BUTTON_HEIGHT_PX` only holds the field to the New Save beside it.
- **The picture has a file of its own, `src/save/picture.rs`** (decided while building 7.3).
  The spec put it in `save.rs`, which was already 580 lines, past where `CONTRIBUTING.md`
  splits a file, and the picture reads on its own: taking it, encoding it, its file. `save.rs`
  keeps `delete_save_files`, which is about the save, and re-exports what the other files call.
- **`save.rs` has a plugin, added in `main.rs`** (decided while building 7.9). The kept
  picture is a resource and is emptied on leaving gameplay, so the picture needs a plugin,
  and `save.rs`, which had none, passes it on.
- **A picture is never encoded on the main thread, so its cost was not measured** (decided
  while building 7.6; task 7.14 is not done as written). The spec encoded in the observer and
  moved to a task only if a `just run` felt it. Cropping and shrinking a 4K window is felt,
  and under the pause menu it would land in the frames the card rises in; the task is a few
  lines, so both observers hand the capture to the `AsyncComputeTaskPool` from the start.
- **The kept picture is the encoded bytes behind a lock, one `PausePicture` per pause**
  (decided while building 7.9). The task fills it when it is done, which is why it sits
  behind an `Arc<Mutex<..>>`. Asking for a pause puts a new, empty `PausePicture` in place
  rather than clearing the old one: a capture from an earlier pause that arrives late fills
  the one it was taken for, which nobody reads any more, so it can never be taken for this
  pause's. Keeping the bytes, not the captured image, is some tens of KB in place of the
  whole window's pixels.
- **`write_atomic` takes anything that reads as bytes** (7.1), so the text callers did not
  change, except two tests in `save.rs` that passed `&format!(..)`, which clippy now calls a
  needless borrow.
- **The crop is a `URect`, and the encoder's test reads the file back** (7.4, 7.6): a named
  rectangle in place of a tuple of four numbers, and a test that a window of another shape
  comes out as a JPEG of the picture's size, which is what protects the format.
- **The empty frame is an opaque grey, and it also stands for a picture that cannot be
  read** (asked by the user before 8.3). The spec gave a save with no picture "an empty
  frame"; the user asked for a grey fill with the missing-picture icon, for a picture that is
  not there as for one that fails to load. `read_save_picture` gives `None` for both, and
  `SAVE_PICTURE_EMPTY_FILL` is opaque, as a picture is, so it reads on a row at rest and on
  the white one under the pointer.
- **The frame always holds an image and an icon, and one of the two shows** (decided while
  building 8.3). A function returns one bundle type, so a row with a picture and a row
  without cannot be two different bundles: `ImageNode::default()` draws nothing, and the
  icon is hidden under a picture.
- **A row is padded as closely on its left as above and below** (decided while building 8.3,
  from the mockup). `row_button` padded its sides as a button does, which would leave the
  picture 20 px in and 8 px down; the row's corner (18) is now its padding (8) plus the
  picture's corner (10), so the two curves run together.
- **The pictures are read as the list spawns, not through the asset server** (checked while
  building 8.1). Bevy can be given a second asset source, which would load them off the main
  thread; it has to be registered before the default plugins, logs an error for every save
  with no picture, and shows nothing while a picture loads, so the missing-picture icon would
  need a system watching each load. Reading the file is a few lines and says at once whether
  there is a picture; its ceiling is the `ponytail:` comment the spec asked for.
- **A text can keep its own colour under a button: `OwnColor`** (found by the review after
  step 8). The ink reaches every text a button holds, which is right for a text lying on the
  button's fill and wrong for the icon of a save's empty frame: it lies on the frame's grey,
  where the dark ink of a row under the pointer was lost. `paint_buttons` leaves a text
  marked `OwnColor` alone.
- **The first screenshot is warmed up after all** (found by the review, task 7.13). Nothing
  was seen to blink in the user's runs, but the reason it can is in Bevy's source: the frame
  of a run's first screenshot is not drawn to the window until its pipeline is built, and an
  autosave takes that screenshot in the middle of play. `warm_up_screenshots` takes one
  nobody keeps as the first loading screen opens, as the spec allowed.
- **A capture is taken out of its event, not copied** (found by the review). The observers
  cloned the captured image, every pixel of the window, on the main thread, which the
  departure above about never encoding there had left behind. `encode_capture` takes it with
  `On::event_mut()`, and logs a capture it cannot encode for the pause's picture as for the
  autosave's.
- **A row's corner is derived in `button.rs`** (found by the review). The 18 px of
  `ROW_BUTTON_RADIUS_PX` were the padding plus the picture's corner, written as a third
  number: `ROW_CONTENT_RADIUS_PX` now lives beside the padding, the row's corner is their sum,
  and the picture's frame reads the same constant.
- **The trash says "Delete" to a screen reader in every language** (found by the review,
  left as it is by the user: accessibility is not being worked on for now). The key
  `save-delete` left with the word, and the `AccessibleLabel` is English, as those of the
  Settings controls already were.
