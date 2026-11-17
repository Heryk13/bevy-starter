# Look: tasks

Roadmap row 0, second of its three specs, built on `main`. The decisions and their reasons live
in the spec, `docs/specs/look/design.md`; this file only breaks its implementation order into
tasks small enough for one sitting each, with the facts each task needs. The screens as
approved are in `docs/roadmap/ui/look-screens.html`.

## How to read this

- **Step** = one commit. Its title is the commit message.
- **Who**: *you* writes the Rust and the shaders; *Claude* does pure moves and renames, docs,
  doc comments and separators, and hands over the lines of an API you have never seen, saying
  why; *both* is a review together (the checklist in `CLAUDE.md`: every name, the order of
  sections and systems, `just check`). A step may instead be written by Claude in small pieces
  for you to read, as the glass's last steps were: that is settled in one line at the start of
  each step, and the column stays as the default.
- **Facts**: the names you cannot guess (types, methods, modules), checked in the Bevy 0.20.0
  source and in this repository on 2026-11-18. Assembling them is the task.
- **Where**: every step is written and tried uncommitted on `3d/player`, where the sky is the
  hardest thing a card sits over, then `just tools/sync main "<message>"` commits it on `main`
  and merges it down the tree.
- A task is done when `just check` passes, except where it says the next task makes it pass.
- The numbers in the spec's palette and in the tasks below are start values from the mockup.
  The ones a `just run` changes are recorded in the departures.
- Mark a task done by changing its `[ ]` to `[x]`.

## Departures from the spec, to record in its "What changed while building it"

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

## The buttons at a glance

What `button_colors` returns from step 3 on, the fill first and the ink second:

| Kind | Who uses it | At rest | Under the pointer |
|------|-------------|---------|-------------------|
| `Primary` | `button("save-new", ..)`, a Yes that destroys nothing | `ACCENT`, `INK` | `ACCENT` a little dimmer, `INK` |
| `Choice` | `menu_button` (the pause menu), `row_button` (a save's row) | white at 10%, `TEXT` | `ACCENT`, `INK` |
| `Quiet` | `overlay-close`, `dialog-no` | clear, `TEXT` | white at 18%, `TEXT` |
| `Danger` | the trash, a Yes that destroys | white at 10%, `DANGER` | `DANGER`, white |

## Step 1: `feat: repaint the palette for glass`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 1.1 The palette in `src/ui.rs`'s `Config`, by the spec's table: `ACCENT`, `DANGER`, `TEXT`, `TEXT_MUTED`, `BORDER`, `FIELD_FILL`, `SURFACE` | you | `Color::srgba(1.0, 1.0, 1.0, 0.92)` for a white at 92%; `SURFACE` is `Color::srgba_u8(0x2C, 0x2C, 0x32, 143)`, 143 being 56% of 255. `INK` waits for step 3: nothing reads it yet, and dead code fails `just check`. The tests of `card_tint` and `lay_over` in `blur.rs` are written over `SURFACE`, so they follow |
| [x] | 1.2 The muted text's shadow: its colour and its offset, as constants in the same `Config` | you | `TextShadow { offset: Vec2, color: Color }`, in `bevy::prelude`; the offset is in logical pixels and the shadow has no blur, so it stays small (1 px down) and soft (black at about 50%) |
| [x] | 1.3 One widget in `ui.rs`'s `Widgets` for what makes a text muted: its font at a size, `TEXT_MUTED` and the shadow. The caller adds the `Text` or the `Localized` | you | shaped like `label()`, which returns `impl Bundle`; `font(size_px)` is already there. Suggested name: `muted_text(size_px)` |
| [x] | 1.4 Its users take it: the menu's version (`menu.rs:163`), the pause menu's status line (`pause.rs:132`), Credits (`credits.rs:32`) and `muted_label` in `saves.rs` | you | `save_row`'s details keep `TextColor(TEXT_MUTED)` alone: from step 3 they turn dark under the pointer, where a shadow would smear. Claude writes that reason beside them |
| [x] | 1.5 `docs/template.md` line 151 stops calling `ACCENT` amber; the palette's comments in `ui.rs` | Claude | `README.md` line 22 ("an amber glow") waits for step 3, when the glow goes |
| [x] | 1.6 `just run` on `3d/player`: every card over the sky, the menu's version over its artwork, each again under Reduce Transparency; tune `SURFACE`'s alpha and the shadow by eye; review; sync | both | amber is gone from this step on, but the buttons keep their border and glow until step 3: a hovered button glows white for two commits |

## Step 2: `feat: round the cards and light their rim`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 2.1 `CARD_RADIUS_PX` 16 → 26 in `src/ui/panel.rs` | you | `glass.wesl` cuts the corners from `in.border_radius_x.x`, so the glass follows by itself |
| [x] | 2.2 `GlassMaterial` is drawn under its node's border, over its shadow | Claude hands the lines | `UiMaterial::stack_z_offset() -> f32` is a provided method whose default is `stack_z_offsets::MATERIAL` (0.05), over `BORDER` (0.01) and `BORDER_GRADIENT` (0.03): that is why today's card border never shows. `bevy::ui_render::stack_z_offsets::BACKGROUND_COLOR` (0.0) sits under both and over `BOX_SHADOW` (-0.1). A backdrop has no border, so it does not care |
| [x] | 2.3 The rim: the card's `BorderColor::all(BORDER)` becomes a `BorderGradient` from the top left to the bottom right: bright white, almost nothing, a dark edge | you | `BorderGradient::from(LinearGradient::to_bottom_right(vec![ColorStop::new(color, percent(0)), ..]))`, the shape `menu.rs` gives its scrim; the three colours and their stops are constants in `panel.rs`'s cards group. After 2.2 the border shows for the first time: look at 2.3 right after it |
| [x] | 2.4 The shade: `GlassMaterial` gains how far its tint darkens from the top of the node to its bottom, 0.0 for a backdrop, a constant for a card | you | a fourth `#[uniform(0)]` field, an `f32` after `flat`, set in `GlassMaterial::backdrop` and `GlassMaterial::card`; the constant lives in `blur.rs`'s `Config` |
| [x] | 2.5 `assets/shaders/glass.wesl`: the same field in its `GlassMaterial` struct, in the same order, and one `mix` darkening `material.tint.rgb` by `in.uv.y` before the tint is used | you | `in.uv.y` runs 0.0 at the node's top to 1.0 at its bottom. It must apply to the flat path too: darken the tint first, then let `glass` and `flat` read the darkened one |
| [x] | 2.6 `docs/flow.md`, where it describes the card and the glass material | Claude | |
| [x] | 2.7 `just run` on `3d/player`: the rim and the shade over the sky, over the menu, in a dialog over the pause menu; under Reduce Transparency the card is solid and keeps both; resize with a card open; review; sync | both | a shader only compiles at run time: `just check` says nothing about 2.5 |

## Step 3: `feat: paint the buttons as capsules`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 3.1 `ButtonKind::Choice`, and `menu_button` and `row_button` move to it; `INK` in `ui.rs`'s palette | Claude (a move), you for `INK` | `INK` is `Color::srgb_u8(0x11, 0x11, 0x14)`. `Primary`'s only user left is `button("save-new", ..)` |
| [x] | 3.2 `ButtonColors` is a fill and an ink, and `button_colors` returns them by the table above. The border and the glow go: `BUTTON_GLOW_*`, the `border` in `button_frame`, `BorderColor` and `BoxShadow` in `button_base` | you | pressing still mixes toward `BUTTON_FILL_PRESSED`, whatever the fill; `CLEAR` stays the Quiet's rest, for the reason its comment gives; the fills are constants of `button.rs`'s colors group |
| [x] | 3.3 The tests of `button_colors`, replacing the six there: each kind at rest and hovered, a pressed one darker | you | suggested: `primary_button_is_white_already_at_rest`, `choice_button_hovered_turns_white_with_dark_ink`, `quiet_button_at_rest_draws_nothing`, `danger_button_hovered_fills_red_with_white_ink`, `pressed_button_darkens_its_fill`, `ink_is_dark_exactly_where_the_fill_is_white` (all four kinds, rest and hovered). `same_color` is already in the tests module |
| [x] | 3.4 `paint_buttons` writes the ink to every text under the button, keeping each text's own alpha | you | the query loses `BorderColor` and `BoxShadow` and gains `Entity`; a second `Query<&Children>` has `iter_descendants(entity)`; a third `Query<&mut TextColor>`; `ink.with_alpha(text_color.0.alpha())`. Without the alpha kept, a row's muted details would turn as dark as its name |
| [x] | 3.5 `row_button` takes a fixed corner, 18 px, in place of the pill's | you | `border_radius: BorderRadius::all(px(..))` after `..button_frame()` cannot be written: a struct's fields come before its `..rest`. Put it with the row's other fields |
| [x] | 3.6 `dialog()` in `panel.rs`: `JustifyContent::End` → `Center` | you | |
| [x] | 3.7 `README.md` line 22 (no amber glow), `docs/flow.md` lines 527 and 541 (`ButtonKind`'s kinds), `ButtonKind`'s own doc comments | Claude | |
| [x] | 3.8 `just run`: the pause menu, Credits, every dialog and the two save lists as they are today; a pointer down the pause menu leaves a trail; a pressed button; a row's details stay muted on white; review; sync | both | the pause menu, Credits and the dialogs are finished by this step |

## Step 4: `feat: draw the menu as words on a bar`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 4.1 `Config` in `src/screens/menu.rs`: the column 300 px wide, the word's padding (20), the title's left edge (80) and the column's own left derived from the two; the word's rest alpha (0.62); where the bar stops being solid (30%) | you | the column's left is derived, never restated (`CONTRIBUTING.md`, Bevy patterns). The title sits in the same padded parent as the column, so it needs the word's padding as its own left margin to stay at 80 |
| [x] | 4.2 A pure function: from how far a word is hovered (0.0 to 1.0), the bar's strength, the word's colour and its shadow's alpha | you | the colour goes from white at the rest alpha to `INK`: `Color::mix(&other, factor)`, as `button_colors` uses it. The shadow fades out as the word darkens |
| [x] | 4.3 Its tests | you | suggested: `word_at_rest_is_greyish_over_no_bar`, `hovered_word_is_dark_over_a_full_bar`, `word_loses_its_shadow_as_it_darkens` |
| [x] | 4.4 The menu's own widget, in `menu.rs`, replacing `menu_button` there: a `Button` with `Hovered`, a component holding how far it is hovered, a `BackgroundGradient`, and its word with a `TextShadow` | you | `bevy::ui_widgets::Button` and `bevy::picking::hover::Hovered`, as `button.rs` imports them; no `ButtonKind`, so `paint_buttons` leaves it alone, while `play_button_sounds` and `Activate` still reach it. `menu_button` stays in `ui.rs` for the pause menu |
| [x] | 4.5 The system that eases and paints the words, in `Update` under `in_state(Screen::Menu)` | you | `approach` from `crate::fade` and `Time<Real>`, as `paint_buttons`; `BUTTON_EASE_SECS` becomes `pub(crate)` and is re-exported by `ui.rs`, its second user. The bar is `LinearGradient::to_right` with three stops: white at the strength at 0% and at the solid stop, clear white at 100% (clear *white*, not `Color::NONE`, for the reason `CLEAR` gives in `button.rs`). The word's `TextColor` and `TextShadow` are on the child |
| [x] | 4.6 The column loses its `row_gap`, so the pointer never falls between two words | you | `BUTTON_GAP_PX` leaves `menu.rs`'s imports |
| [x] | 4.7 `docs/flow.md`'s menu section | Claude | |
| [x] | 4.8 `just run`: the words line up under the title; the bar's trail down the column, never a blink; Continue hidden with no save leaves no hole; every word reads over the artwork; the opening veil; review; sync | both | `update_continue_button` finds the button by `MenuAction` and sets `Display::None`, so it needs no change |

## Step 5: `feat: turn the settings into switches and segments`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 5.1 The switch, replacing `checkbox()` and `set_checkbox` in `src/overlays/settings.rs`: a 46 × 26 capsule and a 20 px knob | you | still a `Checkbox` (`bevy::ui_widgets`) with `Checked`, so `ValueChange<bool>` and the two toggle observers stay as they are. The knob is a child with a marker, `PositionType::Absolute`; its inset and its two `left` values are derived from the three sizes. Off: track white at 16%, knob white. On: track `ACCENT`, knob `INK` |
| [x] | 5.2 The renames the switch brings: `update_fullscreen_checkbox`, `update_reduce_transparency_checkbox`, the checkboxes' `Config` group, the markers' doc comments | Claude | |
| [x] | 5.3 The segmented control, replacing `language_radio` and `RadioMark`: one capsule track, one segment per `Language::ALL` | you | `RadioGroup` and `RadioButton` stay, so `ValueChange<Entity>` and `choose_language` do not change; `update_language_radios` paints the segment's `BackgroundColor` and its text's `TextColor` in place of the ring: chosen `ACCENT` with `INK`, the others clear with `TEXT_MUTED`; `SMALL_FONT_SIZE_PX`. `RADIO_*` go |
| [x] | 5.4 The slider: a 6 px track, an 18 px thumb, the thumb white | you | `SLIDER_TRACK_HEIGHT_PX`, `SLIDER_THUMB_SIZE_PX`; `update_volume_slider` reads both, so it follows |
| [x] | 5.5 `LABEL_WIDTH_PX` 140 → 190; then `OVERLAY_WIDTH_PX` in `src/overlays.rs` widened to what the three language names need beside it | you | measured in the game in English, pt-BR and Japanese, not guessed: about 560. Save Game and Load Game share the constant, and its comment names what it must hold |
| [x] | 5.6 The `ponytail:` comment where the segmented control is built (three or four names; more needs a list); `docs/flow.md`'s Settings section | Claude | |
| [x] | 5.7 `just run` in the three languages: no label or name wraps; both switches save and come back after a restart; the arrow keys still move the slider; Reduce Transparency flips this panel's own glass; review; sync | both | |

## Step 6: `feat: add the icon font and the trash button`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 6.1 `assets/fonts/lucide.ttf` and its licence, the version written beside them | Claude | from the `lucide-static` package of a release at or after 1.70.0 (`font/lucide.ttf`, `font/lucide.css`, `LICENSE`), checked when fetched; ISC, so `README.md`'s credits and the Credits overlay gain a line if Noto Sans JP has one there |
| [x] | 6.2 The icon font built into the binary under a handle known at compile time, installed at `Startup` | Claude hands the lines | `uuid_handle!("..")` (`bevy::asset`) makes a `Handle<Font>` constant; `include_bytes!` and `Font::from_bytes(..)` then `fonts.insert(handle.id(), font)`, as `use_ui_font` does in `i18n.rs` with `AssetId::default()`. `ui.rs` gets its first system, so a `Systems` section |
| [x] | 6.3 `icon()` in `ui.rs`'s `Widgets`, and the trash as a named constant in its `Config` | you | a `Text` holding one code point: `TextFont { font: ICON_FONT.into(), font_size: .., ..default() }` (`TextFont::font` is a `FontSource`, which a handle converts into); 22 px beside 18 px text; the code point of `trash-2` is read in that release's `lucide.css` and written `"\u{....}"`. Claude writes beside them why they live in `ui.rs` with one user |
| [x] | 6.4 A round button holding one icon, 44 px across, of any `ButtonKind`, in `button.rs` | you | `button_base(node, kind, children![icon(..)])`; `BUTTON_HEIGHT_PX` is already 44; `paint_buttons` paints the icon since 3.4, an icon being a text |
| [x] | 6.5 Load Game's Delete becomes that button, `Danger`; `save-delete` leaves the three `main.ftl` | you | `saves.rs:327`; an icon says nothing to a screen reader, so the button takes `AccessibleLabel::new("Delete")`, as the Settings controls do. The translation test fails while one file keeps the key |
| [x] | 6.6 The name field becomes a capsule as tall as a button | you | `BUTTON_HEIGHT_PX` becomes `pub(crate)`, re-exported by `ui.rs`; `BorderRadius::MAX`; `SAVE_NAME_RADIUS_PX` goes, and the padding becomes horizontal only |
| [x] | 6.7 `docs/template.md` (line 55, the fonts), `docs/flow.md` (line 622's table, the shared-files row of `ui.rs`), `README.md` | Claude | |
| [x] | 6.8 `just run`: the trash is drawn, not an empty box; red ink, red fill with a white trash under the pointer; it still asks before deleting; the field's cursor and its outline when focused; `just release`'s licence check is not touched by a font; review; sync | both | |

## Step 7: `feat: keep a picture with every save`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 7.1 `write_atomic` in `src/disk.rs` writes bytes, and names its temporary file after the file it writes | you | `std::fs::write` takes `impl AsRef<[u8]>`, which a `&str` already is, so no caller changes; `Path::with_added_extension("tmp")` turns `save_1.jpg` into `save_1.jpg.tmp`, and `settings.ron` into the `settings.ron.tmp` of today |
| [x] | 7.2 Its test: `written_bytes_read_back_whole` | you | `fresh_test_dir`, a name of its own; `std::fs::read` |
| [x] | 7.3 In `src/save.rs`: a save's picture path from its path, and the tests `picture_sits_beside_its_save` and `a_picture_is_not_listed_as_a_save` | you | `Path::with_extension("jpg")`; `list_saves` already keeps only `.ron` |
| [x] | 7.4 The picture's size as constants (320 × 180) and a pure function for the 16:9 crop around the centre of a window of any size | you | plain `u32`s in, `(x, y, width, height)` out; the ratio is derived from the two constants |
| [x] | 7.5 Its tests, written over the constants | you | `wide_window_loses_its_sides`, `tall_window_loses_its_top_and_bottom`, `window_already_16_by_9_is_kept_whole` |
| [x] | 7.6 An `Image` turned into the picture's bytes | Claude hands the lines | the `image` crate's API, reached without naming the crate: `Image::try_into_dynamic()` gives its `DynamicImage`, then `crop_imm(x, y, w, h)`, `thumbnail_exact(w, h)`, `to_rgb8()` and `write_to(&mut std::io::Cursor::new(Vec::new()), format)`, the format from `bevy::image::ImageFormat::Jpeg.as_image_crate_format()`, an `Option` |
| [x] | 7.7 Writing a picture beside a save, and deleting a save with its picture; `delete_save` in `saves.rs` calls the second | you | the picture goes through `write_atomic`; a picture that is missing when deleting is not an error: `std::io::ErrorKind::NotFound` |
| [x] | 7.8 Their tests on disk: `deleting_a_save_removes_its_picture`, `deleting_a_save_without_a_picture_succeeds` | you | `fresh_test_dir`, one name each |
| [x] | 7.9 The kept picture: a resource holding the last capture of the game with nothing over it, in `save.rs`, since `overlays.rs` fills it and `saves.rs` reads it | you | an `Option<Image>` inside; cleared when a pause is asked and on `OnExit(Screen::Gameplay)` |
| [x] | 7.10 `apply_esc_action` in `src/overlays.rs` takes the screenshot when it asks for `Pause::On` | Claude hands the lines | `bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured}`; `commands.spawn(Screenshot::primary_window()).observe(..)`, the observer an `On<ScreenshotCaptured>` whose `image` is the capture. Spawned beside the `set`, it captures a frame with no card: `OnEnter(Pause::On)` runs a frame later |
| [x] | 7.11 `write_new_save` and `overwrite_save` write the kept picture after the save; with none kept, an overwritten save loses its old one | you | the save is written first and marks the session saved; the picture's failure is only logged, with `error!` |
| [x] | 7.12 `autosave_game` in `src/screens/gameplay.rs` spawns a screenshot whose observer writes the picture beside the autosave when it arrives | you | the path moves into the observer's closure: `.observe(move \|captured: On<ScreenshotCaptured>\| ..)` |
| [x] | 7.13 The first capture of a run, looked at: if the window blinks, a throwaway screenshot is taken under the loading screen | both | Bevy queues the screenshot's pipeline the frame it is first taken. Nothing is added unless the blink is seen |
| [x] | 7.14 The cost of an autosave's picture, measured on `3d/player` with temporary lines never staged | both | `AUTOSAVE_INTERVAL_SECS` shortened for the run and `std::time::Instant` around 7.6's function; it moves to a task only if the frame feels it |
| [x] | 7.15 `docs/flow.md` (saving, the pause, the autosave), `docs/template.md` (what the saves folder holds) | Claude | |
| [x] | 7.16 `just run`: New Save, an overwrite and an autosave each leave a `.jpg` beside their `.ron` in the data folder (`%APPDATA%\<game>\saves`); no picture shows the pause card or the blur; Esc then Save at once; deleting takes both files; review; sync | both | nothing in the game shows a picture yet: the files are opened by hand |

## Step 8: `feat: show each save's picture in the lists`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 8.1 Reading a save's picture from disk into an `Image`, in `save.rs` | Claude hands the lines | `std::fs::read`, then `Image::from_buffer(&bytes, ImageType::Extension("jpg"), CompressedImageFormats::NONE, true, ImageSampler::Default, RenderAssetUsages::RENDER_WORLD)`: `bevy::image` for the first three types, `bevy::asset::RenderAssetUsages` for the last. The asset server reads only `assets/` |
| [x] | 8.2 The empty picture's icon, a named constant beside the trash | you | `image-off` in `lucide.css` |
| [x] | 8.3 `save_row` takes the picture: a 112 × 63 frame with a 10 px corner beside the name over the details; with no picture, the frame holds the icon | you | `ImageNode::new(handle)`; the frame's height is derived from its width and the picture's ratio; the row becomes a row (`FlexDirection::Row`) holding the frame and today's column |
| [x] | 8.4 Both lists read each save's picture as they spawn | you | `ResMut<Assets<Image>>` in `spawn_save_game_overlay` and `spawn_load_game_overlay`; `Assets::add` returns the handle, and the `ImageNode` holding it is what keeps the picture alive until the overlay despawns. A row for a save that cannot be loaded stays the muted line it is |
| [x] | 8.5 `SAVE_LIST_MAX_HEIGHT_PX` shows about four of the taller rows | you | derived from the frame's height, the row's padding and the gap between rows, not restated |
| [x] | 8.6 The `ponytail:` comment on reading every picture on the main thread; `docs/flow.md`'s two lists | Claude | |
| [x] | 8.7 `just run`: each save beside its picture; a save from before this spec shows the empty frame; the corners are round; a row under the pointer turns white without hiding its picture; opening a list of many saves does not stall; review; sync | both | whether an `ImageNode` is clipped by its node's `border_radius` is looked at here; if not, the frame clips it with `Overflow::clip()` |

## Review after step 8: `fix: mend what the look's review found`

The eight commits read again by a reviewer that had not written them, against the rules of
`CONTRIBUTING.md` and the Bevy 0.20.0 source. It found no save lost and no picture of the
wrong moment; what it did find is below, each with what was done.

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | R.1 The icon of a save's empty frame keeps its colour when its row turns white | Claude | `OwnColor`, a marker in `button.rs`; `paint_buttons` asks for `Query<&mut TextColor, Without<OwnColor>>`. Under the pointer the icon had gone from about 8:1 against the frame's grey to about 2:1 |
| [x] | R.2 A screenshot nobody keeps, the first time a loading screen opens | Claude | `warm_up_screenshots` on `OnEnter(Screen::Loading)` with `.run_if(run_once)`; a `Screenshot` spawned with no observer is dropped as it arrives, and Bevy despawns its entity |
| [x] | R.3 The captured image is taken out of its event instead of cloned, and a failed encode is logged on both paths | Claude | `encode_capture(captured, then)`: `std::mem::take(&mut captured.event_mut().image)`; an `Image` has a `Default`. A clone was 8 MB at 1080p and 33 MB at 4K, on the main thread |
| [x] | R.4 Tests for the two branches of `save_pause_picture` | Claude | `a_save_made_under_the_pause_gets_the_kept_picture`, `a_save_with_no_kept_picture_loses_its_old_one`; a `PausePicture` already holding bytes is built by hand in the test |
| [x] | R.5 `ROW_BUTTON_RADIUS_PX` is derived | Claude | `ROW_BUTTON_PADDING_PX + ROW_CONTENT_RADIUS_PX`, in a derived group of `button.rs`; `saves.rs` loses `SAVE_PICTURE_RADIUS_PX` |
| [x] | R.6 Names and comments that no longer said what is there | Claude | `SWITCH_KNOB_ON_LEFT_PX` → `SWITCH_KNOB_ON_OFFSET_PX`; the comments of `FIELD_FILL`, `BUTTON_GAP_PX` and `write_atomic`'s `ponytail:` note; the limit of the early `continue` in `paint_buttons`; the test `a_window_of_any_shape_becomes_a_picture_of_one_size` → `a_captured_window_becomes_a_picture_of_the_pictures_size` |
| [x] | R.7 `docs/flow.md`: `OwnColor`, the derived corner, `encode_capture` and the warm-up | Claude | |
| [x] | R.8 `just run`: the icon of a save with no picture stays light under the pointer; the corners of a row and of its picture as before; a New Save still gets its picture; sync | both | not fixed, by the user's decision: the trash's `AccessibleLabel` stays English (see the departures) |

## Step 9: `docs: mark the look spec built`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 9.1 The spec's status line and its "What changed while building it", from the departures above; the roadmap's row 0; `README.md`'s look sentence | Claude | |
| [x] | 9.2 Sync; `just check` on the four branches; `just run` on `main` and `2d/base`, where no step was tried; push | both | |
