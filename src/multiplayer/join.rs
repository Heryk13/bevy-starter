use bevy::prelude::*;
use lightyear::prelude::client::{NetcodeClient, NetcodeConfig};
use lightyear::prelude::*;
use std::net::{Ipv4Addr, SocketAddr};

use super::remote::{Heading, RemoteIndex, hear_pose};
use super::world::{FreshInWorld, joiner_session, keep_character};
use super::{
    CONNECT_TIMEOUT_SECS, Network, NetworkedBody, PRIVATE_KEY, PROTOCOL_ID, PendingNotice, Pose,
    PoseChannel, WorldState, pose_of,
};
use crate::save::Session;
use crate::screens::{LoadingDestination, Screen, StartGame};
use crate::ui::notice;

// ==== Config =================================================================

// ---- the notices a joiner may get -------------------------------------------
const COULD_NOT_JOIN_KEY: &str = "notice-could-not-join";
const HOST_ENDED_KEY: &str = "notice-host-ended";

// ==== Plugin =================================================================

/// Joining: connecting to a host by its address, then a new game straight into gameplay once
/// connected. The joiner sends its own body's pose and hears everyone else's from the host.
/// Not connected in time, or the host gone, it says so; the host gone from gameplay, it goes
/// back to the menu.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            (enter_host_game, give_up_joining).chain(),
            (send_pose, hear_poses, notice_host_gone).run_if(resource_equals(Network::Joined)),
        ),
    );
    app.add_systems(OnExit(Screen::Gameplay), leave_host_game);
}

// ==== Components =============================================================

/// Marks this game's connection to a host.
#[derive(Component)]
pub(super) struct HostLink;

// ==== Systems ================================================================

/// Starts the game in the host's world as the host sends it, right after connecting: the
/// joiner's character as it last stood in this world, or a new one starting on the ring.
fn enter_host_game(
    mut commands: Commands,
    mut receivers: Query<&mut MessageReceiver<WorldState>, With<HostLink>>,
    mut network: ResMut<Network>,
    mut fresh: ResMut<FreshInWorld>,
) {
    if !matches!(*network, Network::Joining { .. }) {
        return;
    }
    let Some(state) = receivers
        .iter_mut()
        .find_map(|mut receiver| receiver.receive().last())
    else {
        return;
    };
    let (session, fresh_here) = joiner_session(state);
    fresh.0 = fresh_here;
    *network = Network::Joined;
    info!("joined the host's game");
    commands.trigger(StartGame {
        session,
        destination: LoadingDestination::Gameplay,
    });
}
/// Gives up a connection not made within `CONNECT_TIMEOUT_SECS`, a full game's included, and
/// says the game could not be joined.
fn give_up_joining(
    mut commands: Commands,
    mut network: ResMut<Network>,
    time: Res<Time<Real>>,
    links: Query<Entity, With<HostLink>>,
) {
    let Network::Joining { since_secs } = *network else {
        return;
    };
    if time.elapsed_secs() - since_secs < CONNECT_TIMEOUT_SECS {
        return;
    }
    for link in &links {
        commands.entity(link).despawn();
    }
    *network = Network::Offline;
    commands.spawn((notice(COULD_NOT_JOIN_KEY), DespawnOnExit(Screen::Menu)));
}

/// Sends where this player's body stands to the host, every frame it is connected.
fn send_pose(
    mut senders: Query<&mut MessageSender<Pose>, (With<HostLink>, With<Connected>)>,
    body: Option<Single<&Transform, With<NetworkedBody>>>,
) {
    let Some(body) = body else {
        return;
    };
    // the host sets whose pose it is: the id here is never read
    let pose = pose_of(&body, 0);
    for mut sender in &mut senders {
        sender.send::<PoseChannel>(pose);
    }
}

/// Moves every other player's body to the poses the host relays.
fn hear_poses(
    mut commands: Commands,
    mut receivers: Query<&mut MessageReceiver<Pose>, With<HostLink>>,
    mut index: ResMut<RemoteIndex>,
    mut headings: Query<&mut Heading>,
    time: Res<Time<Real>>,
    screen: Res<State<Screen>>,
) {
    for mut receiver in &mut receivers {
        for pose in receiver.receive() {
            if *screen.get() == Screen::Gameplay {
                hear_pose(
                    &mut commands,
                    &mut index,
                    &mut headings,
                    &pose,
                    time.elapsed_secs(),
                );
            }
        }
    }
}

/// Sends the player back to the menu, with a notice, when the host is gone: it closed LAN,
/// left gameplay, quit, or the connection dropped.
fn notice_host_gone(
    mut commands: Commands,
    links: Query<(Entity, Has<Connected>), With<HostLink>>,
    mut network: ResMut<Network>,
    mut pending: ResMut<PendingNotice>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if links.iter().any(|(_, connected)| connected) {
        return;
    }
    for (link, _) in &links {
        commands.entity(link).despawn();
    }
    *network = Network::Offline;
    pending.0 = Some(HOST_ENDED_KEY);
    next_screen.set(Screen::Menu);
}

/// Leaves the host's game as this player leaves gameplay: Quit to Menu, or a save loaded.
fn leave_host_game(
    mut commands: Commands,
    links: Query<Entity, With<HostLink>>,
    mut network: ResMut<Network>,
    mut session: ResMut<Session>,
) {
    if *network != Network::Joined {
        return;
    }
    keep_character(&mut session);
    for link in &links {
        commands.trigger(Disconnect { entity: link });
        commands.entity(link).try_despawn();
    }
    *network = Network::Offline;
}

// ==== Helpers ================================================================

/// Starts connecting to the host at `address`, as Find Server's Join asks.
pub(super) fn join(
    commands: &mut Commands,
    network: &mut Network,
    now_secs: f32,
    address: SocketAddr,
) {
    // any id the host has not seen: the clock, mixed with this process's own
    let client_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |time| u64::try_from(time.as_nanos()).unwrap_or(u64::MAX))
        ^ u64::from(std::process::id());
    let auth = Authentication::Manual {
        server_addr: address,
        client_id,
        private_key: PRIVATE_KEY,
        protocol_id: PROTOCOL_ID,
    };
    let Ok(netcode) = NetcodeClient::new(auth, NetcodeConfig::default()) else {
        error!("could not start connecting to {address}");
        return;
    };
    let link = commands
        .spawn((
            netcode,
            LocalAddr(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))),
            PeerAddr(address),
            UdpIo::default(),
            HostLink,
        ))
        .id();
    commands.trigger(Connect { entity: link });
    #[cfg(feature = "dev")]
    commands.insert_resource(super::OwnId(client_id));
    *network = Network::Joining {
        since_secs: now_secs,
    };
    info!("joining {address}");
}
