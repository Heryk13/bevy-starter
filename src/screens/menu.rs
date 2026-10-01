use bevy::audio::Volume;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use super::loading::LoadingDestination;
use super::{Screen, StartGame};
use crate::overlays::Overlay;
use crate::save::{Session, list_saves, loaded_session, newest_ready, saves_dir};
use crate::ui::button;
use veil::{MenuTransition, VEIL_Z_INDEX, Veil, animate_veils, veil, veil_alpha};

mod veil;

// ==== Config =================================================================

// ---- scrim: dark behind the buttons, clear over the artwork -----------------
const SCRIM_ALPHA: f32 = 0.8;
const SCRIM_STOP_PERCENT: f32 = 60.0;

// ---- button column ----------------------------------------------------------
const BUTTON_GAP_PX: f32 = 12.0;
const COLUMN_PADDING_LEFT_PX: f32 = 32.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        OnEnter(Screen::Menu),
        (
            (spawn_menu_screen, show_continue_button).chain(),
            play_menu_music,
        ),
    );
    app.add_observer(activate_menu_button);
    app.add_observer(start_game_from_menu);
    app.add_systems(
        Update,
        (
            animate_veils,
            // read the veil after this frame's tick, so the sound never lags the picture
            update_menu_music_volume.after(animate_veils),
        )
            .run_if(in_state(Screen::Menu)),
    );
}

// ==== Components =============================================================

/// Names what a menu button does when it is pressed.
#[derive(Component)]
enum MenuAction {
    NewGame,
    Continue,
    LoadGame,
    Settings,
    Credits,
    Quit,
}

/// Marks the menu music, so its volume can be found and faded.
#[derive(Component)]
struct MenuMusic;

// ==== Systems ================================================================

/// Spawns the menu screen and its layers, from the background up.
fn spawn_menu_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        children![
            // ponytail: stretch distorts art that is not at the window's aspect ratio;
            // a real "cover" needs a clipping parent plus min-width and min-height on the
            // image, not worth writing before the final art exists
            (
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                ImageNode::new(asset_server.load("menu/background.jpg"))
                    .with_mode(NodeImageMode::Stretch),
            ),
            (
                Node {
                    width: percent(100),
                    height: percent(100),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundGradient::from(LinearGradient::to_right(vec![
                    ColorStop::new(Color::BLACK.with_alpha(SCRIM_ALPHA), percent(0)),
                    ColorStop::new(Color::NONE, percent(SCRIM_STOP_PERCENT)),
                ])),
            ),
            (
                Node {
                    width: percent(100),
                    height: percent(100),
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    row_gap: px(BUTTON_GAP_PX),
                    padding: UiRect {
                        left: px(COLUMN_PADDING_LEFT_PX),
                        ..default()
                    },
                    ..default()
                },
                children![
                    (button("New Game"), MenuAction::NewGame),
                    (button("Continue"), MenuAction::Continue),
                    (button("Load Game"), MenuAction::LoadGame),
                    (button("Settings"), MenuAction::Settings),
                    (button("Credits"), MenuAction::Credits),
                    (button("Quit"), MenuAction::Quit),
                ],
            ),
            veil(MenuTransition::Opening),
        ],
        DespawnOnExit(Screen::Menu),
    ));
}

/// Hides Continue while no save can be loaded. `Display::None` takes it out of the
/// column, so the buttons below close the gap instead of leaving a hole.
fn show_continue_button(mut buttons: Query<(&MenuAction, &mut Node)>) {
    let saves = saves_dir().map(|dir| list_saves(&dir)).unwrap_or_default();
    let has_save = newest_ready(&saves).is_some();
    for (action, mut node) in &mut buttons {
        if matches!(action, MenuAction::Continue) {
            node.display = if has_save {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

/// Starts the menu music, looping until the menu closes.
fn play_menu_music(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        AudioPlayer::new(asset_server.load("menu/music.ogg")),
        PlaybackSettings::LOOP,
        MenuMusic,
        DespawnOnExit(Screen::Menu),
    ));
}

/// Runs the action of the menu button that was pressed.
fn activate_menu_button(
    activate: On<Activate>,
    mut commands: Commands,
    actions: Query<&MenuAction>,
    mut next_overlay: ResMut<NextState<Overlay>>,
) {
    if let Ok(action) = actions.get(activate.entity) {
        match action {
            MenuAction::NewGame => {
                commands.trigger(StartGame {
                    session: Session::default(),
                    destination: LoadingDestination::Intro,
                });
            }
            MenuAction::Continue => {
                // read again on the click, in case a save changed since the menu opened
                let saves = saves_dir().map(|dir| list_saves(&dir)).unwrap_or_default();
                if let Some(data) = newest_ready(&saves) {
                    commands.trigger(StartGame {
                        session: loaded_session(data),
                        destination: LoadingDestination::Gameplay,
                    });
                }
            }
            MenuAction::LoadGame => {
                next_overlay.set(Overlay::LoadGame);
            }
            MenuAction::Settings => {
                next_overlay.set(Overlay::Settings);
            }
            MenuAction::Credits => {
                next_overlay.set(Overlay::Credits);
            }
            MenuAction::Quit => {
                commands.write_message(AppExit::Success);
            }
        }
    }
}

/// Starts the given game from the menu: keeps it for the screens ahead, then fades
/// out, since the veil is what leaves for the loading screen.
fn start_game_from_menu(
    start_game: On<StartGame>,
    screen: Res<State<Screen>>,
    mut commands: Commands,
) {
    if *screen.get() != Screen::Menu {
        return;
    }
    commands.insert_resource(start_game.session.clone());
    commands.insert_resource(start_game.destination);
    commands.spawn((
        veil(MenuTransition::Closing),
        DespawnOnExit(Screen::Menu),
        VEIL_Z_INDEX,
    ));
}

/// Keeps the menu music at the player's volume, fading it with the veil.
fn update_menu_music_volume(
    mut music: Single<&mut AudioSink, With<MenuMusic>>,
    veil: Option<Single<&Veil>>,
    global_volume: Res<GlobalVolume>,
) {
    let fade = match veil {
        Some(veil) => 1.0 - veil_alpha(veil.timer.elapsed_secs(), veil.transition),
        None => 1.0,
    };
    // set every frame: `GlobalVolume` is only read when a sink starts, so a track that
    // plays for minutes would not follow the settings slider otherwise
    music.set_volume(global_volume.volume * Volume::Linear(fade));
}
