use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::Button;

// ==== Config =================================================================

// ---- palette: dark enough for the white label Text brings by default --------
const BUTTON_NORMAL: Color = Color::srgb(0.15, 0.15, 0.17);
const BUTTON_HOVERED: Color = Color::srgb(0.27, 0.27, 0.31);
const BUTTON_PRESSED: Color = Color::srgb(0.09, 0.09, 0.11);

// ---- sizing -----------------------------------------------------------------
const BUTTON_WIDTH_PX: f32 = 100.0;
const BUTTON_HEIGHT_PX: f32 = 50.0;
const BUTTON_FONT_SIZE_PX: f32 = 16.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    // no run_if in this file: a button lives on any screen and inside any overlay, so
    // there is no state these systems could be gated on
    app.add_systems(Startup, load_button_sounds);
    app.add_systems(Update, (paint_buttons, play_button_sounds));
}

// ==== Widgets ================================================================

// not one of the sections in the file layout: this is what the other files import from
// here, so it reads wrong under `Helpers`, which is for what this file's own systems use

/// Returns a button with the given label, ready for a screen to spawn.
#[expect(dead_code, reason = "the menu in step 3 is its first caller")]
pub(crate) fn button(text: &str) -> impl Bundle {
    (
        Node {
            width: px(BUTTON_WIDTH_PX),
            height: px(BUTTON_HEIGHT_PX),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Button,
        BackgroundColor(BUTTON_NORMAL),
        Hovered::default(),
        children![(
            Text::new(text),
            TextFont {
                font_size: BUTTON_FONT_SIZE_PX.into(),
                ..default()
            }
        )],
    )
}

// ==== Resources ==============================================================

/// Holds the two sounds every button in the app plays.
#[derive(Resource)]
struct ButtonSounds {
    click: Handle<AudioSource>,
    hover: Handle<AudioSource>,
}

// ==== Systems ================================================================

/// Loads the button sounds once, so playing one never waits on the asset server.
fn load_button_sounds(mut commands: Commands, asset_server: Res<AssetServer>) {
    let button_sounds = ButtonSounds {
        click: asset_server.load("ui/click.wav"),
        hover: asset_server.load("ui/hover.wav"),
    };

    commands.insert_resource(button_sounds);
}

/// Paints every button in the app from its hover and press state.
fn paint_buttons(mut buttons: Query<(&mut BackgroundColor, &Hovered, Has<Pressed>), With<Button>>) {
    for (mut background_color, hovered, pressed) in &mut buttons {
        background_color.0 = button_color(hovered.get(), pressed);
    }
}

/// Plays the tick when a pointer enters a button, and the click when one is pressed.
fn play_button_sounds(
    hovered_buttons: Query<&Hovered, (Changed<Hovered>, With<Button>)>,
    pressed_buttons: Query<(), (Added<Pressed>, With<Button>)>,
    sounds: Res<ButtonSounds>,
    mut commands: Commands,
) {
    // ---- hover: the filter catches leaving and spawning too, hence the if ---
    for hovered in &hovered_buttons {
        if hovered.get() {
            commands.spawn((
                AudioPlayer::new(sounds.hover.clone()),
                PlaybackSettings::DESPAWN,
            ));
        }
    }

    // ---- press: being in this list already means it happened this frame -----
    for _ in &pressed_buttons {
        commands.spawn((
            AudioPlayer::new(sounds.click.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}

// ==== Helpers ================================================================

/// Returns the background color of a button in the given interaction state.
fn button_color(hovered: bool, pressed: bool) -> Color {
    if pressed {
        BUTTON_PRESSED
    } else if hovered {
        BUTTON_HOVERED
    } else {
        BUTTON_NORMAL
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // exact comparison, not a tolerance: button_color returns the constant unchanged,
    // so there is no arithmetic that could drift

    #[test]
    fn button_is_normal_when_untouched() {
        assert_eq!(button_color(false, false), BUTTON_NORMAL);
    }

    #[test]
    fn button_changes_color_when_hovered() {
        assert_eq!(button_color(true, false), BUTTON_HOVERED);
    }

    #[test]
    fn button_changes_color_when_pressed() {
        assert_eq!(button_color(false, true), BUTTON_PRESSED);
    }

    #[test]
    fn button_changes_color_when_hovered_and_pressed() {
        assert_eq!(button_color(true, true), BUTTON_PRESSED);
    }
}
