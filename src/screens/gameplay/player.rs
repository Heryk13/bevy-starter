use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde::{Deserialize, Serialize};

#[cfg(feature = "dev")]
use crate::dev_tools::prop_editor::PropEditor;
#[cfg(feature = "dev")]
use crate::dev_tools::speed::{SpeedMultiplier, Speedometer};
use crate::input::{Action, ActionName, Binding, Controls, Page, register_action};
use crate::multiplayer::NetworkedBody; // multiplayer
use crate::save::Session;
use crate::screens::gameplay::playroom::TILE_DISPLAY_PX;
use crate::screens::{Pause, Screen};
use crate::ui::ACCENT;

mod others; // multiplayer

// ==== Config =================================================================

// ---- body: a square above the tiles -----------------------------------------
// in front of the room's tiles, which sit at 0; the 2D prop editor puts props either side of it
pub(crate) const PLAYER_Z: f32 = 1.0;

// ---- walk -------------------------------------------------------------------
// four tiles a second: across the room's floor in a few seconds; per second rather than
// per frame, so it is the same at any frame rate
const PLAYER_WALK_SPEED_PX_PER_S: f32 = 256.0;

// ---- walk actions: what the player rebinds on the Controls page -------------
// up the screen, not forward: the room is seen from above
pub(crate) const MOVE_UP: ActionName = ActionName("move_up");
pub(crate) const MOVE_DOWN: ActionName = ActionName("move_down");
pub(crate) const MOVE_LEFT: ActionName = ActionName("move_left");
pub(crate) const MOVE_RIGHT: ActionName = ActionName("move_right");
// each with the key it starts on and the translation key of its label, in the order the
// page lists them
const WALK_ACTIONS: [(ActionName, KeyCode, &str); 4] = [
    (MOVE_UP, KeyCode::KeyW, "action-move-up"),
    (MOVE_DOWN, KeyCode::KeyS, "action-move-down"),
    (MOVE_LEFT, KeyCode::KeyA, "action-move-left"),
    (MOVE_RIGHT, KeyCode::KeyD, "action-move-right"),
];

// ---- derived from the tiles -------------------------------------------------
// one tile big, so it measures the room as the room measures itself
const PLAYER_SIZE_PX: f32 = TILE_DISPLAY_PX as f32;
// the camera follows once the body comes within a tile of an edge of the screen: the room
// fits the window with a tile to spare, so inside its walls the view stays still
const DEAD_ZONE_MARGIN_PX: f32 = TILE_DISPLAY_PX as f32;

// ==== Plugin =================================================================

/// The 2D player: a square one tile big that walks the room by the four walk actions, the
/// screen camera following it once it nears an edge, and its place kept in the save.
pub(super) fn plugin(app: &mut App) {
    for (name, key, label) in WALK_ACTIONS {
        register_action(
            app,
            Action {
                name,
                default: Binding::Key(key),
                page: Page::Game,
                label: Some(label),
            },
        );
    }
    app.add_systems(OnEnter(Screen::Gameplay), spawn_player);
    app.add_plugins(others::plugin); // multiplayer
    // record_player_state has no order against the autosave, which reads the session it
    // writes: a save a frame behind is a few pixels off. All three only in Pause::Off, as in
    // 3D: a save loaded from the pause menu replaces the session while the screen is still
    // Gameplay, and recording then would write over the position just loaded
    let walk = (walk_player, follow_player.after(walk_player)).run_if(in_state(Pause::Off));
    // the prop editor's free camera takes the walk keys and the camera, a dev tool's, so not
    // in a release build
    #[cfg(feature = "dev")]
    let walk = walk.run_if(not(in_state(PropEditor::On)));
    app.add_systems(
        Update,
        (walk, record_player_state.run_if(in_state(Pause::Off))),
    );
    app.add_systems(OnExit(Screen::Gameplay), bring_camera_home);
}

// ==== Save ===================================================================

/// Where the player stands, the part of `GameData` a save keeps of them.
#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct PlayerState {
    // in the room's pixels, the origin at its middle, where a new game starts
    pub(crate) position: Vec2,
}

// ==== Components =============================================================

/// Marks the player's body, the one entity walking, the camera and the save act on.
#[derive(Component)]
struct Player;

// ==== Systems ================================================================

/// Spawns the player where the session left them, the middle of the room in a new game.
fn spawn_player(mut commands: Commands, session: Res<Session>) {
    let body = (
        Sprite::from_color(ACCENT, Vec2::splat(PLAYER_SIZE_PX)),
        Transform::from_translation(session.game.player.position.extend(PLAYER_Z)),
        Player,
        NetworkedBody, // multiplayer
        DespawnOnExit(Screen::Gameplay),
    );
    // on speed's capsule (F8), a dev tool's, so not in a release build
    #[cfg(feature = "dev")]
    let body = (body, Speedometer { unit: "px/s" });
    commands.spawn(body);
}

/// Walks the player across the screen by the four walk actions, which start on W, S, A and
/// D, counting the game's time, so pause and slow motion hold it. Nothing stops it: there
/// are no collisions yet.
fn walk_player(
    controls: Controls,
    time: Res<Time>,
    mut body: Single<&mut Transform, With<Player>>,
    #[cfg(feature = "dev")] multiplier: Res<SpeedMultiplier>,
) {
    let step = walk_velocity(walk_input(&controls)) * time.delta_secs();
    // speed's capsule (F8) steps it, a dev tool's, so a release build walks at 1×
    #[cfg(feature = "dev")]
    let step = step * multiplier.0;
    body.translation += step.extend(0.0);
}

/// Pushes the screen camera, which draws the room, just enough to keep the body a tile from
/// every edge of the window; while it is further in, the camera stays still.
fn follow_player(
    body: Single<&Transform, With<Player>>,
    mut camera: Single<&mut Transform, (With<IsDefaultUiCamera>, Without<Player>)>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let half_zone = dead_zone_half_size(window.size());
    let wanted = followed(
        camera.translation.truncate(),
        body.translation.truncate(),
        half_zone,
    );
    if wanted != camera.translation.truncate() {
        camera.translation = wanted.extend(camera.translation.z);
    }
}

/// Copies where the player stands into the session every frame, so whatever saves next, the
/// pause menu or the autosave, writes where they are now.
fn record_player_state(body: Single<&Transform, With<Player>>, mut session: ResMut<Session>) {
    session.game.player.position = body.translation.truncate();
}

/// Puts the screen camera back at the origin as gameplay ends, so the menu's screens and the
/// next game start from where they always did.
fn bring_camera_home(mut camera: Single<&mut Transform, With<IsDefaultUiCamera>>) {
    camera.translation = Vec3::ZERO.with_z(camera.translation.z);
}

// ==== Helpers ================================================================

/// Returns what the four walk actions ask for: right and up, one per key held, each against
/// its opposite. Photo mode's free camera pans by the same keys.
pub(crate) fn walk_input(controls: &Controls) -> Vec2 {
    let axis = |positive, negative| {
        f32::from(u8::from(controls.pressed(positive)))
            - f32::from(u8::from(controls.pressed(negative)))
    };
    Vec2::new(axis(MOVE_RIGHT, MOVE_LEFT), axis(MOVE_UP, MOVE_DOWN))
}

/// Returns the velocity the keys ask for, `input` right and up, made one long first so a
/// diagonal is no faster than straight across.
fn walk_velocity(input: Vec2) -> Vec2 {
    input.normalize_or_zero() * PLAYER_WALK_SPEED_PX_PER_S
}

/// Returns half the size of the box the body's middle may move in with the camera still, in
/// a window `window_px` big: the window less the margin and half the body on every side, and
/// nothing on a window too small for it.
fn dead_zone_half_size(window_px: Vec2) -> Vec2 {
    (window_px / 2.0 - Vec2::splat(DEAD_ZONE_MARGIN_PX + PLAYER_SIZE_PX / 2.0)).max(Vec2::ZERO)
}

/// Returns where the camera at `camera` goes for the body at `body` to stay inside the dead
/// zone, `half_zone` either side of the camera: unmoved while it is inside, and on each axis
/// it has left by, pushed just enough to bring it back to the zone's edge.
fn followed(camera: Vec2, body: Vec2, half_zone: Vec2) -> Vec2 {
    let offset = body - camera;
    camera + offset - offset.clamp(-half_zone, half_zone)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::GameData;

    const TOLERANCE: f32 = 1e-4;
    const HALF_ZONE: Vec2 = Vec2::new(500.0, 300.0);

    // ---- walking ------------------------------------------------------------

    #[test]
    fn up_walks_up_the_screen() {
        let velocity = walk_velocity(Vec2::Y);
        assert!(velocity.abs_diff_eq(Vec2::new(0.0, PLAYER_WALK_SPEED_PX_PER_S), TOLERANCE));
    }

    #[test]
    fn a_diagonal_walks_as_fast_as_straight_across() {
        let diagonal = walk_velocity(Vec2::ONE);
        assert!((diagonal.length() - PLAYER_WALK_SPEED_PX_PER_S).abs() < TOLERANCE);
    }

    // ---- the camera ---------------------------------------------------------

    #[test]
    fn the_camera_stays_still_while_the_body_is_inside_the_dead_zone() {
        let camera = followed(Vec2::ZERO, HALF_ZONE / 2.0, HALF_ZONE);
        assert_eq!(camera, Vec2::ZERO);
    }

    #[test]
    fn the_camera_is_pushed_just_enough_once_the_body_leaves_the_zone() {
        let past_right = followed(Vec2::ZERO, Vec2::new(HALF_ZONE.x + 40.0, 0.0), HALF_ZONE);
        assert!(past_right.abs_diff_eq(Vec2::new(40.0, 0.0), TOLERANCE));
        let past_bottom = followed(Vec2::ZERO, Vec2::new(0.0, -HALF_ZONE.y - 10.0), HALF_ZONE);
        assert!(past_bottom.abs_diff_eq(Vec2::new(0.0, -10.0), TOLERANCE));
    }

    #[test]
    fn the_room_fits_the_dead_zone_of_the_starting_window() {
        // the room is 16 × 10 tiles, and a body on its floor stays a tile inside its walls
        let floor_half = Vec2::new(16.0, 10.0) * PLAYER_SIZE_PX / 2.0 - PLAYER_SIZE_PX * 1.5;
        // Bevy's default window, the size the game opens at
        let half_zone = dead_zone_half_size(Vec2::new(1280.0, 720.0));
        assert!(floor_half.x <= half_zone.x && floor_half.y <= half_zone.y);
    }

    // ---- the save -----------------------------------------------------------

    #[test]
    fn a_save_without_a_player_loads_at_the_middle_of_the_room() {
        let read_back: GameData = ron::from_str("()").unwrap();
        assert_eq!(read_back.player.position, Vec2::ZERO);
    }

    #[test]
    fn game_data_reads_back_from_ron_unchanged() {
        let game_data = GameData {
            player: PlayerState {
                position: Vec2::new(12.0, -34.0),
            },
            map: None,
        };
        let read_back: GameData = ron::from_str(&ron::to_string(&game_data).unwrap()).unwrap();
        assert!(
            read_back
                .player
                .position
                .abs_diff_eq(game_data.player.position, TOLERANCE)
        );
    }
}
