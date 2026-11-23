# Props in saves: each save keeps its own props, the default map starts a new game

Status: **built** 2026-11-23, on `main` and `3d/player`. Not yet tried in the game (`just run`
pending).
Date: 2026-11-23
Bevy: 0.20.0
Roadmap: a change to row 13 (the prop editor, `docs/specs/prop-editor/design.md`, built), asked
for after it was built.

## Goal

Each save keeps the props of its own game: one save has a giant pine, another does not. A new
game starts with the default map, `assets/map.ron`. The prop editor (F10) edits the game that is
running, and a button in its window, Save as Default Map, writes that game's props as the
default. A physics prop pushed around is saved where it came to rest; the default map keeps
where each prop was placed.

## Why

The built editor wrote every edit into `assets/map.ron`, the one map of every game, so two saves
could not differ, and a test in one game changed every other (the user, 2026-11-23).

## Decided

| Question | Decision |
| --- | --- |
| Where a game's props live | In `GameData`, beside the player, as `map: Option<MapFile>`: the same `MapFile` the default map is, so a save holds every prop whole, its ids and `next_id` included. A save holding only the changes from the default map would break the day the default changed |
| A new game, an old save | `None`: the game has not read the default map yet. On entering gameplay a `None` becomes a copy of `assets/map.ron`, so a new game and a save written before this field both start with the default map, and the new game needs no change |
| What the editor edits | The game running: `apply_map_edit` changes `Session::game.map`, never a file, and the next save, autosaves included, keeps it (the user's call, option A) |
| The default map | Save as Default Map, a button in the props window, writes the running game's props to `assets/map.ron`, in dev builds only (the user's call, option A). It writes where each prop was placed, never where physics left it, so a test never rearranges the default |
| A pushed physics prop | Saved where it came to rest (the user's call, option A): `PlacedProp` gains `resting: Option<PropRest>`, the position and rotation physics left it at, written into the session each frame on `3d/player`, as the player's own state is. It spawns there when it has one. Placing or moving it in the editor clears it, since it was placed again. Not written when `None`, so the default map stays as it was |
| 2D | No physics, so never a `resting`; the rest is the same, from `main` |
| Save version | Unchanged: `GameData` is `serde(default)`, so an old save loads with `map: None` |

## Out of scope

- A dialog before Save as Default Map overwrites the file: it is a dev tool, and the file is
  committed, so git shows and undoes the change.
- Props that change in other ways during play (broken, picked up by the player).

## Structure

- `main`, `save.rs`: `GameData` gains `map: Option<MapFile>` (`3d/player` and `2d/player` keep
  their `player` beside it).
- `main`, `screens/gameplay/map.rs`: `PlacedProp` gains `resting`; `transform()` uses it;
  `spawn_map_props` fills a `None` from the default map and spawns the session's props.
- `main`, `dev_tools/prop_editor/edits.rs`: `apply_map_edit` edits the session; `applied` clears
  `resting` on `Place` and `Update`. `window.rs`: the Save as Default Map button, which writes
  with `write_map`, every `resting` cleared.
- `3d/player`, `screens/gameplay/props.rs`: `record_prop_rests`, in `Pause::Off`, writes each
  physics prop's position and rotation into its `resting`, the one in the hand aside.

## Tests

- `map.rs`: a prop with a `resting` spawns there, one without at its placement; a `None` map
  becomes the default map; a `resting` of `None` is not written.
- `edits.rs`: placing and updating clear `resting`; the default map written from a session has
  no `resting`.

## Implementation order

1. **Props in the save, on `main`.** `GameData::map`, `resting`, the spawn from the session.
2. **The editor edits the game, on `main`.** `apply_map_edit` on the session, Save as Default
   Map. Merged down.
3. **Physics props saved where they rest, on `3d/player`.**
4. **Docs.** `README.md`, `docs/flow.md`, `docs/template.md`, this spec's status.

## What changed while building it

- **Steps 1 and 2 are one commit** (found while building 1): `MapFile::as_placed`, which strips
  `resting`, only has a caller in Save as Default Map, so step 1 alone failed `just check`'s
  `dead_code`.
- **`map: Option<MapFile>`**, the whole `MapFile` in `GameData`, ids and `next_id` included, so the
  editor carries on numbering in a loaded game.
- **A physics prop only gets a `resting` once it moved**: more than `REST_MOVE_M` (1 cm) or
  `REST_TURN_RAD` from where it was placed, so the settling of a prop put down is not a push.
- **Only a dynamic body is recorded**, which leaves out a prop held in the editor's hand (it is
  kinematic then) with no dev-only code in `props.rs`, a file every build has.
