use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui_widgets::{Activate, TextInput};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

use super::join::join;
use super::{DISCOVERY_PORT, DISCOVERY_STALE_SECS, Heard, Network, read_discovery, typed_address};
use crate::overlays::{Overlay, close_footer, overlay_card, overlay_root};
use crate::ui::{
    ACCENT, BODY_FONT_SIZE_PX, BORDER, BORDER_PX, BUTTON_GAP_PX, BUTTON_HEIGHT_PX, ButtonKind,
    FIELD_FILL, TEXT, button, font, heading, label, row_button,
};

// ==== Config =================================================================

// ---- the IP field -----------------------------------------------------------
const IP_FIELD_PADDING_X_PX: f32 = 16.0;
const IP_FIELD_PADDING_Y_PX: f32 = 6.0;
// an IPv4 address and its port, with room to spare
const IP_FIELD_MAX_CHARS: usize = 32;

// ==== Plugin =================================================================

/// Find Server, from the main menu: the games open on the network, heard by themselves and
/// listed a row each with their address and players, a row to join; Refresh empties the list
/// to hear it again, and a field takes the address of a game that does not show.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(press_find_server);
    app.add_systems(
        OnEnter(Overlay::FindServer),
        (spawn_find_server_overlay, open_listener),
    );
    app.add_systems(
        Update,
        (listen_for_games, fill_game_list)
            .chain()
            .run_if(in_state(Overlay::FindServer)),
    );
    app.add_systems(OnExit(Overlay::FindServer), close_listener);
    app.add_observer(press_refresh.run_if(in_state(Overlay::FindServer)));
    app.add_observer(press_join.run_if(in_state(Overlay::FindServer)));
}

// ==== Components =============================================================

/// Marks the main menu's Find Server.
#[derive(Component)]
pub(crate) struct FindServerButton;

/// Marks the node the games heard are listed in.
#[derive(Component)]
struct GameList;

/// Marks the Refresh button.
#[derive(Component)]
struct RefreshButton;

/// Marks the field an address is typed in.
#[derive(Component)]
struct IpField;

/// What a Join button joins: a game heard, or the address in the field.
#[derive(Component)]
enum JoinTarget {
    Heard(SocketAddr),
    Typed,
}

// ==== Resources ==============================================================

/// The socket Find Server listens on while it is open.
#[derive(Resource)]
struct Listener(UdpSocket);

/// The games heard, each with when it was last heard; a list redrawn only when it changes.
#[derive(Resource, Default, PartialEq)]
struct GamesHeard(Vec<GameHeard>);

/// A game heard on the network: where to join it, how many play of how many, when it was heard.
#[derive(Clone, PartialEq, Debug)]
struct GameHeard {
    address: SocketAddr,
    heard: Heard,
    heard_secs: f32,
}

// ==== Systems ================================================================

/// Opens Find Server when the main menu's button is pressed.
fn press_find_server(
    activate: On<Activate>,
    buttons: Query<(), With<FindServerButton>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
) {
    if buttons.contains(activate.entity) {
        next_overlay.set(Overlay::FindServer);
    }
}

/// Spawns Find Server: its title, Refresh, the list, the address field and its Join, Close.
fn spawn_find_server_overlay(mut commands: Commands) {
    commands.spawn((
        overlay_root(),
        DespawnOnExit(Overlay::FindServer),
        children![(
            overlay_card(),
            children![
                heading("find-server-title"),
                (
                    Node {
                        justify_content: JustifyContent::End,
                        ..default()
                    },
                    children![(
                        button("find-server-refresh", ButtonKind::Quiet),
                        RefreshButton
                    )],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(BUTTON_GAP_PX),
                        ..default()
                    },
                    GameList,
                ),
                label("find-server-by-ip"),
                (
                    Node {
                        column_gap: px(BUTTON_GAP_PX),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        ip_field(),
                        (
                            button("find-server-join", ButtonKind::Primary),
                            JoinTarget::Typed
                        ),
                    ],
                ),
                close_footer(),
            ],
        )],
    ));
}

/// Listens for the games' broadcasts while Find Server is open. A second game on this PC cannot
/// listen on the same port: it can still join by address.
fn open_listener(mut commands: Commands) {
    commands.init_resource::<GamesHeard>();
    match UdpSocket::bind((Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT)).and_then(|socket| {
        socket.set_nonblocking(true)?;
        Ok(socket)
    }) {
        Ok(socket) => commands.insert_resource(Listener(socket)),
        Err(err) => warn!("cannot listen for games on this PC, join by IP instead: {err}"),
    }
}

/// Adds each game heard to the list, or freshens it, and drops each not heard for
/// `DISCOVERY_STALE_SECS`.
fn listen_for_games(
    listener: Option<Res<Listener>>,
    time: Res<Time<Real>>,
    mut games: ResMut<GamesHeard>,
) {
    let now = time.elapsed_secs();
    let mut heard_now = games.0.clone();
    if let Some(listener) = listener {
        let mut buffer = [0_u8; 256];
        while let Ok((length, from)) = listener.0.recv_from(&mut buffer) {
            let Some(heard) = buffer
                .get(..length)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .and_then(read_discovery)
            else {
                continue;
            };
            let address = SocketAddr::new(from.ip(), heard.port);
            heard_now = with_game_heard(heard_now, address, heard, now);
        }
    }
    heard_now.retain(|game| now - game.heard_secs < DISCOVERY_STALE_SECS);
    // the time heard changes every second: compare what the list shows, not when
    if shown(&heard_now) != shown(&games.0) || heard_now.len() != games.0.len() {
        games.0 = heard_now;
    } else {
        games.bypass_change_detection().0 = heard_now;
    }
}

/// Lists the games heard, a Join row each, or says none was heard, whenever the list changes.
fn fill_game_list(
    mut commands: Commands,
    games: Res<GamesHeard>,
    lists: Query<(Entity, Ref<GameList>)>,
) {
    let Ok((list, added)) = lists.single() else {
        return;
    };
    if !games.is_changed() && !added.is_added() {
        return;
    }
    commands.entity(list).despawn_children();
    if games.0.is_empty() {
        commands.spawn((label("find-server-empty"), ChildOf(list)));
    }
    for game in &games.0 {
        commands.spawn((
            row_button((
                Text::new(game_row_text(game)),
                font(BODY_FONT_SIZE_PX),
                TextColor(TEXT),
            )),
            JoinTarget::Heard(game.address),
            ChildOf(list),
        ));
    }
}

/// Empties the list on Refresh, to hear every game again within a second.
fn press_refresh(
    activate: On<Activate>,
    buttons: Query<(), With<RefreshButton>>,
    mut games: ResMut<GamesHeard>,
) {
    if buttons.contains(activate.entity) {
        games.0.clear();
    }
}

/// Joins the game of the row pressed, or the address typed, closing Find Server; an address
/// that is not one does nothing.
fn press_join(
    activate: On<Activate>,
    mut commands: Commands,
    targets: Query<&JoinTarget>,
    fields: Query<&EditableText, With<IpField>>,
    mut network: ResMut<Network>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    time: Res<Time<Real>>,
) {
    let Ok(target) = targets.get(activate.entity) else {
        return;
    };
    if *network != Network::Offline {
        return;
    }
    let address = match target {
        JoinTarget::Heard(address) => Some(*address),
        JoinTarget::Typed => fields
            .single()
            .ok()
            .and_then(|field| typed_address(&field.value().to_string())),
    };
    let Some(address) = address else {
        return;
    };
    join(&mut commands, &mut network, time.elapsed_secs(), address);
    next_overlay.set(Overlay::None);
}

/// Stops listening as Find Server closes.
fn close_listener(mut commands: Commands) {
    commands.remove_resource::<Listener>();
    commands.remove_resource::<GamesHeard>();
}

// ==== Helpers ================================================================

/// Returns the address field: typed in, with the cursor in the accent.
fn ip_field() -> impl Bundle {
    (
        EditableText {
            max_characters: Some(IP_FIELD_MAX_CHARS),
            ..default()
        },
        // `EditableText` only holds the text, this is what types into it
        TextInput,
        // a click only hands the keyboard to an entity with a `TabIndex`
        TabIndex::default(),
        IpField,
        TextCursorStyle {
            color: ACCENT,
            ..default()
        },
        font(BODY_FONT_SIZE_PX),
        TextColor(TEXT),
        Node {
            flex_grow: 1.0,
            min_height: px(BUTTON_HEIGHT_PX),
            padding: UiRect::axes(px(IP_FIELD_PADDING_X_PX), px(IP_FIELD_PADDING_Y_PX)),
            border: UiRect::all(px(BORDER_PX)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(FIELD_FILL),
        BorderColor::all(BORDER),
    )
}

/// Returns `games` with the game at `address` heard `now`: freshened, or added at the end.
fn with_game_heard(
    mut games: Vec<GameHeard>,
    address: SocketAddr,
    heard: Heard,
    now: f32,
) -> Vec<GameHeard> {
    match games.iter_mut().find(|game| game.address == address) {
        Some(game) => {
            game.heard = heard;
            game.heard_secs = now;
        }
        None => games.push(GameHeard {
            address,
            heard,
            heard_secs: now,
        }),
    }
    games
}

/// Returns what the list shows of `games`: their rows, without when each was heard.
fn shown(games: &[GameHeard]) -> Vec<String> {
    games.iter().map(game_row_text).collect()
}

/// Returns a game's row: its address, then how many play of how many. No words to translate,
/// only an address and numbers.
fn game_row_text(game: &GameHeard) -> String {
    format!(
        "{}    {}/{}",
        game.address.ip(),
        game.heard.players,
        game.heard.max
    )
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn heard(players: u32) -> Heard {
        Heard {
            port: 5888,
            players,
            max: 10,
        }
    }

    #[test]
    fn a_game_heard_again_is_freshened_not_listed_twice() {
        let address = SocketAddr::from(([192, 168, 0, 5], 5888));
        let games = with_game_heard(Vec::new(), address, heard(1), 1.0);
        let games = with_game_heard(games, address, heard(2), 2.0);
        assert_eq!(games.len(), 1);
        assert_eq!(games.first().map(|game| game.heard.players), Some(2));
    }

    #[test]
    fn a_row_shows_the_address_and_the_players() {
        let game = GameHeard {
            address: SocketAddr::from(([26, 1, 2, 3], 5888)),
            heard: heard(2),
            heard_secs: 0.0,
        };
        assert_eq!(game_row_text(&game), "26.1.2.3    2/10");
    }
}
