# Multiplayer core: open a game to the LAN, find it, join it, see each other

Status: **built** 2026-11-23, tried with two windows on one PC; Radmin between two PCs still to try.
Date: 2026-11-23
Bevy: 0.20.0, lightyear 0.31.0
Roadmap: row 18, its part 2 of 8 (the user's plan of 2026-11-23, below), from its "as agreed"
section; the spike on the branch `spike/multiplayer` proved the pieces.

## The plan this is part of

The user's goal: "me and a friend walking in the same playroom, him placing props and them
showing for me, the maps in sync; only the host saves the default map". Eight parts, each a spec:

1. Spike (done, branch `spike/multiplayer`).
2. **This: the core.** Open to LAN, Find Server, join by IP, the others seen, the host leaving.
3. Pause and save split: Esc never freezes a shared game; the host saves the world, each player
   their own character.
4. The shared world: the host's props for whoever joins, the shared prop editor (a held prop
   locked for the others), physics props, Save as Default Map for the host only.
5. Prop files sent both ways through the host, with the agreed safeguards.
6. Dev tools in multiplayer, and the `// multiplayer` markers checked for the CLI.
7. Gamepad support (split screen's players 2 and up play on gamepads).
8. Split screen.

## Goal

In gameplay, the pause menu's Open to LAN opens the game to the network, and its switch closes
it. In the main menu, Find Server lists the games open on the network by themselves, a row each
with its address and players (`2/10`), refreshed every few seconds and by a Refresh button, and a
field takes an IP for a game that does not show. Joining puts the player in the playroom, where
the others are seen as the same capsule; nobody has a name. Ten players at most, the host one of
them. When the host leaves, closes LAN or quits, everyone else goes back to the menu with a
notice that the host ended the game.

## Decided

| Question | Decision |
| --- | --- |
| Who moves a body | Each player's own game, sending where the body stands and faces; the host relays every pose to everyone (the user's call, option A). No prediction, no rollback |
| The crate | `lightyear` 0.31 with `std`, `client`, `server`, `netcode`, `udp` only: its `replication` and `interpolation` corrupted messages and panicked in the spike (`lightyear_interpolation` `timeline.rs:299`), and messages are all option A needs. Both `ClientPlugins` and `ServerPlugins` in every game, since each group registers messages of its own, numbered in the order they are registered, and a peer with one group numbers them unlike a peer with the other |
| The connection | netcode over UDP on `GAME_PORT`, the host binding every address, `server_addr_check` off: a LAN or Radmin friend dials the host's own address, never the `0.0.0.0` it binds. The key and protocol id are constants: a game among friends, not a service |
| Finding games | The host broadcasts a short packet on `DISCOVERY_PORT` every `DISCOVERY_EVERY_SECS` (the game's name and version, its game port, its players); Find Server listens and lists each address heard, dropping one silent for `DISCOVERY_STALE_SECS`. The spike heard it through the Radmin adapter too. A second game on the same PC cannot listen on the same port: it can still join by IP |
| Where it lives | `main`: `src/multiplayer.rs` and its folder, every part a genre needs none of (the connection, discovery, Find Server, the LAN switch, the host leaving, relaying poses, the others' bodies moving). `3d/player` and `2d/player`: putting `NetworkedBody` on their player, and what another player looks like (an observer on `Add<RemotePlayer>`). A pose is a `Vec3` and a yaw for both genres |
| Markers | Every file of it is `multiplayer.rs` and its folder; every line outside them that names it ends with `// multiplayer`, so the CLI can remove it whole (row 21) |
| Joining | Connected, the joiner starts a new game straight into gameplay (the loading screen, then the playroom). Its own save, the host's world and props are parts 3 and 4: until then it plays the default map and saves as it does now |
| A failed join | Not connected within `CONNECT_TIMEOUT_SECS`, or refused (the game is full), shows a notice: could not join |
| Ten players | `MAX_PLAYERS = 10`, the host one: netcode takes `MAX_PLAYERS - 1` clients |
| The host leaving | Closing LAN or leaving gameplay stops the server; each joiner, disconnected, goes back to the menu with a notice: the host ended the game |
| The others' bodies | A `RemotePlayer` entity per other player, its `Transform` eased toward the last pose heard; gone after `REMOTE_SILENT_SECS` with no pose, and as the host hears it leave |
| Text | Locale keys in English, Brazilian Portuguese and Japanese: the two buttons, Find Server's title, its Refresh, IP field and empty list, the notices; a `notice()` widget beside `dialog()` in `ui/panel.rs`, with OK alone |

## Out of scope

Parts 3 to 8. A dedicated server, NAT punching, names.

## Structure

- `multiplayer.rs`: the plugin, the protocol (the `Pose` message and its channel), `Network`
  (offline, hosting, joining, joined), constants, `NetworkedBody`, `RemotePlayer`.
- `multiplayer/host.rs`: Open to LAN, the server, the broadcast, relaying.
- `multiplayer/join.rs`: connecting, the joiner's sending and hearing, the host leaving.
- `multiplayer/find_server.rs`: the overlay, its list and its IP field.
- `multiplayer/remote.rs`: the others' bodies, eased and forgotten.

## Tests

Pure helpers: the discovery packet written and read back; a packet from another game or version
refused; a stale address dropped from the list; an IP typed with or without the port; the next
eased step toward a pose.

## Implementation order

1. **The core, on `main`.** Everything above but the genres' lines.
2. **The 3D body**, on `3d/player`. *Try:* two windows on one PC, one Open to LAN, the other
   Find Server: both capsules walk.
3. **The 2D body**, on `2d/player`.
4. **Docs.**

## What changed while building it

- **A player branch's part is a file of its own too**, `player/others.rs` (decided while building
  2): drawing the others is an observer, a function no marker line can carry, so the file goes
  whole with `multiplayer.rs` and its folder. The CLI's list of what multiplayer removes is
  `multiplayer.rs`, `multiplayer/`, `player/others.rs` and the marked lines.
- **A joiner starts on a ring around the spawn point** (found at the first two-window try, the
  user saw it too): both bodies started in the same place, one inside the other, and physics
  threw one into the air. A place for each player already in, `MAX_PLAYERS` places round
  `JOINER_RING_RADIUS_M` (2 m) in 3D and two tiles in 2D; the host keeps the middle.
- **Joining shows no "connecting" line**: Find Server closes on Join and the menu waits up to
  `CONNECT_TIMEOUT_SECS` before the game starts or the notice comes.
- **The others in 3D are kinematic capsules**, so the player bumps into them and never pushes
  them, since each player's own game moves them.
