# File explorer: tasks

Roadmap row 4, built on `main`, but step 2, which is `3d/player`'s. The decisions and their
reasons live in the spec, `docs/specs/file-explorer/design.md`; this file only breaks its
implementation order into tasks small enough for one sitting each, with the facts each task
needs. The look is the mockup's block "4" in `docs/roadmap/ui/mockup.html`.

## How to read this

- **Step** = one commit. Its title is the commit message.
- **Who**: *Claude* writes each step in pieces and the user reads and runs it; *you* is a
  `just run`, which only the user's eyes can do; *both* is the review at the end of a step
  (the checklist in `CLAUDE.md`: every name, the order of sections and systems, `just check`).
- **Facts**: the names that cannot be guessed, checked in the Bevy 0.20.0 source and in this
  repository on 2026-11-19.
- **Where**: every step is written and tried uncommitted on `3d/player`. The user syncs to
  `main` when there is enough to send; step 2 goes to `3d/player` with a sync of its own.
- **Nothing unused, in either build**, as in every dev tool: what only the explorer reaches
  for outside `dev_tools/` sits behind `#[cfg(feature = "dev")]`.
- Mark a task done by changing its `[ ]` to `[x]`.

## Departures from the spec, to record in its "What changed while building it"

- **Esc closes the topmost modal, not every dialog** (decided while building 1.1): the
  `Dialog` marker became private to `panel.rs`, and the Esc rule reads `ClosesOnEsc`, which a
  dialog and the explorer both carry, closing the one with the highest `GlobalZIndex` per
  press. `EscAction::CloseDialog` became `CloseModal`.
- **The keys panel leaves out a tool's inner keys** (asked by the user while building step 1,
  a fix to row 2 riding with this step): Take Photo, Slower and Faster only work inside their
  open tool, which shows them, so `register_inner_key` in `dev_tools.rs` keeps them off the
  keys panel. Settings' Dev page still lists them.
- **The cursor follows `ClosesOnEsc`** (decided while building 2.1, changed with the drawer
  below): `first_person.rs` frees it while anything Esc closes first is open, a dialog or the
  explorer, rather than while a floating root blocks the pointer, which the drawer no longer
  does. `ClosesOnEsc` is already on `main`, so step 2 changes no shared code. `lock_cursor`
  and `unlock_cursor` became one `hold_cursor`, run every frame, writing only on a change.
- **The explorer is a drawer, not a window over everything** (asked by the user after step
  6): it is used while playing, to put things into the game, so it slides up from under the
  screen in `SLIDE_SECS` (0.25 s, eased) and back down to close, and covers the bottom 38%
  (`DRAWER_HEIGHT_PERCENT`) with a 12 px margin. Its root lets the pointer through, so the
  game above it has no backdrop and stays sharp; the drawer itself takes the pointer. The
  slide moves the root's `UiTransform`, since `animate_entrances` sets the card's own.
  Esc slides it away too: the Esc rule sends the topmost `ClosesOnEsc` a `Dismiss` event
  (`ui/panel.rs`) instead of despawning it, and each modal answers its own way, a dialog
  at once, the drawer by sliding.
- **Faint scrollbars, and a grip that resizes the drawer** (asked by the user after the
  drawer). The list and the sidebar scroll inside a frame of their own (`scroll_frame`,
  `scrolling_column`), and each `ExplorerScroll` node gets Bevy's own `Scrollbar` laid over
  the frame's right edge: 4 px, white at 22%, hidden while everything fits. A grip of three
  lines (`Icon::GripHorizontal`) sits in the middle of the drawer's top edge; dragging it
  resizes the drawer between 20% and 85% of the screen (the pure `dragged_height`), and
  `DrawerHeight` keeps the size until the game closes, so the explorer opens as it was.
- **The sidebar is a tree, as a code editor shows one** (asked by the user after the
  scrollbars): Gallery's or Assets' folders and files, folders first, each open folder's
  entries under it one level in, a thin guide line for each level, a chevron to open and
  close, the file's kind as its icon. Rows are plain, 24 px, with no capsule or corner: a
  faint fill under the pointer and a brighter one for what is shown. A press on a folder
  opens it and shows it, or closes the folder shown; a press on a file shows its folder
  with the file selected, so the preview shows it. The folder shown is always open, with
  every folder holding it, so entering one from the list opens the tree down to it.
  `OpenFolders` keeps what was opened until the game closes. The rows are not `Button`s,
  whose sounds would tick on every row the pointer crosses; they take `PointerClick` and
  `Hovered` themselves. The layout is the pure `tree_rows`, with three tests.
- **A button repaints when its kind changes** (a bug the user found: rows stayed selected
  after another was picked, as if Shift were held, and lost their white under the
  pointer). `paint_buttons` in `ui/button.rs` left a resting button alone, its kind
  included, so a row turned from Primary back to Choice kept its white until the next
  hover. It now repaints a button whose `ButtonKind` changed. The fix is in shared code, so
  it reaches `main` with this row.
- **Everything pressable shows a pointing hand** (asked by the user, first for the grip,
  then for the whole UI): `point_at_pressables` in `ui/button.rs` sets the window's
  `CursorIcon` to `SystemCursorIcon::Pointer` while the mouse is over a `Button`, `Checkbox`,
  `RadioButton`, `Slider` or `Hovered`, or a node they hold, read from Bevy's `HoverMap`, and
  back to the default once it leaves, writing only on a change. Shared code, so it reaches
  `main` with this row.
- **The title bar sits right at the drawer's top** (asked by the user, for more room below
  it): the grip no longer takes a line of its own over the title bar. The title bar is the
  close button and the places on the left, the grip over the path in the middle (a strip of
  120 by 18 px, easy to catch), and Empty Trash on the right, all lined up at the top. On
  the way there the grip lay over the whole top edge, which the user found too tight, and a
  smaller close button was tried and turned back; instead its X is larger, 28 px
  (`CLOSE_ICON_PX`), in a circle as tall as the places beside it, 40 px (`PLACE_HEIGHT_PX`). A 1 px line in `BORDER` sets the title bar
  apart from what the drawer shows.
- **The calendar moved too** (decided while building 3.1): `civil_from_days` and the
  `SECS_PER_*` constants left `photo.rs` for `dev_tools.rs` with `gallery_dir()`, since the
  list writes dates as well, rather than `photo.rs` exporting them through `photo_mode.rs`.
- **Until the sidebar, `Explorer` holds the gallery's path**, not a place: a `Place` with
  Assets and Trash would be variants nothing builds before step 6, which `just check`
  refuses.
- **The list is rebuilt only when the folder changes** (found while building 3.2): a row
  rebuilt between the two clicks of a double click is a new entity, so its second click
  would count as a first. The selection repaints the rows instead, as `ButtonKind::Primary`
  for the selected one, which `paint_buttons` already paints white with ink letters.
- **Sizes count in 1024s**, as the Windows explorer does, so the two agree on a file: the
  spec's "under 1000" became "under 1024", and a size keeps a decimal under 10 of its unit.
- **The preview decodes by the file's own extension** (decided while building 4.1), with a
  `decode_picture` of its own in `preview.rs`: the save pictures' one in `picture.rs` reads
  JPEG only, the format a save's picture is written in.
- **A name in Recents is its path inside the gallery** (decided while building 6.1),
  `trailer/shot.png`: the row says where a photo was sorted into, and the explorer finds it
  from the gallery alone. `selected_on_disk` rebuilds it part by part, so Windows' explorer
  gets a path with its own separators when it opens one.
- **`Place` came with the sidebar**, as the step 3 departure said, and with it Trash as a
  place: its folder is listed from step 6, empty until step 8 puts things in it.
- **The sidebar is rebuilt each time what is shown changes**, as the list and the path are,
  so the highlight moves and a folder made elsewhere shows up, with no repainting of its own.
- **The user's adjustments after steps 4 to 6**, each seen in the game:
  - **A few rows no longer fill the list**: a row button grows to fill what holds it, so
    every row of the list and folder of the sidebar sits in a line node of its own, which
    grows only across.
  - **The path is words alone, centred**: no capsules; the folder shown is white, the ones
    above it muted, each still a `Button` that goes back to it. The title bar's two sides are
    as wide as each other (`TITLE_SIDE_WIDTH_PX`), so the path sits in the window's middle.
  - **The places are four icons on the title bar** (Recents, Gallery, Assets, Trash), after
    the close button, a straight line either side of them, the one shown filled white,
    each with its name small under it, since Bevy has no tooltip to name an icon on hover.
    A place is one button, the icon over its name inside the white fill when it is shown,
    52 by 40 px with an 18 px icon and a 10 px name (first the close button's height and 2 px
    more, then made smaller at the user's word); it is built with
    `laid_out_button` (`ui/button.rs`, dev builds only), a button whose caller lays it out.
  - **The close button turns half red under the pointer**: a new `ButtonKind::Close`, bare
    at rest as Quiet is, filled with `DANGER` at half its alpha when hovered, behind
    `#[cfg(feature = "dev")]` as `Page::Dev` is, since only a dev window has one. The button
    tests' `KINDS` became a slice, so dev builds test it with the others.
    The sidebar holds
    only "Locations", the folders of the place shown, Gallery's or Assets', and is hidden
    for a place with none, Recents and Trash always, so the list takes its room. The spec
    had each place as a row of the sidebar with its folders under it; the user moved the
    icons to the top, then up to the title bar, so Recents leaves no empty column.
  - **Empty Trash is a button**, on the title bar's right while Trash is shown, with a
    question before it (`trash.rs`, a layer of its own at `EXPLORER_DIALOG_Z_INDEX`, 310,
    over the explorer). The spec had it on a right click of Trash's empty space, in step 8;
    the rest of Trash stays there. Its buttons are `words_button` (`ui/button.rs`, dev builds
    only), since `button()` takes a translation key and a dev tool's words are English.
  - **`Explorer` counts its changes to the disk** (`disk_changes`, part of `Shown`), so the
    list, the path and the sidebar list the folder again after one, though it is the same.
- **A review after step 6**, asked by the user, Claude's own pass and an independent one:
  - **Esc passes over a hidden modal**: photo mode hides the explorer but it stayed a
    `ClosesOnEsc`, so the first Esc dismissed it unseen. `apply_esc_action` now only weighs
    modals whose `InheritedVisibility` is on.
  - **`hold_cursor` asks once per change**: where the OS refuses to hold the cursor, Bevy
    puts the old mode back, and comparing with the window asked again every frame, a
    warning each time. It now keeps what it last asked in a `Local`.
  - **A double click is two presses on the row itself**: Bevy counts clicks per entity hit,
    and a first click on the name and a second on the date were two first clicks. The row's
    texts let the pointer through, and only a count of exactly 2 opens.
  - **Closing gives Esc and the cursor back at once**: the explorer drops `ClosesOnEsc` as
    it starts to slide away, and the Empty Trash question closes with it, so an answer
    never empties the trash from a window already gone.
  - **The preview shows the icon of a picture it cannot decode** (a compressed `.ktx2`, a
    broken PNG), instead of an empty frame.
  - **The tree reads the disk only when the folder shown or the open folders change**, not
    on every selection, which only moves the highlight; the list opens a new folder at its
    top; Recents walks no link to a folder, which could loop for ever; `InnerKeys` always
    exists.
  - **Constants**: the question's layer is `DEV_TOOLS_Z_INDEX` plus 10, not 310 by hand, and
    has its row in `docs/flow.md`; the list's heading reuses `BUTTON_PADDING_PX`.
  - **Split into files**, each part of the window on its own: `drawer.rs` (slide, height,
    grip), `title_bar.rs` (close, places, path), `tree.rs` (was `sidebar.rs`, now only the
    tree), `scroll.rs` (frames and scrollbars); `file_explorer.rs` keeps the action, opening
    and closing, `Explorer` and the window's layout. `Explorer` stays a resource, with the
    reason written beside it.
- **Steps 7 to 10, written together**, asked by the user to try them at once:
  - **`actions.rs` is three files**: `actions.rs` (every change to the disk, `Change`,
    `request_change` and `apply`, dragging, files from the OS, "+ Folder"), `menu.rs` (the
    right-click menus) and `rename.rs` (the field and the name rules). Whole it passed 900
    lines. The questions moved out of `trash.rs` into `dialog.rs`, since Trash, the menu and
    `usage.rs` all ask over the explorer on its layer, 310.
  - **Bevy's menu widget is not used**: `bevy_ui_widgets::menu` opens a list from a menu
    button and keeps it open through the keyboard focus; a right-click menu opens where the
    pointer is, a few lines of its own. It lets the pointer through around its sheet, as an
    OS menu does: a press elsewhere closes it and still lands where it was aimed.
  - **`Controls` still hears Esc while a text field has the keyboard**: the actions with no
    label, Esc alone, type nothing and are the way out of every panel, so the save name's Esc
    closes its overlay as before. The rename field is a `ClosesOnEsc` at a layer of its own,
    301, so the Esc rule closes a question, then the rename, then the explorer, with no arm
    of its own in `esc_action`.
  - **A click elsewhere with a refused name gives the rename up**, instead of keeping the
    field red and listening: the click went somewhere, and the field turns red as the name
    is typed, so Enter on it does nothing.
  - **"+ Folder" sits on the title bar's right**, where Empty Trash is, the two never shown
    together, not beside the path: the path's middle is the grip's.
  - **Recents renames nothing**: its names are paths through the gallery's folders. Its menu
    is Open and Delete.
  - **Trash keeps each entry under its own name**, ` 2` when two of one name are deleted, not
    under the time it was deleted: the list shows the name, `trash.ron` keeps the time, and
    Trash's date column is when it was deleted. Each row says where it came from
    (`from Gallery › trailer`).
  - **A folder dragged in from the OS is copied too**, not refused: the copy of a folder is
    the one between places already.

## Step 1: `feat: open a file explorer on Numpad 0`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 1.1 Esc closes the topmost modal: a `ClosesOnEsc` marker on every dialog's root, and the Esc rule's first line closes the one with the highest `GlobalZIndex`; its tests | Claude | `apply_esc_action` and `esc_action` in `overlays.rs`; `dialog()` in `ui/panel.rs` |
| [x] | 1.2 `src/dev_tools/file_explorer.rs`: the `FILE_EXPLORER` action (Numpad 0, "File Explorer"), opening and closing, silent in photo mode as every dev key | Claude | `KeyCode::Numpad0`; `.run_if(not(in_state(Pause::Photo)))` |
| [x] | 1.3 The window: a floating root on the dev tools' layer that blocks the pointer (so it has a backdrop), `DevUi`, `ClosesOnEsc`; the glass window inset from the screen's edges; its title bar with a close button and the place's name | Claude | `floating_root(DEV_TOOLS_Z_INDEX)` without `Pickable::IGNORE`; `dev_window()` placed by `top`, `bottom`, `left`, `right`; `icon_button(Icon::X, ButtonKind::Quiet)` |
| [x] | 1.4 `just run`: Numpad 0 opens and closes it over the menu, the pause menu and the running game; Esc and the close button close it; Esc with nothing else open does not pause; a dialog over the pause menu still closes first. Review | both | on `3d/player` the cursor stays locked over the running game until step 2 |

## Step 2 (on `3d/player`): `feat: free the cursor under a modal`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 2.1 `first_person.rs` locks the cursor and turns the view only in `Pause::Off` with no floating root that blocks the pointer | Claude | `Floating` in `ui::blur`, `Option<&Pickable>`, `CursorOptions::grab_mode`, `CursorOptions::visible` |
| [x] | 2.2 `just run`: over the running game the cursor shows and the view stays still; closing locks it again; the keys panel leaves it locked. Sync to `3d/player` | both | |

## Step 3: `feat: list a folder in the file explorer`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 3.1 `listing.rs`: the places (Gallery, Assets, Trash and their roots), reading a folder, the pure sorting, sizes, dates and kinds, and their tests | Claude | `FileAssetReader::get_base_path()` in `bevy::asset::io::file`; `gallery/` moves to `dev_tools.rs`; `civil_from_days` from `photo.rs` |
| [x] | 3.2 `list.rs`: the rows (icon, name, date, size), selecting on a click, entering on a double click; the title becomes the path, each part a button | Claude | `On<Pointer<Click>>` carries `count`; the `Explorer` resource holds the place, the folder and the selection |
| [x] | 3.3 `just run`: the photos listed, a folder entered and left. Review | both | |

## Step 4: `feat: preview the selected file`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 4.1 `preview.rs`: an image decoded on another thread and shown large, its details under it; another file shows its icon; a folder or nothing, an empty preview | Claude | `decode_picture` in `picture.rs` decodes bytes into an `Image`; a `Task` polled with `check_ready`, as photo mode's notice |
| [x] | 4.2 `just run`. Review | both | |

## Step 5: `feat: open a file in its own app`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 5.1 A double click on a file opens it with the OS's default app | Claude | `std::process::Command`: `explorer` on Windows, `open` on macOS, `xdg-open` elsewhere |
| [x] | 5.2 `just run`. Review | both | |

## Step 6: `feat: show the explorer's places in a sidebar`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 6.1 `sidebar.rs`: Recents, Gallery and Assets with one level of folders, Trash; the shown one highlighted; Recents' listing and its test | Claude | |
| [x] | 6.2 `just run`. Review | both | |

## Step 7: `feat: make and rename folders and files`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 7.1 `input.rs`: `Controls` hears nothing while the focused entity is a text field; its test | Claude | `Res<InputFocus>` in `bevy::input_focus`; a text field carries `EditableText` |
| [x] | 7.2 `actions.rs`: the right-click menus, "+ Folder", renaming in place, the name rules and their tests | Claude | `PointerButton::Secondary`; the save screens' `EditableText` + `TextInput` |
| [x] | 7.3 `just run`: typing E while renaming over photo mode takes no photo. Review | both | |

## Step 8: `feat: keep a trash in the file explorer`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 8.1 `trash.rs`: Delete into `trash/`, `trash.ron`, Restore, Delete for Good, Empty Trash, the dialogs; the tests on a `fresh_test_dir` | Claude | `ron` and `serde`, as `settings.rs` writes its file; `write_atomic` for `trash.ron` |
| [x] | 8.2 `just run`: a photo deleted, restored, deleted again, emptied; a file deleted from Assets comes back to Assets. Review | both | |

## Step 9: `feat: move files by dragging them`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 9.1 Dragging onto a folder, a place and a part of the path; copying between places; the tests | Claude | `Pointer<DragStart>`, `Pointer<DragDrop>` with `dropped` |
| [x] | 9.2 Files dropped from the OS copied into the folder shown | Claude | `FileDragAndDrop::DroppedFile` in `bevy::window` |
| [x] | 9.3 `just run`. Review | both | |

## Step 10: `feat: ask before changing an asset the game uses`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 10.1 `usage.rs`: the pure search and its tests; walking `src/` and the text files of `assets/` | Claude | the repository is `FileAssetReader::get_base_path()` |
| [x] | 10.2 The modal before renaming, moving or deleting in Assets | Claude | |
| [x] | 10.3 `just run`: deleting `menu/music.ogg` lists `src/screens/menu.rs`; Cancel leaves it. Review | both | |

## Step 11: `docs: mark the file explorer spec built`

| | Task | Who | Facts |
|---|------|-----|-------|
| [x] | 11.1 `README.md`, `docs/flow.md`, `docs/template.md`, the roadmap's row and pieces, the spec's status and "What changed" | Claude | |
| [ ] | 11.2 Sync; `just run` on `main` and on `2d/base` | you | |
