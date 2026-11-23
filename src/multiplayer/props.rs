use bevy::platform::collections::HashMap;
use bevy::prelude::*;
#[cfg(feature = "dev")]
use lightyear::prelude::server::ClientOf;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use super::host::HostServer;
use super::join::HostLink;
#[cfg(feature = "dev")]
use super::{HOST_ID, OwnId, WorldChannel};
use super::{Network, PoseChannel};
#[cfg(feature = "dev")]
use crate::dev_tools::prop_editor::edits::{Held, LockedProps, MapChange, MapEdit};
use crate::screens::Screen;
use crate::screens::gameplay::map::{Prop, PropId, PropMode};

// ==== Plugin =================================================================

/// The props of a shared world: the host's physics runs every prop that moves, and its games
/// that joined follow where the host says each one is. In dev builds, where the prop editor is,
/// every edit goes through the host: a joined game sends what it asks, the host makes it and
/// sends what it made to everyone, the asker included, so only the host hands out ids; and a
/// prop in someone's hand is locked for everyone else until it is put down.
pub(super) fn plugin(app: &mut App) {
    app.register_message::<PropPoses>()
        .add_direction(NetworkDirection::ServerToClient);
    app.add_systems(
        Update,
        (
            send_prop_poses.run_if(resource_equals(Network::Hosting)),
            follow_prop_poses.run_if(resource_equals(Network::Joined)),
        )
            .run_if(in_state(Screen::Gameplay)),
    );
    #[cfg(feature = "dev")]
    editing(app);
}

/// The editor's half, dev builds only: edits, their changes and the locks, through the host.
// both peers must be dev builds, or both release, for their messages to number alike: a
// release build has no editor, and registers none of these
#[cfg(feature = "dev")]
fn editing(app: &mut App) {
    app.register_message::<MapEdit>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<MapChange>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<LockAsk>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<LockState>()
        .add_direction(NetworkDirection::ServerToClient);
    app.init_resource::<Locks>();
    app.add_observer(send_edit_to_host);
    app.add_observer(send_change_to_joiners);
    app.add_observer(lock_held_prop);
    app.add_observer(unlock_put_down_prop);
    app.add_observer(unlock_leaver_props);
    app.add_systems(
        Update,
        (
            (make_joiner_edits, hear_lock_asks, share_locks)
                .chain()
                .run_if(resource_equals(Network::Hosting)),
            (hear_changes, hear_locks).run_if(resource_equals(Network::Joined)),
        ),
    );
}

// ==== Messages ===============================================================

/// Where the props that move stand on the host, sent to the games that joined.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct PropPoses(Vec<PropPose>);

/// Where one prop stands.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
struct PropPose {
    id: PropId,
    position: Vec3,
    rotation: Quat,
}

/// A joined game's player took a prop into the hand, or let it go.
#[cfg(feature = "dev")]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
struct LockAsk {
    id: PropId,
    held: bool,
}

/// Every prop in someone's hand, with whose hand: the host sends it whenever it changes.
#[cfg(feature = "dev")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
struct LockState(Vec<(PropId, u64)>);

// ==== Resources ==============================================================

/// The host's record of every prop in someone's hand, by whose: `HOST_ID` for its own player.
#[cfg(feature = "dev")]
#[derive(Resource, Default, PartialEq)]
struct Locks(HashMap<PropId, u64>);

// ==== Systems ================================================================

/// Sends where every physics prop that moved this frame stands, unreliably: a lost pose is
/// replaced by the next.
fn send_prop_poses(
    moved: Query<(&Prop, &PropMode, &Transform), Changed<Transform>>,
    server: Single<&Endpoint, With<HostServer>>,
    mut sender: ServerMultiMessageSender,
) {
    let poses: Vec<PropPose> = moved
        .iter()
        .filter(|(_, mode, _)| **mode == PropMode::Physics)
        .map(|(prop, _, transform)| PropPose {
            id: prop.0,
            position: transform.translation,
            rotation: transform.rotation,
        })
        .collect();
    if !poses.is_empty() {
        let _ = sender.send::<_, PoseChannel>(&PropPoses(poses), *server, &NetworkTarget::All);
    }
}

/// Puts each physics prop where the host says it stands. A genre makes them kinematic in a
/// game joined: the host's physics moves them, not this game's.
fn follow_prop_poses(
    mut receivers: Query<&mut MessageReceiver<PropPoses>, With<HostLink>>,
    mut props: Query<(&Prop, &mut Transform)>,
) {
    let mut latest: HashMap<PropId, PropPose> = HashMap::default();
    for mut receiver in &mut receivers {
        for poses in receiver.receive() {
            for pose in poses.0 {
                latest.insert(pose.id, pose);
            }
        }
    }
    if latest.is_empty() {
        return;
    }
    for (prop, mut transform) in &mut props {
        if let Some(pose) = latest.get(&prop.0) {
            transform.translation = pose.position;
            transform.rotation = pose.rotation;
        }
    }
}

/// Sends an edit this joined game's player asks to the host, which makes it and sends back
/// what it made.
#[cfg(feature = "dev")]
fn send_edit_to_host(
    edit: On<MapEdit>,
    network: Res<Network>,
    mut senders: Query<&mut MessageSender<MapEdit>, (With<HostLink>, With<Connected>)>,
) {
    if *network != Network::Joined {
        return;
    }
    for mut sender in &mut senders {
        sender.send::<WorldChannel>(edit.clone());
    }
}

/// Makes the edits the games that joined ask, as if this game's player had asked them.
#[cfg(feature = "dev")]
fn make_joiner_edits(
    mut commands: Commands,
    mut receivers: Query<&mut MessageReceiver<MapEdit>, With<ClientOf>>,
) {
    for mut receiver in &mut receivers {
        for edit in receiver.receive() {
            commands.trigger(edit);
        }
    }
}

/// Sends every change the host makes to every game that joined, the one that asked included.
#[cfg(feature = "dev")]
fn send_change_to_joiners(
    change: On<MapChange>,
    network: Res<Network>,
    server: Option<Single<&Endpoint, With<HostServer>>>,
    mut sender: ServerMultiMessageSender,
) {
    let (Network::Hosting, Some(server)) = (*network, server) else {
        return;
    };
    let _ = sender.send::<_, WorldChannel>(&*change, *server, &NetworkTarget::All);
}

/// Applies the changes the host sends, as this game's own.
#[cfg(feature = "dev")]
fn hear_changes(
    mut commands: Commands,
    mut receivers: Query<&mut MessageReceiver<MapChange>, With<HostLink>>,
) {
    for mut receiver in &mut receivers {
        for change in receiver.receive() {
            commands.trigger(change);
        }
    }
}

/// Locks a placed prop as this game's player takes it into the hand: on the host its own
/// lock, in a joined game an ask to the host.
#[cfg(feature = "dev")]
fn lock_held_prop(
    add: On<Add<Held>>,
    props: Query<&Prop>,
    network: Res<Network>,
    mut locks: ResMut<Locks>,
    mut senders: Query<&mut MessageSender<LockAsk>, (With<HostLink>, With<Connected>)>,
) {
    let Ok(prop) = props.get(add.entity) else {
        return;
    };
    ask_lock(*network, &mut locks, &mut senders, prop.0, true);
}

/// Unlocks a prop as this game's player lets it go: put down, put back, or gone.
#[cfg(feature = "dev")]
fn unlock_put_down_prop(
    remove: On<Remove<Held>>,
    props: Query<&Prop>,
    network: Res<Network>,
    mut locks: ResMut<Locks>,
    mut senders: Query<&mut MessageSender<LockAsk>, (With<HostLink>, With<Connected>)>,
) {
    let Ok(prop) = props.get(remove.entity) else {
        return;
    };
    ask_lock(*network, &mut locks, &mut senders, prop.0, false);
}

/// Records the locks the games that joined ask for, each under its own player.
#[cfg(feature = "dev")]
fn hear_lock_asks(
    mut receivers: Query<(&RemoteId, &mut MessageReceiver<LockAsk>), With<ClientOf>>,
    mut locks: ResMut<Locks>,
) {
    for (id, mut receiver) in &mut receivers {
        let PeerId::Netcode(owner) = id.0 else {
            continue;
        };
        for ask in receiver.receive() {
            set_lock(&mut locks, ask.id, owner, ask.held);
        }
    }
}

/// Lets go of every prop a player held as it leaves.
#[cfg(feature = "dev")]
fn unlock_leaver_props(
    left: On<Remove<ClientOf>>,
    ids: Query<&RemoteId>,
    mut locks: ResMut<Locks>,
) {
    if let Ok(RemoteId(PeerId::Netcode(owner))) = ids.get(left.entity) {
        locks.0.retain(|_, holder| holder != owner);
    }
}

/// Sends every lock to every game that joined whenever they change, and keeps the host's own
/// player from picking up what the others hold.
#[cfg(feature = "dev")]
fn share_locks(
    locks: Res<Locks>,
    server: Single<&Endpoint, With<HostServer>>,
    mut sender: ServerMultiMessageSender,
    mut locked: ResMut<LockedProps>,
) {
    if !locks.is_changed() {
        return;
    }
    let state = LockState(locks.0.iter().map(|(id, owner)| (*id, *owner)).collect());
    let _ = sender.send::<_, WorldChannel>(&state, *server, &NetworkTarget::All);
    locked.0 = held_by_others(&state, HOST_ID);
}

/// Keeps this joined game's player from picking up what the others hold.
#[cfg(feature = "dev")]
fn hear_locks(
    mut receivers: Query<&mut MessageReceiver<LockState>, With<HostLink>>,
    own: Res<OwnId>,
    mut locked: ResMut<LockedProps>,
) {
    for mut receiver in &mut receivers {
        for state in receiver.receive() {
            locked.0 = held_by_others(&state, own.0);
        }
    }
}

// ==== Helpers ================================================================

/// Locks or unlocks `id` for this game's player: in the host's record when hosting, by an ask
/// to the host when joined, and not at all alone.
#[cfg(feature = "dev")]
fn ask_lock(
    network: Network,
    locks: &mut Locks,
    senders: &mut Query<&mut MessageSender<LockAsk>, (With<HostLink>, With<Connected>)>,
    id: PropId,
    held: bool,
) {
    match network {
        Network::Hosting => set_lock(locks, id, HOST_ID, held),
        Network::Joined => {
            for mut sender in senders.iter_mut() {
                sender.send::<WorldChannel>(LockAsk { id, held });
            }
        }
        Network::Offline | Network::Joining { .. } => {}
    }
}

/// Records `owner` holding `id`, or letting it go: a prop already held by another is theirs.
#[cfg(feature = "dev")]
fn set_lock(locks: &mut Locks, id: PropId, owner: u64, held: bool) {
    if held {
        locks.0.entry(id).or_insert(owner);
    } else if locks.0.get(&id) == Some(&owner) {
        locks.0.remove(&id);
    }
}

/// Returns the props in `state` held by someone other than `own`.
#[cfg(feature = "dev")]
fn held_by_others(state: &LockState, own: u64) -> bevy::platform::collections::HashSet<PropId> {
    state
        .0
        .iter()
        .filter(|(_, owner)| *owner != own)
        .map(|(id, _)| *id)
        .collect()
}

// ==== Tests ==================================================================

#[cfg(all(test, feature = "dev"))]
mod tests {
    use super::*;

    #[test]
    fn a_prop_held_is_the_first_holders_until_they_let_it_go() {
        let mut locks = Locks::default();
        set_lock(&mut locks, PropId(1), 7, true);
        set_lock(&mut locks, PropId(1), 9, true);
        assert_eq!(locks.0.get(&PropId(1)), Some(&7));
        set_lock(&mut locks, PropId(1), 9, false);
        assert_eq!(locks.0.get(&PropId(1)), Some(&7));
        set_lock(&mut locks, PropId(1), 7, false);
        assert!(locks.0.is_empty());
    }

    #[test]
    fn a_player_is_never_locked_out_of_what_it_holds() {
        let state = LockState(vec![(PropId(1), 7), (PropId(2), 9)]);
        let locked = held_by_others(&state, 7);
        assert!(!locked.contains(&PropId(1)));
        assert!(locked.contains(&PropId(2)));
    }
}
