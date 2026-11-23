use bevy::prelude::*;
use std::path::PathBuf;

use crate::disk::game_data_dir;
use crate::input::ActionName;
use crate::ui::blur::GlassLayers;
use crate::ui::{BORDER_PX, card_glass, floating_root};

mod file_explorer;
mod fly;
mod fps;
mod free_camera;
mod keys_panel;
mod photo_mode;
// read by the third-person camera, which draws its own rays in the second view
pub(crate) mod physics_debug;
// read by each genre, which shows the props, moves the camera and stops the player
pub(crate) mod prop_editor;
mod prop_editor_3d;
mod recording;
mod slow_motion;
// read by each player, which puts its body on the capsule and walks by the multiplier
pub(crate) mod speed;

// ==== Config =================================================================

// above the dialog (200), so a dev window is drawn by a camera of its own over every other
// layer: sharp over the pause menu, with whatever is open under it blurred in its glass
const DEV_TOOLS_Z_INDEX: GlobalZIndex = GlobalZIndex(300);

// ---- windows: small glass read-outs in a corner, from the mockup's popover --
const DEV_WINDOW_PADDING_X_PX: f32 = 12.0;
const DEV_WINDOW_PADDING_Y_PX: f32 = 10.0;
const DEV_WINDOW_RADIUS_PX: f32 = 22.0;
const DEV_WINDOW_GAP_PX: f32 = 6.0;

// ---- capture: E takes a photo, and starts and stops a recording -------------
// one action for both, since a key belongs to one action only, and each tool reads it in its
// own moment: photo mode in `Pause::Photo`, recording while its bar is open or it records
const CAPTURE: ActionName = ActionName("capture");
const CAPTURE_KEY: KeyCode = KeyCode::KeyE;
// English, as every dev tool's words: on the Dev page and the keys panel
const CAPTURE_LABEL: &str = "Capture";

// ---- gallery: photos and videos land in it, the file explorer browses it ----
// in the game's data folder
const GALLERY_DIR_NAME: &str = "gallery";

// ---- dates: what photos and videos are named by, what the explorer shows ----
const SECS_PER_MINUTE: u64 = 60;
const SECS_PER_HOUR: u64 = 60 * SECS_PER_MINUTE;
const SECS_PER_DAY: u64 = 24 * SECS_PER_HOUR;

// ==== Plugin =================================================================

/// The tools a game is made with, left out of release builds: `main.rs` adds this plugin
/// only with the `dev` feature. Each tool is a file of its own under `dev_tools/`.
pub(super) fn plugin(app: &mut App) {
    // here as well as where a tool registers one, since the keys panel reads it whether any
    // tool has inner keys or not
    app.init_resource::<InnerKeys>();
    app.init_resource::<GlassLayers>();
    app.world_mut()
        .resource_mut::<GlassLayers>()
        .0
        .push(DEV_TOOLS_Z_INDEX);
    // in the order of their keys, which is the order the keys panel and the Dev page list them
    app.add_plugins((
        keys_panel::plugin,
        fps::plugin,
        photo_mode::plugin,
        slow_motion::plugin,
        physics_debug::plugin,
        fly::plugin,
        // no key of its own: photo mode's, after photo mode and fly, whose hint and keys it adds
        free_camera::plugin,
        speed::plugin,
        recording::plugin,
        prop_editor::plugin,
        // no key of its own: the editor's, after it, the 3D models, thumbnails and placing
        prop_editor_3d::plugin,
        file_explorer::plugin,
    ));
}

// ==== Inner Keys =============================================================

/// Leaves `action` off the keys panel: a key that only works inside a tool already open,
/// which shows it there (Capture on photo mode's bar, Slower and Faster on slow motion's
/// and speed's capsules). Settings' Dev page still lists it, since it can still be changed.
/// Called from the tool's plugin, so the keys panel never names another tool's keys.
fn register_inner_key(app: &mut App, action: ActionName) {
    app.init_resource::<InnerKeys>();
    app.world_mut().resource_mut::<InnerKeys>().0.push(action);
}

// ==== Gallery ================================================================

/// Returns the folder photos land in and the file explorer opens on, in the game's data
/// folder, or `None` on a machine with no data folder.
fn gallery_dir() -> Option<PathBuf> {
    game_data_dir().map(|dir| dir.join(GALLERY_DIR_NAME))
}

// ==== Dates ==================================================================

/// Returns the year, month and day `days` days after 1970-01-01: Howard Hinnant's
/// `civil_from_days`, the usual way to turn a day count into the Gregorian calendar, with
/// its well-known constants. It counts from 0000-03-01, so a leap day falls at the end of a
/// year, in eras of 400 years, which repeat exactly; days before 1970 never come here.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let days = days + 719_468;
    let era = days / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

/// Returns the date and time, in UTC, `secs` seconds after 1970 began, as a photo's or a
/// recording's name spells it: `2026-11-18_14-32-05`, which sorts in the order they were
/// taken.
fn file_stamp(secs: u64) -> String {
    let (year, month, day) = civil_from_days(secs / SECS_PER_DAY);
    let secs_today = secs % SECS_PER_DAY;
    let hour = secs_today / SECS_PER_HOUR;
    let minute = secs_today % SECS_PER_HOUR / SECS_PER_MINUTE;
    let second = secs_today % SECS_PER_MINUTE;
    format!("{year:04}-{month:02}-{day:02}_{hour:02}-{minute:02}-{second:02}")
}

/// Returns the name of the `count`th file with `extension` taken in the second `stamp`
/// names: the stamp alone for the first, then `_2`, `_3`.
fn numbered_file_name(stamp: &str, count: u32, extension: &str) -> String {
    if count <= 1 {
        format!("{stamp}.{extension}")
    } else {
        format!("{stamp}_{count}.{extension}")
    }
}

// ==== Windows ================================================================

/// Spawns `window` under a root of its own carrying `root_marker`, which the tool despawns
/// to close it: the whole window on the dev tools' layer, like every floating root, but
/// letting the pointer through, so the game under it stays sharp and keeps taking clicks.
/// Every node inside lets it through too, since Bevy's picking stops at a node with no
/// `Pickable`, and the window is built from widgets that bring none.
fn spawn_dev_window(commands: &mut Commands, root_marker: impl Bundle, window: impl Bundle) {
    commands
        // on the root as it spawns, not left to the pass below: `blur.rs` reads it there to
        // give this root no backdrop
        .spawn((
            floating_root(DEV_TOOLS_Z_INDEX),
            Pickable::IGNORE,
            DevUi,
            root_marker,
            children![window],
        ))
        .insert_recursive::<Children>(Pickable::IGNORE);
}

/// Returns a dev window placed where `place` says (its `top`, `left` or `right`), for
/// `spawn_dev_window`: a glass sheet that lays its children in a column. Not a `card()`,
/// though floating UI is drawn on one: a card is a centered panel sized for reading, this a
/// corner read-out sized to its few lines.
fn dev_window(place: Node) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            row_gap: px(DEV_WINDOW_GAP_PX),
            padding: UiRect::axes(px(DEV_WINDOW_PADDING_X_PX), px(DEV_WINDOW_PADDING_Y_PX)),
            border: UiRect::all(px(BORDER_PX)),
            border_radius: BorderRadius::all(px(DEV_WINDOW_RADIUS_PX)),
            ..place
        },
        card_glass(),
    )
}

// ==== Components =============================================================

/// Marks the root of anything the dev tools draw, so photo mode can hide it all and take
/// the game alone. Every dev window has it from `spawn_dev_window`; a root spawned any other
/// way puts it on itself, and the key that opens it does nothing in photo mode
/// (`.run_if(not(in_state(Pause::Photo)))`), so nothing hidden opens behind a photo.
#[derive(Component)]
struct DevUi;

// ==== Resources ==============================================================

/// The keys the keys panel leaves out, since the tool they work in shows them itself.
#[derive(Resource, Default)]
struct InnerKeys(Vec<ActionName>);

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // checked against the Unix clock: 2024-01-01 is 1704067200, and the 29th of February is
    // 59 days after it
    const LEAP_DAY_SECS: u64 = 1_704_067_200 + 59 * SECS_PER_DAY;
    // 2026-01-01 is 1767225600
    const LAST_SECOND_OF_2025: u64 = 1_767_225_600 - 1;

    #[test]
    fn the_first_second_of_1970_is_its_first_day() {
        assert_eq!(file_stamp(0), "1970-01-01_00-00-00");
    }

    #[test]
    fn a_leap_day_is_named_as_one() {
        let secs = LEAP_DAY_SECS + 12 * SECS_PER_HOUR + 34 * SECS_PER_MINUTE + 56;
        assert_eq!(file_stamp(secs), "2024-02-29_12-34-56");
    }

    #[test]
    fn the_last_second_of_a_year_stays_in_it() {
        assert_eq!(file_stamp(LAST_SECOND_OF_2025), "2025-12-31_23-59-59");
        assert_eq!(file_stamp(LAST_SECOND_OF_2025 + 1), "2026-01-01_00-00-00");
    }

    #[test]
    fn a_second_file_in_the_same_second_gets_a_name_of_its_own() {
        let stamp = "2026-11-18_14-32-05";
        assert_eq!(
            numbered_file_name(stamp, 1, "png"),
            "2026-11-18_14-32-05.png"
        );
        assert_eq!(
            numbered_file_name(stamp, 2, "mp4"),
            "2026-11-18_14-32-05_2.mp4"
        );
    }
}
