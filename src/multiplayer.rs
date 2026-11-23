use bevy::prelude::*;
use lightyear::prelude::client::ClientPlugins;
use lightyear::prelude::server::ServerPlugins;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use crate::screens::Screen;
use crate::screens::gameplay::map::MapFile;
use crate::ui;

// the props' files, which only the prop editor of a dev build adds
#[cfg(feature = "dev")]
mod files;
mod find_server;
mod host;
mod join;
mod props;
mod remote;
// read by each genre, which starts a joiner new to a world on the ring
pub(crate) mod world;

#[cfg(feature = "dev")]
pub(crate) use files::PropFileAdded;
pub(crate) use find_server::FindServerButton;
pub(crate) use host::LanButton;

// ==== Config =================================================================

// ---- the connection: netcode over UDP ---------------------------------------
const GAME_PORT: u16 = 5888;
// the same in every copy of the game: a game among friends, not a service to guard
const PROTOCOL_ID: u64 = 0xB5_7A_27_E2;
const PRIVATE_KEY: [u8; 32] = *b"bevy-starter lan co-op key 2026!";
// how often the games tell each other where everyone is: 60 a second
const TICK: Duration = Duration::from_micros(16_667);

// ---- players ----------------------------------------------------------------
// the host one of them; a genre places that many players at the start
pub(crate) const MAX_PLAYERS: u32 = 10;

// ---- finding games: a broadcast on the LAN ----------------------------------
const DISCOVERY_PORT: u16 = 5889;
const DISCOVERY_EVERY_SECS: f32 = 1.0;
// a game not heard for this long has closed, or left the network
const DISCOVERY_STALE_SECS: f32 = 3.5;
// the game's name and version: another game, or another version of this one, is not listed
const DISCOVERY_TAG: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

// ---- joining ----------------------------------------------------------------
const CONNECT_TIMEOUT_SECS: f32 = 6.0;

// ---- the others' bodies -----------------------------------------------------
// gone with no word for this long: the player left, or the connection did
const REMOTE_SILENT_SECS: f32 = 2.0;
// how fast another body catches up with its last pose: most of the way in a tenth of a second
const REMOTE_EASE_PER_S: f32 = 15.0;

// ==== Plugin =================================================================

/// Playing together over the network, a player hosting and up to `MAX_PLAYERS - 1` joining:
/// the host opens the game to the LAN from the pause menu, the others find it from the main
/// menu, or type its address. Each game moves its own player's body and sends where it stands;
/// the host relays every pose to everyone, and each game shows the others as `RemotePlayer`s,
/// which the genre draws. Its parts are files of their own: hosting, joining, Find Server, and
/// the others' bodies. A game without multiplayer deletes this file, its folder and every line
/// marked `// multiplayer`.
pub(crate) fn plugin(app: &mut App) {
    // both in every game: each group registers messages of its own, numbered in the order they
    // are registered, and a game with one group would number them unlike a game with the other
    app.add_plugins(ClientPlugins {
        tick_duration: TICK,
    });
    app.add_plugins(ServerPlugins {
        tick_duration: TICK,
    });
    // after the plugin groups, before any connection is spawned, as lightyear asks
    app.register_message::<Pose>()
        .add_direction(NetworkDirection::Bidirectional);
    app.add_channel::<PoseChannel>(ChannelSettings {
        mode: ChannelMode::UnorderedUnreliable,
        ..default()
    })
    .add_direction(NetworkDirection::Bidirectional);
    app.register_message::<WorldState>()
        .add_direction(NetworkDirection::ServerToClient);
    app.add_channel::<WorldChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
        ..default()
    })
    .add_direction(NetworkDirection::Bidirectional);
    app.init_resource::<Network>();
    app.init_resource::<PendingNotice>();
    #[cfg(feature = "dev")]
    app.init_resource::<OwnId>();
    app.add_systems(OnEnter(Screen::Menu), show_pending_notice);
    app.add_plugins((
        host::plugin,
        join::plugin,
        find_server::plugin,
        remote::plugin,
        world::plugin,
        props::plugin,
    ));
    #[cfg(feature = "dev")]
    app.add_plugins(files::plugin);
}

// ==== Run Conditions =========================================================

/// Tells whether this game is shared with others, hosting or joined: a shared game never
/// freezes, since the others keep playing in it.
pub(crate) fn is_shared_game(network: Option<Res<Network>>) -> bool {
    network.is_some_and(|network| matches!(*network, Network::Hosting | Network::Joined))
}

/// Tells whether this game joined another's: the world is the host's, not this player's.
pub(crate) fn is_joined(network: Option<Res<Network>>) -> bool {
    network.is_some_and(|network| *network == Network::Joined)
}

// ==== Network ================================================================

/// Where this game stands on the network: alone, hosting, joining a host, or joined.
#[derive(Resource, Clone, Copy, Default, PartialEq, Debug)]
pub(crate) enum Network {
    #[default]
    Offline,
    Hosting,
    Joining {
        since_secs: f32,
    },
    Joined,
}

/// Where a player's body stands and faces, sent by its own game: the host's `owner` is
/// `HOST_ID`, a joiner's the id the host knows it by, set by the host whatever the joiner says.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
struct Pose {
    owner: u64,
    position: Vec3,
    yaw: f32,
}

/// The channel poses go by: unreliable, since a lost pose is replaced by the next a tick later.
struct PoseChannel;

/// The host's world as a player joins it: which world it is, and its props, so the joiner sees
/// the host's map and is put back where it stood the last time it was in this world.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct WorldState {
    world_id: u64,
    map: MapFile,
}

/// The channel the world goes by: reliable and in order, since nothing replaces a lost edit.
struct WorldChannel;

// the host's own player, which no joiner's id can be: netcode ids come from the joiners' clocks
const HOST_ID: u64 = u64::MAX;

// ==== Components =============================================================

/// Marks the body this game moves and sends to the others: the genre puts it on its player.
#[derive(Component)]
pub(crate) struct NetworkedBody;

/// Another player's body, by the id the host knows that player by: the genre observes it being
/// added to draw it, and `remote.rs` moves it.
#[derive(Component)]
pub(crate) struct RemotePlayer {
    pub(crate) id: u64,
}

// ==== Resources ==============================================================

/// A notice to show as the menu comes back: why a shared game ended for this player.
#[derive(Resource, Default)]
struct PendingNotice(Option<&'static str>);

/// The id the others know this game's player by: `HOST_ID` hosting, its netcode id joined.
// only the prop editor's locks read it, and only dev builds have the editor
#[cfg(feature = "dev")]
#[derive(Resource, Clone, Copy, Default)]
struct OwnId(u64);

// ==== Systems ================================================================

/// Shows the pending notice over the menu, once.
fn show_pending_notice(mut commands: Commands, mut pending: ResMut<PendingNotice>) {
    if let Some(message) = pending.0.take() {
        commands.spawn((notice(message), DespawnOnExit(Screen::Menu)));
    }
}

// ==== Notice =================================================================

// it lives here, not with the dialogs in ui.rs, since only multiplayer tells the player things
// with an OK alone: a game without multiplayer would keep it unused
/// Returns a modal notice with OK alone, ready to spawn over whatever it tells about.
/// `message` is a translation key.
pub(super) fn notice(message: &'static str) -> impl Bundle {
    (
        ui::floating_root(ui::DIALOG_Z_INDEX),
        ui::Dialog,
        ui::ClosesOnEsc,
        children![(
            ui::card(ui::DIALOG_WIDTH_PX),
            children![
                (ui::label(message), TextLayout::justify(Justify::Center)),
                (
                    Node {
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    children![(
                        ui::button("dialog-ok", ui::ButtonKind::Primary),
                        ui::CloseDialog
                    )],
                ),
            ],
        )],
    )
}

// ==== Helpers ================================================================

/// Returns the pose of a body standing at `transform`, for `owner`.
fn pose_of(transform: &Transform, owner: u64) -> Pose {
    Pose {
        owner,
        position: transform.translation,
        yaw: transform.rotation.to_euler(EulerRot::YXZ).0,
    }
}

/// Returns what the host broadcasts: the tag, the game port, and how many play of how many.
fn discovery_packet(players: u32) -> String {
    format!("{DISCOVERY_TAG} {GAME_PORT} {players} {MAX_PLAYERS}")
}

/// A game heard on the network: its port, and how many play of how many.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Heard {
    port: u16,
    players: u32,
    max: u32,
}

/// Reads a broadcast packet, or `None` when it is not this game's, at this version.
fn read_discovery(text: &str) -> Option<Heard> {
    let mut parts = text.split_whitespace();
    if parts.next()? != DISCOVERY_TAG {
        return None;
    }
    Some(Heard {
        port: parts.next()?.parse().ok()?,
        players: parts.next()?.parse().ok()?,
        max: parts.next()?.parse().ok()?,
    })
}

/// Returns the address a player typed, `GAME_PORT` when it has none, or `None` when it is not
/// an address.
fn typed_address(text: &str) -> Option<SocketAddr> {
    let text = text.trim();
    text.parse::<SocketAddr>().ok().or_else(|| {
        text.parse::<IpAddr>()
            .ok()
            .map(|ip| SocketAddr::new(ip, GAME_PORT))
    })
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_broadcast_reads_back_as_sent() {
        let heard = read_discovery(&discovery_packet(3));
        assert_eq!(
            heard,
            Some(Heard {
                port: GAME_PORT,
                players: 3,
                max: MAX_PLAYERS,
            })
        );
    }

    #[test]
    fn another_game_or_version_is_not_listed() {
        assert_eq!(read_discovery("other-game/1.0.0 5888 1 10"), None);
        assert_eq!(read_discovery("garbage"), None);
        assert_eq!(read_discovery(""), None);
    }

    #[test]
    fn a_typed_address_takes_the_game_port_when_it_has_none() {
        assert_eq!(
            typed_address(" 192.168.0.5 "),
            Some(SocketAddr::from(([192, 168, 0, 5], GAME_PORT)))
        );
        assert_eq!(
            typed_address("26.1.2.3:6000"),
            Some(SocketAddr::from(([26, 1, 2, 3], 6000)))
        );
        assert_eq!(typed_address("not an address"), None);
    }
}
