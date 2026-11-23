# Multiplayer saves: a shared game never freezes, the host keeps the world, a joiner its character

Status: **built** 2026-11-23, tried with two windows on one PC.
Date: 2026-11-23
Bevy: 0.20.0, lightyear 0.31.0
Roadmap: row 18, part 3 of 8 (`docs/specs/multiplayer-core/design.md` lists them), from its "as
agreed" section; the user asked for every remaining part to be built and tried on their PC.

## Goal

Esc in a shared game opens the pause menu without freezing anything: the others keep playing,
and only that player stands still. The host's game is the world, saved by the host as ever. A
player who joins gets the host's world (its props, and an id) as it connects, and keeps only its
own character, on its own PC, under that world: joining the same world again puts it back where
it stood; a first visit starts on the ring around the spawn point.

## Decided

| Question | Decision |
| --- | --- |
| The pause | Hosting or joined (`is_shared_game`), entering `Pause::On` does not pause `Time<Virtual>`; walking runs in `Pause::Off` only, so the player stands still under the menu while the world runs. Opening LAN from the pause menu unpauses the time it froze; closing LAN there freezes it again. Photo mode (`Pause::Photo`) the same, part 6's ask, done here since it is the same line |
| A world's id | `GameData::world_id`, a `u64` given as gameplay begins when it is 0 (a new game, a save from before): the clock mixed with the process id, never 0. A joiner keeps the host's |
| Joining | The host sends `WorldState` (the id and the map) on a reliable channel the moment a player connects; the joiner starts its game when it arrives, not when it connects |
| The joiner's character | `characters.ron` in the data folder, a `GameData` per world id, the map left out: where it stood, how it looked, its camera. Kept every `CHARACTER_KEEP_EVERY_SECS` (10 s) and as it leaves gameplay; keeping counts its play as saved, so quitting asks nothing |
| A first visit | `FreshInWorld`: true when the world has no character kept, and only then does a genre put the joiner on the ring |
| A joiner's pause menu | No Save, no Load Game: hidden by their label keys, so the pause menu names nothing of multiplayer. No autosave either |
| Old saves | Still load: `GameData` is `serde(default)`, and `world_id` 0 gets an id. The roadmap expected old saves to break; nothing needed it |

## What changed while building it

Nothing yet.
