# File explorer: browse the gallery and the assets on Numpad 0

Status: **built** on 2026-11-19, approved the same day.
Date: 2026-11-19
Bevy: 0.20.0
Roadmap: row 4 of `docs/roadmap/roadmap.md`, from its "File explorer, as agreed" section. The
look is the mockup's block "4" in `docs/roadmap/ui/mockup.html`.

## Goal

Photo mode (row 3) fills `gallery/` in the data folder, and screen recording (row 8) will too.
Finding a photo means leaving the game for the OS's own explorer and a folder buried in
`%APPDATA%`. And building the map (row 13) means putting files into `assets/`: a model to
spawn, a texture, a sound. The file explorer brings both folders into the game: Numpad 0 opens
a window that works like the Windows explorer, with a large preview of what is selected,
folders to sort things into, a file opened in the OS's own app with a double click, and files
dragged in from the OS's explorer copied where the window is.

## Decided

| Question | Decision |
| --- | --- |
| A dev tool | Yes: Numpad 0 is on the roadmap's table of dev keys. `src/dev_tools/file_explorer.rs` and its parts, dev builds only, English words |
| Key | File Explorer (Numpad 0), an action on Settings' Dev page, so the keys panel lists it. Numpad 0 again closes the window, and so does its close button |
| Where it opens | Anywhere, the running game included, chosen by the user. The game keeps running under it, as the prop editor's will (row 13): nothing freezes. On `3d/player` the cursor comes free while it is open and the view stops following the mouse (below) |
| The window | A large glass window, centred, the screen's size less a margin, blocking clicks to what is under it: a floating root that does not let the pointer through, so it has the backdrop every modal has |
| What it shows | Three places, each a root the path never climbs out of: **Gallery**, `gallery/` in the data folder, the player's photos and videos; **Assets**, the folder Bevy loads the game's assets from (`FileAssetReader::get_base_path()` joined with `assets`, the repository's `assets/` under `just run`); and **Trash**, `trash/` in the data folder, what was deleted from the other two. The rest of the data folder (saves, settings) is not shown: the game keeps those itself |
| Assets is the game's own | A file renamed, moved or deleted there is one the code may load by name, so the game may stop finding it. Before any of the three, the explorer looks for where it is used (below), and when it is, asks first |
| Where an asset is used | Asked by the user. Renaming, moving or deleting a file or folder in Assets first searches the repository's text for its path: every `.rs` in `src/`, and the text files of `assets/` (shaders, `.ron`, `.ftl`, `.gltf`). When it is found, a modal lists each use as `src/screens/menu.rs:126` beside the line itself, with Cancel and a red Rename Anyway, Move Anyway or Delete Anyway. Not found, it goes ahead without a word |
| Files from the OS | A file dragged from the OS's explorer onto the window is copied into the folder shown (Bevy's `FileDragAndDrop::DroppedFile`), with ` 2`, ` 3` before its extension if the name is taken. A folder dragged in is refused, with a line in the log. Row 13's props window will take dropped files the same way into `assets/props/` |
| Layout | The mockup's: a title bar (close button, the path), a sidebar, the list, the preview |
| The path | The title is the path from its root (`Gallery › trailer`, `Assets › props`), each part a button that goes back to it. No back and forward arrows: the path already goes back, and nothing yet goes forward |
| The sidebar | Recents, then under "Locations" Gallery and Assets, each with its folders under it, then Trash, the folder shown highlighted. Only one level of folders is listed under Gallery and Assets; deeper ones are reached through the list |
| Recents | Every file in the gallery, its folders included, newest first. Not the assets: hundreds of them would bury the photos |
| The list | Folders first, then files, each sorted by name. Columns: name, date, size. A folder's size is "—" |
| Selecting and opening | One click selects. A double click enters a folder, or opens a file in the OS's default app |
| Opening a file | `explorer` on Windows, `open` on macOS, `xdg-open` on Linux, through `std::process::Command`: no crate |
| The preview | A selected image (PNG, JPEG), large, over its name, its size in pixels, its file size and its date. Any other file shows its icon over the same lines but the pixels; a folder or nothing selected shows an empty preview. A model's preview (a `.glb` turned on a turntable) is row 13's, which renders models into thumbnails anyway |
| Right click on a file or folder | A menu: Open, Rename, Delete |
| Right click on empty space | A menu: New Folder |
| "+ Folder" | The one button besides the close button, beside the path: a new folder, named `New Folder`, `New Folder 2`, ... ready to be renamed |
| Renaming | The name in the list becomes the text field the save screens use (`EditableText` and `TextInput`); Enter keeps it, Esc gives up, a click elsewhere keeps it |
| A name refused | Empty, with `/`, `\`, `:` or the other characters Windows refuses, `.` or `..`, or a name the folder already has: the field turns red and keeps listening |
| Deleting | Moves the file or folder into the explorer's own Trash, chosen by the user, with no question asked, as a trash never asks. Not the OS's trash: that would need the `trash` crate in every game made from the template, and a trash of its own is a folder and a list |
| Trash | Lists what was deleted, newest first, each with where it came from. Right click: Restore (back where it was, ` 2` if that name is taken now, its folders made again if they are gone) or Delete for Good (after a dialog, "Delete «name» for good?", Yes in red). Right click on empty space: Empty Trash, after the same kind of dialog. Nothing is renamed, made or dropped in it, and nothing is moved into it but by Delete |
| How Trash remembers | Each deleted entry is moved into `trash/` under a name of its own (the time it was deleted, then its name), and `trash/trash.ron` keeps, for each, where it came from: the place and the path inside it. An entry the file does not know (put there by hand) restores nowhere; it can only be deleted for good. Moving out of Assets crosses into the data folder, so it copies and then removes, as moving between places does |
| Moving | Drag a file or a folder onto a folder in the list, onto a place or a folder in the sidebar, or onto a part of the path. Dropping a folder into itself or into its own folders does nothing. Between Gallery and Assets it copies, as the Windows explorer does between disks, since the two may sit on different disks, and `std::fs::rename` cannot cross them |
| Dates | The file's last change, in UTC, as the photos are named (`2026-11-18 14:32`), from the calendar code photo mode already has |
| Sizes | `2.4 MB`, `980 KB`, `12 B`: the unit that keeps the number under 1000, one decimal |
| Icons | By kind, from the extension: a folder, an image, a film (row 8's videos), a box for a model (`.glb`, `.gltf`), a note for a sound, a page for anything else |

Recents stays, the user's answer: one entry at the top of the sidebar, the photos newest first
whatever folder they were sorted into, so the photo just taken is always at the top.

## Out of scope

- **Search**, which the mockup shows: a gallery of photos taken by hand is short, and sorted by
  the time in its names.
- **Back and forward arrows**: the path does their work.
- **Selecting several files** at once, copying, and undoing (Trash undoes a delete).
- **The OS's own trash**: the explorer keeps its own.
- **Files outside the two places**: saves and settings stay where the game keeps them.
- **Spawning a model into the world** from the explorer: row 13's props window, which reads
  `assets/props/`, the folder the explorer fills.
- **Playing a video or a sound in the preview**: a double click opens it in the OS's app.

## Structure

### Files

`file_explorer.rs` would pass 400 lines whole, and its parts make sense alone, so it is split
the `screens.rs` way from the start:

| File | Holds |
| --- | --- |
| `src/dev_tools/file_explorer.rs` | the action, opening and closing, the window and its title bar, which folder is shown and what is selected |
| `src/dev_tools/file_explorer/listing.rs` | reading a folder and Recents from the disk, the pure sorting, names, sizes and dates, and their tests |
| `src/dev_tools/file_explorer/list.rs` | the list: its rows, selecting, entering, opening, dragging |
| `src/dev_tools/file_explorer/sidebar.rs` | Recents, Gallery and Assets with their folders, Trash |
| `src/dev_tools/file_explorer/preview.rs` | the preview: the image decoded on another thread, and what is written under it |
| `src/dev_tools/file_explorer/actions.rs` | the right-click menu, renaming, New Folder, moving, files dropped from the OS, and the disk calls behind them |
| `src/dev_tools/file_explorer/trash.rs` | deleting into Trash, `trash.ron`, Restore, Delete for Good, Empty Trash |
| `src/dev_tools/file_explorer/usage.rs` | where an asset is used: the search and its modal |

### State

The window is open while a `FileExplorer` root exists, as the keys panel is. What it shows is
one resource, `Explorer`: the place shown (Recents, or a folder under Gallery or Assets), the
selected entry, and the entry being renamed, if any. Every part redraws from it when it
changes, so a rename, a move or a delete only has to change the disk and read the folder again.

`gallery/` and its name move out of photo mode's `photo.rs` to `dev_tools.rs`, now that two
tools use them. The assets folder's path comes from Bevy, so it is wherever Bevy loads from,
the repository under `just run`.

### The disk

Every disk call is `std::fs`. The explorer writes no file's contents, so `write_atomic` has
no part here: it renames, moves, deletes and makes folders. A failure is logged with
`error!`, and the folder is read again, so the window always shows the disk as it is. Moving
within one place is `std::fs::rename`; between the two places, and from the OS, it is
`std::fs::copy`, a folder copied file by file.

Every disk call goes through a function of `listing.rs` or `actions.rs` that takes its root's
path, so its tests run in a `fresh_test_dir` and never touch the player's folder or the
repository's assets.

Bevy hot-reloads what it has already loaded from `assets/` in dev builds, so a file the
explorer changes there that the game uses is read again; one it does not use is left alone.

### Typing and the actions

While the rename field has the keyboard, the letters typed must not also be actions: an E
typed over photo mode would take a photo. `Controls` already hears nothing while a binding is
being chosen; it also hears nothing while the focused entity (`InputFocus`) is a text field.
That change lives in `input.rs` and helps the save screens too, where a name typed under the
pause menu could otherwise reach a game action.

### Where an asset is used

The code names every asset it loads by its path under `assets/`, as text: `"menu/music.ogg"`,
a `*_PATH` constant, `include_bytes!("../assets/fonts/...")`, and a folder read whole by
`load_folder("intro/frames")`. So a search of that text finds almost every use, with no crate
and nothing to keep in step:

- **A file** is used where its path under `assets/` (`menu/background.jpg`) appears anywhere
  in a line, and where a folder it sits in appears as a whole string (`"intro/frames"`,
  `"intro"`), since a folder read whole uses every file in it.
- **A folder** is used where its path appears followed by a `/` (`menu/`), or as a whole
  string, or where a folder it sits in does.
- The search reads `src/` and the text files of `assets/`, from the repository Bevy loads the
  assets from: the folder above `assets/` (`FileAssetReader::get_base_path()`). A build run
  outside the repository has no `src/` there; it then finds nothing and says so in the log
  once. It never reads `target/`.

What it cannot see: a path put together at run time (`format!("level_{n}.glb")`), and a model
that names its textures inside a binary `.glb`. Both are rare in this template; the modal is a
warning, not a guard, and Cancel is always one click away.

The search is a pure function from a path and the files' text to the lines that use it,
tested on text written in the test; walking the folders is a thin function around it. The
template's `src/` and `assets/` are a few hundred KB, read when the question is asked, on the
main thread.

### Over the running game

The explorer opens anywhere and freezes nothing, so the game goes on under it, keys
included: on `3d/player` the player can still walk while it is open, unless a name is being
typed. What it cannot do is look around, since the mouse now points at the window.

That part is `3d/player`'s, the only branch that locks the cursor. `first_person.rs` locks it
and turns the view only while the game runs (`Pause::Off`) **and no floating root that blocks
the pointer is open**; when one opens it frees the cursor and stops the view, and when the
last one closes it locks the cursor again. It reads `Floating` and `Pickable` the way
`blur.rs` does, so it never names the explorer, and any later modal over the running game
frees the cursor the same way. A dev window that lets the pointer through (the keys panel,
slow motion) leaves the cursor locked, as today.

`main` has no locked cursor, so there is nothing to change there; and a helper on `main` that
only `3d/player` calls would be dead code on `main`, which `just check` refuses.

### Esc

Esc closes, in order, the dialog, then the rename (giving it up), then the explorer, before
the rest of the Esc rule. The explorer is a dev tool, so the arm sits behind
`#[cfg(feature = "dev")]` as photo mode's does, and `esc_action` gains one input: whether the
explorer is open.

### Look

The window is glass like a card, with the dev windows' padding and radius. Its parts follow
the mockup: the sidebar a slightly darker glass inside it, the list's rows lit white under the
pointer and the selected one filled white with ink letters, as a `Choice` button is. The
right-click menu is a small glass popover where the pointer is.

## Text

English literals, like every dev tool: "Gallery", "Recents", "Name", "Date", "Size", "Open",
"Rename", "Delete", "New Folder", "Trash", "Restore", "Delete for Good", "Empty Trash", and the
dialogs' questions. `dialog()` takes a translation key, so the explorer's dialogs are built
from `floating_root` and `card` with plain words, as every dev tool's text is.

## Tests

- `listing.rs`: folders before files and each by name; a folder's entries read back from a
  `fresh_test_dir`; Recents across folders newest first; sizes (`12 B`, `980 KB`, `2.4 MB`) and
  dates; the path's parts from a folder inside a root; a file's kind from its extension.
- `actions.rs`: a name refused for each reason and one accepted; `New Folder 2` after
  `New Folder`, `tree 2.glb` after `tree.glb`; rename, move, copy between two roots, a file
  dropped from outside on a `fresh_test_dir`; a folder never moved into itself.
- `trash.rs`: a deleted file lands in `trash/` and `trash.ron` says where it came
  from; restored, it is back where it was; restored onto a name now taken, it gets ` 2`;
  restored into a folder since deleted, the folder is made again; deleted for good and
  emptied, it is gone from both.
- `usage.rs`: a file found by its path in a `load` line and in an `include_bytes!`; a file in a
  folder read whole found by that folder; a folder found by its path and a `/`; a path that
  only starts the same (`menu/background.jpg` against `menu/background.jpg.bak`) found once,
  as one line; nothing found for a file no line names.
- `input.rs`: no action heard while a text field has the focus.
- `overlays.rs`: Esc closes the explorer before it pauses or resumes anything.

## Implementation order

Every step compiles, passes `just check` and runs, written and tried on `3d/player`, committed
on `main`, but step 2, which is `3d/player`'s own. The user syncs when there is enough to
send; step 2 then needs a sync of its own, to `3d/player`.

1. **The window.** The action, opening and closing anywhere, the glass window with its title
   bar and close button, Esc. *Try:* Numpad 0 opens and closes it over the menu, the pause
   menu and the running game.
2. **The cursor, on `3d/player`.** `first_person.rs` frees the cursor and stops the view while
   a floating root that blocks the pointer is open. *Try:* over the running game the cursor
   shows and the view stays still; closing the explorer locks it again; the keys panel and
   slow motion still leave it locked.
3. **The list.** `listing.rs` and its tests, the rows with their icons, selecting, entering a
   folder, the path going back; it opens on Gallery. *Try:* the photos are listed, a folder is
   entered and left.
4. **The preview.** *Try:* a photo selected shows large with its details; a `.glb` shows its
   icon and details; a folder shows none.
5. **Opening a file.** *Try:* a double click opens the photo in the OS's viewer.
6. **The sidebar.** Recents, Gallery and Assets with their folders, Trash. *Try:* each shows
   what it should; Assets lists the repository's `assets/`.
7. **Folders and names.** "+ Folder", the right-click menus, renaming with the text field and
   `Controls` silent while typing. *Try:* each on a photo and a folder; typing E while
   renaming over photo mode takes no photo.
8. **Trash.** Delete into Trash, `trash.ron`, Restore, Delete for Good and Empty Trash with
   their dialogs. *Try:* a photo deleted, restored, deleted again and emptied; a file deleted
   from Assets comes back to Assets.
9. **Moving and dropping.** Dragging onto a folder, the sidebar and the path; copying between
   Gallery and Assets; files dropped from the OS's explorer. *Try:* a photo dragged into a
   folder lands there; a `.glb` dragged from Windows onto the window lands in the folder shown.
10. **Where an asset is used.** `usage.rs` and its tests, the modal before renaming, moving or
    deleting in Assets. *Try:* deleting `menu/music.ogg` lists `src/screens/menu.rs`, and
    Cancel leaves it; renaming a file in `loading/` lists the folder's `load_folder`; a file
    nothing names goes without a modal.
11. **Docs.** `README.md`, `docs/flow.md` (the window, the Esc rule, `Controls` while typing),
    `docs/template.md`, the roadmap's row and its table of pieces, this spec's status.

## What changed while building it

Each change, with its reason, is in `docs/roadmap/file-explorer-tasks.md`'s departures. In short:

- **A drawer, not a window over everything**: it slides up over the bottom third of the
  screen, lets the pointer through around it and has no backdrop, so the game stays sharp
  above it; a grip resizes it, and Esc slides it away too.
- **Esc closes the topmost modal**, any `ClosesOnEsc`, by its layer: no arm of its own in
  `esc_action`. The rename field is one too, at 301, so the order is question, rename,
  explorer. The cursor on `3d/player` follows the same marker, not `Floating` and `Pickable`.
- **The places are icons on the title bar** (Recents, Gallery, Assets, Trash), and the sidebar
  is a tree of the place's folders and files, as a code editor shows one.
- **More files**: `drawer.rs`, `title_bar.rs`, `tree.rs` (for `sidebar.rs`), `scroll.rs`,
  `dialog.rs`, `menu.rs` and `rename.rs` beside the ones planned, each part of the window on
  its own. `Explorer` stays a resource, with the reason beside it.
- **Shared code it brought**: `ButtonKind::Close`, `words_button` and `laid_out_button` (dev
  builds), a pointing hand over everything pressable, a button repainted when its kind changes,
  `Dismiss`, and the tools' inner keys left out of the keys panel.
- **`Controls` still hears Esc while a text field has the keyboard**, the one action with no
  label; a click elsewhere with a refused name gives the rename up.
- **Empty Trash and "+ Folder" are buttons** on the title bar's right; Trash keeps each entry
  under its own name, its time in `trash.ron`; Recents renames nothing; a folder dropped in
  from the OS is copied like a file.
- **Bevy's menu widget is not used**: it opens from a menu button through the keyboard focus,
  and a right-click menu opens where the pointer is.
