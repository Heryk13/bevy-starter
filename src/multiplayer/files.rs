use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use lightyear::prelude::server::ClientOf;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use super::Network;
use super::host::HostServer;
use super::join::HostLink;
use crate::dev_tools::prop_editor::PropKind;
use crate::dev_tools::prop_editor::files::{PropFileChange, PropFileEdit, is_safe_path};
use crate::dev_tools::prop_editor::library::{
    PROPS_DIR_NAME, inside_props, is_prop_file, props_dir, read_props,
};
use crate::disk::write_atomic;
use crate::save::Session;
use crate::screens::Screen;
use crate::screens::gameplay::map::{Prop, prop_bundle};

// ==== Config =================================================================

// ---- sending a file: in pieces small enough for one packet's worth of messages -
const CHUNK_BYTES: usize = 16 * 1024;
// a file received is never bigger: a prop larger than this is no prop, and a game that sent
// one would only be filling the friend's disk
const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;
// how often the host reads its props folder again, to tell the others of a file added
const LIST_EVERY_SECS: f32 = 2.0;

// ==== Plugin =================================================================

/// The props' files and folders, through the host, dev builds only: the host tells each game
/// that joins which props it has, in which folders, and again whenever its folder changes;
/// each asks for those it lacks, and gets them in pieces; a file dropped into a joined game's
/// editor goes to the host, which lists it to the others. A folder made, a name changed or a
/// file deleted goes as the map's edits do: a joined game sends its `PropFileEdit` to the host,
/// which makes it and sends the `PropFileChange` it made to every joined game, the asker too.
/// Receiving a path is a door into the PC, so only a path inside the props folder is kept
/// (`is_safe_path`), only a prop's extension is taken, nothing larger than `MAX_FILE_BYTES` is
/// written, and a file is written whole or not at all. The game only ever loads a file
/// received as a model or an image, never runs it.
pub(super) fn plugin(app: &mut App) {
    app.register_message::<PropList>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<FileAsk>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<FileChunk>()
        .add_direction(NetworkDirection::Bidirectional);
    app.register_message::<PropFileEdit>()
        .add_direction(NetworkDirection::ClientToServer);
    app.register_message::<PropFileChange>()
        .add_direction(NetworkDirection::ServerToClient);
    app.add_channel::<FileChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
        ..default()
    })
    .add_direction(NetworkDirection::Bidirectional);
    app.init_resource::<Incoming>();
    app.add_observer(list_props_to_joiner);
    app.add_observer(upload_dropped_file);
    app.add_observer(send_file_edit_to_host);
    app.add_observer(send_file_change_to_joiners);
    app.add_systems(
        Update,
        (
            (
                list_props_when_they_change,
                send_asked_files,
                receive_uploads,
                make_joiner_file_edits,
            )
                .run_if(resource_equals(Network::Hosting)),
            // from the moment the host is reached, not only once in its game: the list it sends as
            // a player connects comes before the world, and a message unread in its frame is lost
            (ask_for_missing_props, receive_downloads),
            hear_file_changes.run_if(resource_equals(Network::Joined)),
        ),
    );
}

// ==== Events =================================================================

/// A file dropped on the window landed in the props folder, by its name there: what a game
/// that joined another sends on to the host.
#[derive(Event, Clone, Debug)]
pub(crate) struct PropFileAdded(pub(crate) String);

// ==== Messages ===============================================================

/// The host's props, each by its name and size.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
struct PropList(Vec<(String, u64)>);

/// A joined game asks for a prop's file by its name.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct FileAsk(String);

/// A piece of a prop's file: its name, where the piece starts, the whole file's size.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct FileChunk {
    name: String,
    offset: u64,
    total: u64,
    bytes: Vec<u8>,
}

/// The channel files go by: reliable and in order, apart from the edits, so a big file never
/// holds an edit up.
struct FileChannel;

// ==== Resources ==============================================================

/// The files being received, by name: the pieces so far, and the size they add up to.
#[derive(Resource, Default)]
struct Incoming(HashMap<String, (u64, Vec<u8>)>);

// ==== Systems ================================================================

/// Tells a game that just joined which props the host has.
fn list_props_to_joiner(
    connected: On<Add<Connected>>,
    mut joiners: Query<&mut MessageSender<PropList>, With<ClientOf>>,
    kind: Option<Res<PropKind>>,
) {
    if let Ok(mut sender) = joiners.get_mut(connected.entity) {
        sender.send::<FileChannel>(PropList(props_on_disk(kind.as_deref())));
    }
}

/// Reads the props folder every `LIST_EVERY_SECS` and tells every game that joined of a change.
fn list_props_when_they_change(
    time: Res<Time<Real>>,
    mut since_secs: Local<f32>,
    mut last: Local<Vec<(String, u64)>>,
    kind: Option<Res<PropKind>>,
    server: Single<&Endpoint, With<HostServer>>,
    mut sender: ServerMultiMessageSender,
) {
    *since_secs += time.delta_secs();
    if *since_secs < LIST_EVERY_SECS {
        return;
    }
    *since_secs = 0.0;
    let now = props_on_disk(kind.as_deref());
    if now != *last {
        let _ = sender.send::<_, FileChannel>(&PropList(now.clone()), *server, &NetworkTarget::All);
        *last = now;
    }
}

/// Asks the host for every prop it lists that this game lacks, or has at another size.
fn ask_for_missing_props(
    mut receivers: Query<&mut MessageReceiver<PropList>, With<HostLink>>,
    mut senders: Query<&mut MessageSender<FileAsk>, (With<HostLink>, With<Connected>)>,
    kind: Option<Res<PropKind>>,
) {
    let have = props_on_disk(kind.as_deref());
    for mut receiver in &mut receivers {
        for list in receiver.receive() {
            for (name, size) in missing(&list.0, &have) {
                info!("asking the host for {name} ({size} bytes)");
                for mut sender in &mut senders {
                    sender.send::<FileChannel>(FileAsk(name.clone()));
                }
            }
        }
    }
}

/// Sends each game the files it asks for, in pieces.
fn send_asked_files(
    mut joiners: Query<
        (&mut MessageReceiver<FileAsk>, &mut MessageSender<FileChunk>),
        With<ClientOf>,
    >,
    kind: Option<Res<PropKind>>,
) {
    let extensions = kind.as_deref().map_or(&[][..], |kind| kind.extensions);
    for (mut receiver, mut sender) in &mut joiners {
        for FileAsk(name) in receiver.receive() {
            let Some(name) = safe_name(&name, extensions) else {
                continue;
            };
            for chunk in chunks_of(&name) {
                sender.send::<FileChannel>(chunk);
            }
        }
    }
}

/// Sends a file dropped into this joined game's editor to the host, which lists it to the others.
fn upload_dropped_file(
    added: On<PropFileAdded>,
    network: Res<Network>,
    mut senders: Query<&mut MessageSender<FileChunk>, (With<HostLink>, With<Connected>)>,
) {
    if *network != Network::Joined {
        return;
    }
    for mut sender in &mut senders {
        for chunk in chunks_of(&added.0) {
            sender.send::<FileChannel>(chunk);
        }
    }
}

/// Writes the files the host sends, each once whole.
fn receive_downloads(
    mut commands: Commands,
    mut receivers: Query<&mut MessageReceiver<FileChunk>, With<HostLink>>,
    mut incoming: ResMut<Incoming>,
    kind: Option<Res<PropKind>>,
    session: Res<Session>,
    screen: Res<State<Screen>>,
    props: Query<&Prop>,
) {
    let extensions = kind.as_deref().map_or(&[][..], |kind| kind.extensions);
    for mut receiver in &mut receivers {
        for chunk in receiver.receive() {
            let Some(name) = take_chunk(&mut incoming, chunk, extensions) else {
                continue;
            };
            // the props the map places with this file were skipped as gameplay began: they
            // come in now that it is here
            if *screen.get() != Screen::Gameplay {
                continue;
            }
            let file = format!("{PROPS_DIR_NAME}/{name}");
            let placed = session.game.map.iter().flat_map(|map| &map.props);
            for waiting in placed.filter(|placed| placed.file == file) {
                if !props.iter().any(|prop| prop.0 == waiting.id) {
                    commands.spawn(prop_bundle(waiting));
                }
            }
        }
    }
}

/// Writes the files the games that joined send, each once whole.
fn receive_uploads(
    mut receivers: Query<&mut MessageReceiver<FileChunk>, With<ClientOf>>,
    mut incoming: ResMut<Incoming>,
    kind: Option<Res<PropKind>>,
) {
    let extensions = kind.as_deref().map_or(&[][..], |kind| kind.extensions);
    for mut receiver in &mut receivers {
        for chunk in receiver.receive() {
            take_chunk(&mut incoming, chunk, extensions);
        }
    }
}

/// Sends a file edit this joined game's player asks to the host, which makes it and sends back
/// what it made.
fn send_file_edit_to_host(
    edit: On<PropFileEdit>,
    network: Res<Network>,
    mut senders: Query<&mut MessageSender<PropFileEdit>, (With<HostLink>, With<Connected>)>,
) {
    if *network != Network::Joined {
        return;
    }
    for mut sender in &mut senders {
        sender.send::<FileChannel>(edit.clone());
    }
}

/// Makes the file edits the games that joined ask, as if this game's player had asked them:
/// `apply_prop_file_edit` refuses any path that leaves the props folder.
fn make_joiner_file_edits(
    mut commands: Commands,
    mut receivers: Query<&mut MessageReceiver<PropFileEdit>, With<ClientOf>>,
) {
    for mut receiver in &mut receivers {
        for edit in receiver.receive() {
            commands.trigger(edit);
        }
    }
}

/// Sends every file change the host makes to every game that joined, the one that asked
/// included, on the files' channel, so it never overtakes a file still coming.
fn send_file_change_to_joiners(
    change: On<PropFileChange>,
    network: Res<Network>,
    server: Option<Single<&Endpoint, With<HostServer>>>,
    mut sender: ServerMultiMessageSender,
) {
    let (Network::Hosting, Some(server)) = (*network, server) else {
        return;
    };
    let _ = sender.send::<_, FileChannel>(&*change, *server, &NetworkTarget::All);
}

/// Makes the file changes the host sends on this game's disk, refusing any path that leaves
/// the props folder: the host is a friend, but a path is still a door into the PC.
fn hear_file_changes(
    mut commands: Commands,
    mut receivers: Query<&mut MessageReceiver<PropFileChange>, With<HostLink>>,
) {
    for mut receiver in &mut receivers {
        for change in receiver.receive() {
            if is_safe_change(&change) {
                commands.trigger(change);
            } else {
                warn!("refused a change to the props from the host: {change:?}");
            }
        }
    }
}

// ==== Helpers ================================================================

/// Returns the props the genre takes in the props folder and every folder in it, by their
/// path inside it (`trees/pine.glb`) and size, sorted.
fn props_on_disk(kind: Option<&PropKind>) -> Vec<(String, u64)> {
    let extensions = kind.map_or(&[][..], |kind| kind.extensions);
    let (entries, _) = read_props(extensions);
    let mut props: Vec<(String, u64)> = entries
        .into_iter()
        .map(|entry| (inside_props(&entry.file).to_string(), entry.size))
        .collect();
    props.sort();
    props
}

/// Returns the props in `listed` that `have` lacks, or holds at another size.
fn missing(listed: &[(String, u64)], have: &[(String, u64)]) -> Vec<(String, u64)> {
    listed
        .iter()
        .filter(|entry| !have.contains(entry))
        .cloned()
        .collect()
}

/// Returns `name` when it is the path of a prop the genre takes inside the props folder: no way
/// out of it (`is_safe_path`), a prop's extension.
fn safe_name(name: &str, extensions: &[&str]) -> Option<String> {
    (is_safe_path(name) && is_prop_file(name, extensions)).then(|| name.to_string())
}

/// Tells whether every path in a change from the host is inside the props folder.
fn is_safe_change(change: &PropFileChange) -> bool {
    match change {
        PropFileChange::FolderMade { path } | PropFileChange::Deleted { path } => {
            is_safe_path(path)
        }
        PropFileChange::Renamed { from, to } => is_safe_path(from) && is_safe_path(to),
    }
}

/// Returns a prop's file in pieces, or none when it cannot be read or is too big to send.
fn chunks_of(name: &str) -> Vec<FileChunk> {
    let Ok(bytes) = std::fs::read(props_dir().join(name)) else {
        warn!("could not read the prop {name} to send it");
        return Vec::new();
    };
    let total = bytes.len() as u64;
    if total > MAX_FILE_BYTES {
        warn!("not sending {name}: {total} bytes is more than a prop may be");
        return Vec::new();
    }
    if bytes.is_empty() {
        return vec![FileChunk {
            name: name.to_string(),
            offset: 0,
            total: 0,
            bytes: Vec::new(),
        }];
    }
    bytes
        .chunks(CHUNK_BYTES)
        .enumerate()
        .map(|(index, piece)| FileChunk {
            name: name.to_string(),
            offset: (index * CHUNK_BYTES) as u64,
            total,
            bytes: piece.to_vec(),
        })
        .collect()
}

/// Adds a piece to the file it belongs to, refusing a name or a size that is not a prop's, and
/// writes the file into the props folder once it is whole, returning its name then.
fn take_chunk(incoming: &mut Incoming, chunk: FileChunk, extensions: &[&str]) -> Option<String> {
    let Some(name) = safe_name(&chunk.name, extensions) else {
        warn!("refused a file named {:?}: not a prop's name", chunk.name);
        return None;
    };
    if chunk.total > MAX_FILE_BYTES {
        warn!(
            "refused {name}: {} bytes is more than a prop may be",
            chunk.total
        );
        return None;
    }
    let (total, bytes) = incoming
        .0
        .entry(name.clone())
        .or_insert_with(|| (chunk.total, Vec::new()));
    // a piece out of place means the file changed under its sender: start it over
    if chunk.offset != bytes.len() as u64 || chunk.total != *total {
        *total = chunk.total;
        bytes.clear();
        if chunk.offset != 0 {
            return None;
        }
    }
    bytes.extend_from_slice(&chunk.bytes);
    if bytes.len() as u64 >= *total {
        let whole = std::mem::take(bytes);
        incoming.0.remove(&name);
        match write_atomic(&props_dir().join(&name), whole) {
            Ok(()) => {
                info!("received the prop {name}");
                return Some(name);
            }
            Err(err) => error!("could not write the prop {name}: {err}"),
        }
    }
    None
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EXTENSIONS: &[&str] = &["glb"];

    #[test]
    fn only_a_prop_path_inside_the_props_folder_is_taken() {
        assert_eq!(
            safe_name("pine.glb", EXTENSIONS),
            Some("pine.glb".to_string())
        );
        assert_eq!(safe_name("../pine.glb", EXTENSIONS), None);
        assert_eq!(
            safe_name("trees/pine.glb", EXTENSIONS),
            Some("trees/pine.glb".to_string())
        );
        assert_eq!(safe_name("trees/../../pine.glb", EXTENSIONS), None);
        assert_eq!(safe_name("dir\\pine.glb", EXTENSIONS), None);
        assert_eq!(safe_name("C:pine.glb", EXTENSIONS), None);
        assert_eq!(safe_name(".glb", EXTENSIONS), None);
        assert_eq!(safe_name("pine.exe", EXTENSIONS), None);
        assert_eq!(safe_name("", EXTENSIONS), None);
    }

    #[test]
    fn a_prop_is_missing_when_absent_or_of_another_size() {
        let listed = vec![("a.glb".to_string(), 10), ("b.glb".to_string(), 20)];
        let have = vec![("a.glb".to_string(), 10), ("b.glb".to_string(), 5)];
        assert_eq!(missing(&listed, &have), vec![("b.glb".to_string(), 20)]);
    }

    #[test]
    fn a_file_bigger_than_a_prop_is_refused() {
        let mut incoming = Incoming::default();
        take_chunk(
            &mut incoming,
            FileChunk {
                name: "huge.glb".to_string(),
                offset: 0,
                total: MAX_FILE_BYTES + 1,
                bytes: vec![0; 4],
            },
            EXTENSIONS,
        );
        assert!(incoming.0.is_empty());
    }
}
