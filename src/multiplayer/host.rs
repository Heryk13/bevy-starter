use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use lightyear::prelude::server::{ClientOf, NetcodeConfig, NetcodeServer, ServerUdpIo, Start};
use lightyear::prelude::*;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

use super::remote::{Heading, RemoteIndex, forget_player, hear_pose};
use super::{
    DISCOVERY_EVERY_SECS, DISCOVERY_PORT, GAME_PORT, HOST_ID, MAX_PLAYERS, Network, NetworkedBody,
    PRIVATE_KEY, PROTOCOL_ID, Pose, PoseChannel, discovery_packet, pose_of,
};
use crate::i18n::Localized;
use crate::screens::{Pause, Screen};

// ==== Config =================================================================

// ---- the pause menu's switch: what it says, open or not ---------------------
const OPEN_TO_LAN_KEY: &str = "pause-open-to-lan";
const CLOSE_LAN_KEY: &str = "pause-close-lan";

// ==== Plugin =================================================================

/// Hosting: the pause menu's Open to LAN starts a server in this game, which keeps playing as
/// one of the players, and says so on the network every `DISCOVERY_EVERY_SECS`; the same
/// switch, or leaving gameplay, stops it. The host relays each joiner's pose to the others,
/// sends its own to all, and shows the joiners' bodies.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(press_lan_button);
    app.add_observer(hear_player_leave);
    app.add_systems(
        Update,
        (
            label_lan_buttons,
            (relay_poses, broadcast_game).run_if(resource_equals(Network::Hosting)),
        ),
    );
    app.add_systems(OnExit(Screen::Gameplay), close_lan);
}

// ==== Components =============================================================

/// Marks the pause menu's Open to LAN switch.
#[derive(Component)]
pub(crate) struct LanButton;

/// Marks the server this game runs while hosting.
#[derive(Component)]
pub(super) struct HostServer;

// ==== Resources ==============================================================

/// The socket the host broadcasts on, and when it last did.
#[derive(Resource)]
struct Broadcast {
    socket: UdpSocket,
    sent_secs: Option<f32>,
}

// ==== Systems ================================================================

/// Opens the game to the LAN, or closes it, when the switch is pressed: the game under the pause
/// menu runs while it is open, and freezes again once it closes.
fn press_lan_button(
    activate: On<Activate>,
    mut commands: Commands,
    buttons: Query<(), With<LanButton>>,
    mut network: ResMut<Network>,
    servers: Query<Entity, With<HostServer>>,
    pause: Option<Res<State<Pause>>>,
    mut time: ResMut<Time<Virtual>>,
) {
    if !buttons.contains(activate.entity) {
        return;
    }
    match *network {
        Network::Offline => {
            open_lan(&mut commands);
            *network = Network::Hosting;
            // pressed in the pause menu, which froze the game: a shared game never freezes, nor
            // runs slowed, since the host runs the world for everyone
            time.unpause();
            time.set_relative_speed(1.0);
        }
        Network::Hosting => {
            stop_servers(&mut commands, &servers);
            *network = Network::Offline;
            // alone again, under the pause menu the switch is in: frozen as it would have been
            if pause.is_some_and(|pause| *pause.get() == Pause::On) {
                time.pause();
            }
        }
        // a joiner's pause menu has no switch to press
        Network::Joining { .. } | Network::Joined => {}
    }
}

/// Writes on each switch whether pressing it opens the game or closes it.
fn label_lan_buttons(
    network: Res<Network>,
    buttons: Query<&Children, With<LanButton>>,
    mut labels: Query<&mut Localized>,
) {
    let key = if *network == Network::Hosting {
        CLOSE_LAN_KEY
    } else {
        OPEN_TO_LAN_KEY
    };
    for children in &buttons {
        for child in children {
            if let Ok(mut label) = labels.get_mut(*child)
                && label.0 != key
            {
                label.0 = key;
            }
        }
    }
}

/// Relays every joiner's pose to the other joiners, moving that player's body here too, and
/// sends the host's own pose to every joiner. The host, not the joiner, says whose pose it is.
fn relay_poses(
    mut commands: Commands,
    mut joiners: Query<(&RemoteId, &mut MessageReceiver<Pose>), With<ClientOf>>,
    server: Single<&Endpoint, With<HostServer>>,
    mut sender: ServerMultiMessageSender,
    body: Option<Single<&Transform, With<NetworkedBody>>>,
    mut index: ResMut<RemoteIndex>,
    mut headings: Query<&mut Heading>,
    time: Res<Time<Real>>,
) {
    for (id, mut receiver) in &mut joiners {
        let PeerId::Netcode(owner) = id.0 else {
            continue;
        };
        for mut pose in receiver.receive() {
            pose.owner = owner;
            hear_pose(
                &mut commands,
                &mut index,
                &mut headings,
                &pose,
                time.elapsed_secs(),
            );
            let to_others = NetworkTarget::AllExceptSingle(id.0);
            let _ = sender.send::<_, PoseChannel>(&pose, *server, &to_others);
        }
    }
    if let Some(body) = body {
        let pose = pose_of(&body, HOST_ID);
        let _ = sender.send::<_, PoseChannel>(&pose, *server, &NetworkTarget::All);
    }
}

/// Says on the network every `DISCOVERY_EVERY_SECS` that this game is open, and how many play.
fn broadcast_game(
    broadcast: Option<ResMut<Broadcast>>,
    time: Res<Time<Real>>,
    joiners: Query<(), With<ClientOf>>,
) {
    let Some(mut broadcast) = broadcast else {
        return;
    };
    let now = time.elapsed_secs();
    if broadcast
        .sent_secs
        .is_some_and(|sent| now - sent < DISCOVERY_EVERY_SECS)
    {
        return;
    }
    broadcast.sent_secs = Some(now);
    let players = 1 + u32::try_from(joiners.iter().count()).unwrap_or(MAX_PLAYERS);
    let target = SocketAddr::from((Ipv4Addr::BROADCAST, DISCOVERY_PORT));
    if let Err(err) = broadcast
        .socket
        .send_to(discovery_packet(players).as_bytes(), target)
    {
        warn!("could not tell the network the game is open: {err}");
    }
}

/// Takes away a joiner's body as the host hears it leave.
fn hear_player_leave(
    left: On<Remove<ClientOf>>,
    mut commands: Commands,
    ids: Query<&RemoteId>,
    mut index: ResMut<RemoteIndex>,
) {
    if let Ok(RemoteId(PeerId::Netcode(id))) = ids.get(left.entity) {
        forget_player(&mut commands, &mut index, *id);
    }
}

/// Closes the game to the LAN as gameplay ends: the joiners go back to their menu.
fn close_lan(
    mut commands: Commands,
    mut network: ResMut<Network>,
    servers: Query<Entity, With<HostServer>>,
) {
    if *network == Network::Hosting {
        stop_servers(&mut commands, &servers);
        *network = Network::Offline;
    }
}

// ==== Helpers ================================================================

/// Starts the server, listening on every address of this PC, and the broadcast.
fn open_lan(commands: &mut Commands) {
    let config = NetcodeConfig {
        protocol_id: PROTOCOL_ID,
        private_key: PRIVATE_KEY,
        // a LAN or Radmin friend dials this PC's own address, never the 0.0.0.0 it binds
        server_addr_check: false,
        ..default()
    }
    .with_max_clients(usize::try_from(MAX_PLAYERS - 1).unwrap_or(1));
    let server = commands
        .spawn((
            NetcodeServer::new(config),
            LocalAddr(SocketAddr::from((Ipv4Addr::UNSPECIFIED, GAME_PORT))),
            ServerUdpIo,
            HostServer,
        ))
        .id();
    commands.trigger(Start { entity: server });
    #[cfg(feature = "dev")]
    commands.insert_resource(super::OwnId(HOST_ID));
    match UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).and_then(|socket| {
        socket.set_broadcast(true)?;
        Ok(socket)
    }) {
        Ok(socket) => commands.insert_resource(Broadcast {
            socket,
            sent_secs: None,
        }),
        Err(err) => warn!("could not open the broadcast, joining by IP still works: {err}"),
    }
    info!("open to LAN on port {GAME_PORT}");
}

/// Stops the server and the broadcast: every joiner is disconnected.
fn stop_servers(commands: &mut Commands, servers: &Query<Entity, With<HostServer>>) {
    for server in servers {
        commands.entity(server).despawn();
    }
    commands.remove_resource::<Broadcast>();
    info!("closed to LAN");
}
