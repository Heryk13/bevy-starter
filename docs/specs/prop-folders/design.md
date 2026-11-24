# Prop folders: the prop editor's window as a small file explorer, shared in a shared game

Status: **in progress** 2026-11-24, on `main`.
Date: 2026-11-24
Bevy: 0.20.0

## Goal

The prop editor's window (F10) lists `assets/props/` the way a file explorer does: folders to
keep the props in order, and a right click on a cell for what is done to it. On a prop's file:
Favourite (or Unfavourite), Rename, Delete. On a folder: Rename, Delete. On the grid's empty
space: New Folder. The star under each cell goes: Favourite is in the menu now. In a shared game
a folder made, a name changed or a file deleted reaches everyone's disk, through the host.

The user's words, 2026-11-24: "com o botão direito vai ter esses botões, favoritar, renomear,
excluir", "tira a estrelinha embaixo do card", "criar pastas, renomear pastas e tudo isso
sincronizado, tipo um file explorer mesmo", "o favorites é individual", "o recent são os
recents usados", and "o botão direito é apenas o botão direito, não quero que o item fique
selecionado".

## Decided

| Question | Decision |
| --- | --- |
| A right click that selects | Bevy 0.20's buttons press under any pointer button (`button_on_pointer_down` inserts `Pressed` without looking at which), so a right click activated every button of the game, a cell taking its prop into the hand. Fixed once for every button, in `ui/button.rs`: the button that pressed is remembered, and a `Pressed` a non-primary button made is taken back before the release can activate it |
| Where folders live | Inside `assets/props/`, any depth. A prop's asset path keeps its folders (`props/trees/pine.glb`), so the map, the saves and the thumbnails need nothing new |
| What the grid shows | The folder open: its folders first, a folder icon each, then its props. A click on a folder opens it; a back button in the toolbar, beside the path (`Props › trees`), goes up. Search looks in every folder, the results flat. Favourites and Recent stay flat lists of files, wherever they are |
| `PropLibrary` | Unchanged in shape: every prop's file, in every folder, as the genres' thumbnails read it (`prop_editor_3d.rs`, `prop_editor_2d.rs`), so no genre branch changes. The folders are a resource of their own, `PropFolders` |
| Favourites and recents | The developer's own, in their data folder, as before: never shared. Recent stays the props placed last. A rename carries both to the new path |
| Rename | In a small field where the menu was: Enter keeps it, Esc or a click elsewhere gives it up, red while the name is refused. The rules are the file explorer's (`is_name_allowed`), shared from there; a prop keeps an extension its genre takes. Only the name changes: the folder stays the same |
| What a rename changes | The file or folder on disk, and every asset path that starts with it: the props of the game in progress (`Session::game.map`), the live props' `PropFile`, the favourites and the recents. Saves on disk are not rewritten: an older save names the old path, and its props wait, skipped, as any prop whose file is gone does, until the name comes back |
| Delete | A file or a folder goes to the file explorer's Trash, which can put it back; every prop placed from it is deleted, as the Delete key does |
| New Folder | `New Folder`, or the first number after it that is free, as the explorer names one, in the folder open |
| Sharing | One message, `PropFileEdit` (`NewFolder`, `Rename`, `Delete`), the way `MapEdit` goes: alone or hosting, made at once; joined, sent to the host, which makes it and sends it to every joined game, the asker too, each making it on its own disk. The props a delete removes go as `MapEdit::Delete`s, through the host as always |
| Paths over the network | A path received is taken only when every part of it is a plain name: no `..`, no root, no drive, nothing starting with `.`; a file must have an extension its genre takes. The props' files now travel with their folders (`PropList` and `FileChunk` name `trees/pine.glb`) |

## Ceilings

- A folder with nothing in it reaches the games that are joined when it is made, but not a game
  that joins later: `PropList` lists files.
- Saves on disk keep the old path after a rename (see above).

## Implementation order

One commit per step, each with `just check`.

1. `fix: keep the right mouse button from pressing buttons`: `ui/button.rs`.
2. `feat: open folders in the prop editor`: `library.rs` reads every folder, `PropFolders`;
   `window.rs` shows the folder open, the back button and the path, and loses the star.
3. `feat: favourite, rename and delete props from their menu`: `cell_menu.rs` grows the menus
   and the rename field; the disk changes and the path rewriting in a file of their own,
   `prop_editor/files.rs`.
4. `feat: share prop folders, renames and deletes in a shared game`: `multiplayer/files.rs`.
5. `docs: ...`: README, `docs/flow.md`, `docs/template.md`, and this spec's status.

## What changed while building it

Nothing yet.
