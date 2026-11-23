use bevy::audio::Volume;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};

use super::LoadingDestination;
use super::{Screen, StartGame};
use crate::GAME_TITLE;
use crate::fade::approach;
use crate::i18n::Localized;
use crate::multiplayer::FindServerButton; // multiplayer
use crate::overlays::Overlay;
use crate::save::{Session, loaded_session, newest_ready, saves_on_disk};
use crate::ui::blur::{FrozenBackground, unfreeze_background};
use crate::ui::{
    ACCENT, BODY_FONT_SIZE_PX, BUTTON_EASE_SECS, INK, MUTED_SHADOW_COLOR, SMALL_FONT_SIZE_PX, TEXT,
    TITLE_FONT_SIZE_PX, font, muted_text,
};
use veil::{MenuTransition, VEIL_Z_INDEX, Veil, animate_veils, veil, veil_alpha};

mod veil;

// ==== Config =================================================================

// ---- scrim: dark behind the words, clear over the artwork -------------------
const SCRIM_ALPHA: f32 = 0.85;
const SCRIM_STOP_PERCENT: f32 = 65.0;

// ---- the game's name, over the column of words ------------------------------
const TITLE_LEFT_PX: f32 = 80.0;
// room under the title, so the name reads apart from the choices
const TITLE_GAP_PX: f32 = 28.0;
const COLUMN_WIDTH_PX: f32 = 300.0;

// ---- words: the menu's choices, each on a bar that shows under the pointer --
const WORD_HEIGHT_PX: f32 = 44.0;
// between the left edge of a word's bar and the word
const WORD_PADDING_PX: f32 = 20.0;
// greyish at rest, so the word under the pointer is the one that stands out
const WORD_REST_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.62);
// a bar is solid this far across, then fades out to its right edge
const BAR_SOLID_PERCENT: f32 = 30.0;

// ---- derived ----------------------------------------------------------------
// the column starts left of the title by a word's padding, so the words themselves line
// up under the title, and their bars reach out past it
const COLUMN_LEFT_PX: f32 = TITLE_LEFT_PX - WORD_PADDING_PX;

// ---- version, in the corner -------------------------------------------------
const VERSION_MARGIN_PX: f32 = 20.0;
const VERSION_TEXT: &str = concat!("v", env!("CARGO_PKG_VERSION"));

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        OnEnter(Screen::Menu),
        (
            (spawn_menu_screen, update_continue_word).chain(),
            play_menu_music,
        ),
    );
    // a delete in the Load Game list may have taken the last save Continue could load
    app.add_systems(
        OnExit(Overlay::LoadGame),
        update_continue_word.run_if(in_state(Screen::Menu)),
    );
    app.add_observer(activate_menu_word.run_if(in_state(Screen::Menu)));
    app.add_observer(start_game_from_menu.run_if(in_state(Screen::Menu)));
    app.add_systems(
        Update,
        (
            paint_menu_words,
            animate_veils,
            // read the veil after this frame's tick, so the sound never lags the picture
            update_menu_music_volume.after(animate_veils),
            update_frozen_background.after(animate_veils),
        )
            .run_if(in_state(Screen::Menu)),
    );
    app.add_systems(OnExit(Screen::Menu), unfreeze_background);
}

// ==== Components =============================================================

/// Names what one of the menu's words does when it is pressed.
#[derive(Component)]
enum MenuAction {
    NewGame,
    Continue,
    LoadGame,
    Settings,
    Credits,
    Quit,
}

/// Marks one of the menu's words, with how far it has eased toward hovered, 0.0 to 1.0.
#[derive(Component, Default)]
struct MenuWord {
    hover: f32,
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
                    row_gap: px(TITLE_GAP_PX),
                    padding: UiRect::left(px(COLUMN_LEFT_PX)),
                    ..default()
                },
                children![
                    (
                        Text::new(GAME_TITLE),
                        font(TITLE_FONT_SIZE_PX),
                        TextColor(TEXT),
                        Node {
                            margin: UiRect::left(px(WORD_PADDING_PX)),
                            ..default()
                        },
                    ),
                    // no gap between the words: the pointer going down the column is always
                    // over one of them, so the bar passes from word to word and never blinks
                    (
                        Node {
                            width: px(COLUMN_WIDTH_PX),
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        children![
                            (menu_word("menu-new-game"), MenuAction::NewGame),
                            (menu_word("menu-continue"), MenuAction::Continue),
                            (menu_word("menu-load-game"), MenuAction::LoadGame),
                            (menu_word("menu-find-server"), FindServerButton), // multiplayer
                            (menu_word("menu-settings"), MenuAction::Settings),
                            (menu_word("menu-credits"), MenuAction::Credits),
                            (menu_word("menu-quit"), MenuAction::Quit),
                        ],
                    ),
                ],
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    right: px(VERSION_MARGIN_PX),
                    bottom: px(VERSION_MARGIN_PX),
                    ..default()
                },
                Text::new(VERSION_TEXT),
                muted_text(SMALL_FONT_SIZE_PX),
            ),
            veil(MenuTransition::Opening),
        ],
        DespawnOnExit(Screen::Menu),
    ));
}

/// Shows Continue when a save can be loaded and hides it otherwise. `Display::None` takes
/// it out of the column, so the words below close the gap instead of leaving a hole.
fn update_continue_word(mut words: Query<(&MenuAction, &mut Node)>) {
    let saves = saves_on_disk();
    let has_save = newest_ready(&saves).is_some();
    for (action, mut node) in &mut words {
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

/// Runs the action of the menu's word that was pressed.
fn activate_menu_word(
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
                let saves = saves_on_disk();
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
fn start_game_from_menu(start_game: On<StartGame>, mut commands: Commands) {
    commands.insert_resource(start_game.session.clone());
    commands.insert_resource(start_game.destination);
    commands.spawn((
        veil(MenuTransition::Closing),
        DespawnOnExit(Screen::Menu),
        VEIL_Z_INDEX,
    ));
}

/// Eases each of the menu's words toward hovered and paints it from there: the bar behind
/// it, the word and the word's shadow. Counts real time, as every button does.
fn paint_menu_words(
    time: Res<Time<Real>>,
    mut words: Query<(&Hovered, &mut MenuWord, &mut BackgroundGradient, &Children)>,
    mut labels: Query<(&mut TextColor, &mut TextShadow)>,
) {
    let step = time.delta_secs() / BUTTON_EASE_SECS;
    for (hovered, mut word, mut bar, children) in &mut words {
        let hover_target = f32::from(u8::from(hovered.get()));
        // as `paint_buttons` does: a word resting where it was last painted is left alone,
        // and a new one is painted once, since its label spawns as plain muted text
        if !word.is_added() && word.hover == hover_target {
            continue;
        }
        word.hover = approach(word.hover, hover_target, step);
        let paint = menu_word_paint(word.hover);
        *bar = bar_gradient(paint.bar);
        for child in children {
            if let Ok((mut color, mut shadow)) = labels.get_mut(*child) {
                color.0 = paint.word;
                shadow.color = paint.shadow;
            }
        }
    }
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

/// Tells the glass whether the menu behind an overlay is still, which it is while no veil
/// fades over it. An overlay opened under the opening veil would otherwise keep, for as long
/// as it stays open, a blur of the menu half covered.
fn update_frozen_background(
    veils: Query<(), With<Veil>>,
    mut frozen_background: ResMut<FrozenBackground>,
) {
    frozen_background.0 = veils.is_empty();
}

// ==== Helpers ================================================================

// ---- the menu's words -------------------------------------------------------

/// Returns one of the menu's choices: a word alone, on a bar that `paint_menu_words` brings
/// in under the pointer.
fn menu_word(key: &'static str) -> impl Bundle {
    (
        Node {
            height: px(WORD_HEIGHT_PX),
            padding: UiRect::horizontal(px(WORD_PADDING_PX)),
            align_items: AlignItems::Center,
            ..default()
        },
        // a `Button` with `Hovered` and no `ButtonKind`: it is pressed and it sounds like any
        // button, while `paint_buttons` leaves its paint to this file
        Button,
        Hovered::default(),
        MenuWord::default(),
        bar_gradient(menu_word_paint(0.0).bar),
        // `muted_text` for the shadow it brings; `paint_menu_words` gives the word its own
        // color the first time it sees it
        children![(Localized(key), muted_text(BODY_FONT_SIZE_PX))],
    )
}

/// Returns the bar behind a word: `color` from its left edge to `BAR_SOLID_PERCENT` of the
/// way across, then fading out to its right edge.
fn bar_gradient(color: Color) -> BackgroundGradient {
    BackgroundGradient::from(LinearGradient::to_right(vec![
        ColorStop::new(color, percent(0)),
        ColorStop::new(color, percent(BAR_SOLID_PERCENT)),
        // the same color let through entirely, not `Color::NONE`: fading toward a clear
        // black would pass through grey
        ColorStop::new(color.with_alpha(0.0), percent(100)),
    ]))
}

/// The colors one of the menu's words is painted in at one moment.
#[derive(Debug, PartialEq)]
struct MenuWordPaint {
    bar: Color,
    word: Color,
    shadow: Color,
}

/// Returns the paint of a menu word `hover` of the way (0.0 to 1.0) toward hovered: no bar
/// under a greyish word at rest, the accent under a dark word when hovered.
fn menu_word_paint(hover: f32) -> MenuWordPaint {
    MenuWordPaint {
        bar: ACCENT.with_alpha(hover),
        word: WORD_REST_COLOR.mix(&INK, hover),
        // a dark word would smear over its own shadow, so the shadow leaves as it darkens
        shadow: MUTED_SHADOW_COLOR.with_alpha(MUTED_SHADOW_COLOR.alpha() * (1.0 - hover)),
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    /// Returns whether two colors match channel by channel, within the tolerance.
    fn same_color(a: Color, b: Color) -> bool {
        let (a, b) = (a.to_srgba(), b.to_srgba());
        [
            a.red - b.red,
            a.green - b.green,
            a.blue - b.blue,
            a.alpha - b.alpha,
        ]
        .iter()
        .all(|difference| difference.abs() < TOLERANCE)
    }

    // ---- menu_word_paint ----------------------------------------------------

    #[test]
    fn word_at_rest_is_greyish_over_no_bar() {
        let paint = menu_word_paint(0.0);
        assert!(paint.bar.alpha() < TOLERANCE);
        assert!(same_color(paint.word, WORD_REST_COLOR));
    }

    #[test]
    fn hovered_word_is_dark_over_a_full_bar() {
        let paint = menu_word_paint(1.0);
        assert!(same_color(paint.bar, ACCENT));
        assert!(same_color(paint.word, INK));
    }

    #[test]
    fn word_loses_its_shadow_as_it_darkens() {
        assert!(same_color(menu_word_paint(0.0).shadow, MUTED_SHADOW_COLOR));
        assert!(menu_word_paint(1.0).shadow.alpha() < TOLERANCE);
    }
}
