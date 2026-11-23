use avian3d::prelude::LinearVelocity;
use bevy::camera::Viewport;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::look::{EYE_OFFSET_M, look_delta_px, turn_view};
use super::{Player, capsule_body, walk_velocity};
use crate::input::GuestPad;
use crate::screens::gameplay::playroom::{PLAYROOM_CAMERA_ORDER, PlayroomCamera, PlayroomView};
use crate::screens::{Pause, Screen};

// ==== Config =================================================================

// ---- players on one PC ------------------------------------------------------
// two: with three or four views the level went dark in all of them on the first try
// (2026-11-24), lit by the sky alone, as if the sun's shadows covered everything; the layouts
// for three and four (`views_of`) are ready for the day that is found
const MAX_LOCAL_PLAYERS: usize = 2;
// the button a gamepad joins with, and leaves with: one no action of the game takes
const JOIN_PAD_BUTTON: GamepadButton = GamepadButton::Select;
// how far apart the players start, each to the right of the one before, so none overlap
const GUEST_SPAWN_OFFSET_M: f32 = 1.2;

// ==== Plugin =================================================================

/// A split screen: while the game runs, Select on a gamepad brings in one more player on this
/// PC, up to `MAX_LOCAL_PLAYERS`, and Select again on it takes that player out. Each one has a
/// body like the first player's, walks and looks around with its gamepad's sticks, and sees
/// through a view of its own, the window cut top and bottom (`views_of`). The others
/// are not saved, nor sent to a shared game: they are the first player's guests, gone as
/// gameplay ends. A game without a split screen deletes this file and every line marked
/// `// split screen`.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            (take_or_give_back_pads, drop_guests_without_pad)
                .chain()
                .run_if(in_state(Pause::Off)),
            (walk_guests, look_around_as_guests).run_if(in_state(Pause::Off)),
            lay_out_views,
        )
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
    app.add_systems(OnExit(Screen::Gameplay), give_back_every_pad);
}

// ==== Components =============================================================

/// Another player on this PC, its body: the gamepad it plays with, the camera it sees
/// through, and its seat, the order it joined in, which orders the views.
#[derive(Component)]
struct Guest {
    pad: Entity,
    camera: Entity,
    seat: u32,
}

/// Marks the camera another player on this PC sees through.
#[derive(Component)]
struct GuestCamera;

// ==== Systems ================================================================

/// Brings in a player for each gamepad whose Select was just pressed, or takes out the player
/// that gamepad already plays.
fn take_or_give_back_pads(
    mut commands: Commands,
    pads: Query<(Entity, &Gamepad, Has<GuestPad>)>,
    guests: Query<(Entity, &Guest)>,
    player: Single<&Transform, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut count = guests.iter().count();
    let mut next_seat = guests
        .iter()
        .map(|(_, guest)| guest.seat)
        .max()
        .unwrap_or(0)
        + 1;
    for (pad, gamepad, taken) in &pads {
        if !gamepad.just_pressed(JOIN_PAD_BUTTON) {
            continue;
        }
        if taken {
            for (body, _) in guests.iter().filter(|(_, guest)| guest.pad == pad) {
                commands.entity(body).despawn();
                count -= 1;
            }
            commands.entity(pad).remove::<GuestPad>();
            info!("a player left the split screen");
            continue;
        }
        if count + 1 >= MAX_LOCAL_PLAYERS {
            continue;
        }
        let at = player.translation + player.right() * GUEST_SPAWN_OFFSET_M * next_seat as f32;
        let body = commands
            .spawn((
                capsule_body(&mut meshes, &mut materials),
                Transform::from_translation(at).with_rotation(player.rotation),
                DespawnOnExit(Screen::Gameplay),
            ))
            .id();
        let camera = commands
            .spawn((
                PlayroomView,
                GuestCamera,
                // below the first player's, one each, so no two views draw in the same order
                Camera {
                    order: PLAYROOM_CAMERA_ORDER - next_seat as isize,
                    ..default()
                },
                Transform::from_xyz(0.0, EYE_OFFSET_M, 0.0),
                ChildOf(body),
            ))
            .id();
        commands.entity(body).insert(Guest {
            pad,
            camera,
            seat: next_seat,
        });
        commands.entity(pad).insert(GuestPad);
        next_seat += 1;
        count += 1;
        info!("a player joined the split screen");
    }
}

/// Takes out the players whose gamepad was unplugged.
fn drop_guests_without_pad(
    mut commands: Commands,
    guests: Query<(Entity, &Guest)>,
    pads: Query<(), With<Gamepad>>,
) {
    for (body, guest) in &guests {
        if !pads.contains(guest.pad) {
            commands.entity(body).despawn();
        }
    }
}

/// Walks each other player by its gamepad's left stick, as the first player walks.
fn walk_guests(
    mut guests: Query<(&Guest, &Transform, &mut LinearVelocity)>,
    pads: Query<&Gamepad>,
) {
    for (guest, transform, mut velocity) in &mut guests {
        let stick = pads.get(guest.pad).map_or(Vec2::ZERO, Gamepad::left_stick);
        let walk = walk_velocity(stick, transform.rotation);
        velocity.x = walk.x;
        velocity.z = walk.z;
    }
}

/// Turns each other player's view by its gamepad's right stick, as the first player's turns.
fn look_around_as_guests(
    time: Res<Time>,
    mut guests: Query<(&Guest, &mut Transform)>,
    mut cameras: Query<&mut Transform, (With<GuestCamera>, Without<Guest>)>,
    pads: Query<&Gamepad>,
) {
    for (guest, mut body) in &mut guests {
        let Ok(mut camera) = cameras.get_mut(guest.camera) else {
            continue;
        };
        let stick = pads.get(guest.pad).map_or(Vec2::ZERO, Gamepad::right_stick);
        let delta = look_delta_px(Vec2::ZERO, stick, time.delta_secs());
        turn_view(&mut body, &mut camera, delta);
    }
}

/// Gives the first player and each other one their part of the window, in the order they
/// joined; the first player alone has the whole of it. Only the view drawn first clears the
/// window: the views share one image, and each that clears it clears all of it, the views
/// drawn before included (`playroom.rs` found the same of the UI cameras). The last to join
/// has the lowest order, so it draws first. Writes only a view that changed.
fn lay_out_views(
    window: Single<&Window, With<PrimaryWindow>>,
    mut first: Single<&mut Camera, With<PlayroomCamera>>,
    guests: Query<&Guest>,
    mut cameras: Query<&mut Camera, (With<GuestCamera>, Without<PlayroomCamera>)>,
) {
    let mut seated: Vec<&Guest> = guests.iter().collect();
    seated.sort_by_key(|guest| guest.seat);
    let size = window.physical_size();
    let views = views_of(seated.len() + 1, size);
    // the first player alone keeps the whole window, which `None` is
    let first_view = views.first().filter(|_| !seated.is_empty()).cloned();
    set_view(&mut first, first_view, seated.is_empty());
    let last_seat = seated.last().map(|guest| guest.seat);
    for (guest, view) in seated.iter().zip(views.iter().skip(1)) {
        if let Ok(mut camera) = cameras.get_mut(guest.camera) {
            set_view(
                &mut camera,
                Some(view.clone()),
                Some(guest.seat) == last_seat,
            );
        }
    }
}

/// Gives every gamepad back to the first player as gameplay ends, its guests gone with it.
fn give_back_every_pad(mut commands: Commands, pads: Query<Entity, With<GuestPad>>) {
    for pad in &pads {
        commands.entity(pad).remove::<GuestPad>();
    }
}

// ==== Helpers ================================================================

/// Puts `camera` on `view`, `None` being the whole window, clearing the window first or not as
/// `clears` says, unless it is so already, since writing the same would still mark the camera
/// changed.
fn set_view(camera: &mut Camera, view: Option<Viewport>, clears: bool) {
    let same = match (&camera.viewport, &view) {
        (None, None) => true,
        (Some(now), Some(next)) => {
            now.physical_position == next.physical_position
                && now.physical_size == next.physical_size
        }
        _ => false,
    };
    if !same {
        camera.viewport = view;
    }
    let clearing = !matches!(camera.clear_color, ClearColorConfig::None);
    if clearing != clears {
        camera.clear_color = if clears {
            ClearColorConfig::Default
        } else {
            ClearColorConfig::None
        };
    }
}

/// Returns the views of `players` players on a window of `size` pixels, in seat order: two
/// share it top and bottom, a third takes the bottom half's right, and four have a quarter
/// each. The first player keeps the top or the top left.
fn views_of(players: usize, size: UVec2) -> Vec<Viewport> {
    let half = size / 2;
    let view = |x: u32, y: u32, width: u32, height: u32| Viewport {
        physical_position: UVec2::new(x, y),
        physical_size: UVec2::new(width, height).max(UVec2::ONE),
        ..default()
    };
    match players {
        0 | 1 => vec![view(0, 0, size.x, size.y)],
        2 => vec![
            view(0, 0, size.x, half.y),
            view(0, half.y, size.x, size.y - half.y),
        ],
        3 => vec![
            view(0, 0, size.x, half.y),
            view(0, half.y, half.x, size.y - half.y),
            view(half.x, half.y, size.x - half.x, size.y - half.y),
        ],
        _ => vec![
            view(0, 0, half.x, half.y),
            view(half.x, 0, size.x - half.x, half.y),
            view(0, half.y, half.x, size.y - half.y),
            view(half.x, half.y, size.x - half.x, size.y - half.y),
        ],
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: UVec2 = UVec2::new(1280, 721);
    // every layout `views_of` has, beyond the players a game lets in today
    const LAID_OUT_PLAYERS: usize = 4;

    /// Returns the area each view covers.
    fn areas(views: &[Viewport]) -> u32 {
        views
            .iter()
            .map(|view| view.physical_size.x * view.physical_size.y)
            .sum()
    }

    #[test]
    fn the_views_cover_the_window_once_whatever_the_players() {
        for players in 1..=LAID_OUT_PLAYERS {
            let views = views_of(players, WINDOW);
            assert_eq!(views.len(), players);
            assert_eq!(areas(&views), WINDOW.x * WINDOW.y);
        }
    }

    #[test]
    fn two_players_share_the_window_top_and_bottom() {
        let views = views_of(2, WINDOW);
        assert_eq!(views[0].physical_position, UVec2::ZERO);
        assert_eq!(views[0].physical_size.x, WINDOW.x);
        assert_eq!(views[1].physical_position.y, views[0].physical_size.y);
    }

    #[test]
    fn a_view_never_has_no_size_on_a_window_shrunk_to_nothing() {
        for view in views_of(LAID_OUT_PLAYERS, UVec2::ZERO) {
            assert!(view.physical_size.x > 0 && view.physical_size.y > 0);
        }
    }
}
