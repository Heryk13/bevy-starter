# Multiplayer world: shared props, prop files through the host, dev tools in a shared game

Status: **built** 2026-11-23, tried with two windows on one PC.
Date: 2026-11-23
Bevy: 0.20.0, lightyear 0.31.0
Roadmap: row 18, parts 4, 5 and 6 of 8 (`docs/specs/multiplayer-core/design.md` lists them),
from its "as agreed" section; the user asked for every remaining part to be built and tried on
their PC.

## Goal

The friends in a game build its map together: a prop one places shows for all, a prop in one
player's hand is locked for the others, and the host's physics moves every physics prop for
everyone. A prop's file the others lack goes to them through the host, both ways. Slow motion
is off in a shared game, and photo mode never freezes one (part 3 did that).

## Decided

| Question | Decision |
| --- | --- |
| Who hands out ids | The host only. The editor's `MapEdit` became the ask; `apply_map_edit` turns it into a `MapChange` (`Placed` with its id, `Updated`, `Deleted`), which `apply_map_change` applies. Alone, both happen at once; hosting, every change goes to every joiner, the asker included; joined, the edit goes to the host (`apply_map_edit` does not run there) and its change comes back |
| `Transform` over the network | Written as its three parts (`transform_parts`, `serde(with)`): Bevy writes its own `Transform` only with its `serialize` feature, which this game does not build |
| A prop in someone's hand | `Held` on a placed prop asks the host for a lock; the host keeps who holds what (`Locks`), sends it all whenever it changes, and each game keeps its player from picking up what the others hold (`LockedProps`, in `edits.rs`, empty alone). A leaver's locks go with it |
| Physics props | The host's physics runs them; the host sends the physics props that moved each frame (`PropPoses`, unreliable), and a joined game puts them there. A player branch makes them kinematic in a joined game (`player/others.rs`), so its own physics never fights the host's |
| Save as Default Map | The host's: the button does nothing in a joined game |
| Prop files | Dev builds only. The host lists its props (name and size) to each player that connects, and again within `LIST_EVERY_SECS` (2 s) of its folder changing; a joiner asks for each it lacks or holds at another size; files go in `CHUNK_BYTES` (16 KB) pieces on a channel of their own, so a big file never holds an edit up. A file dropped into a joined game's editor goes to the host, whose next list sends it on. A file that arrives makes the map's props that wait for it appear |
| Safeguards | A bare name only (no `/`, `\`, `:`, `..`, nor a leading dot), a prop's extension only (`PropKind`), nothing over `MAX_FILE_BYTES` (50 MB), written with `write_atomic`, never run. A crafted file could still crash Bevy's loader: join only friends' games, as agreed |
| Dev and release | Both peers must be dev builds, or both release: the editor's messages exist only in dev builds, and messages are numbered in the order they are registered |
| Slow motion | Neither opens nor steps in a shared game; opening LAN or joining sets the time's speed back to 1× |
| Markers | Tried on a copy of `3d/player`: `multiplayer.rs`, `multiplayer/`, `player/others.rs` and every line ending `// multiplayer` deleted, the rest builds and passes clippy, dev and release |

## Out of scope

Holding a prop shows the others only where it is put down, not on its way there.

## What changed while building it

- **A `resting` is always written** (found at the first two-window try): `skip_serializing_if`
  left the field out of a prop's bytes, and lightyear's binary format reads every field in
  order, so the host's world could not be read. The default map now writes `resting: None`.
- **A joiner asks for files as soon as it reaches the host** (found at the first file try): the
  list the host sends on connecting came before the world, and the asking ran only once in the
  game, so that first list was lost until the folder changed.
