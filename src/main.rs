#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

#[cfg(feature = "dev")]
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::prelude::*;

mod screens;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevy Starter".into(),
            ..default()
        }),
        ..default()
    }));

    #[cfg(feature = "dev")]
    app.add_plugins(FpsOverlayPlugin::default());

    app.add_plugins(screens::plugin);

    app.add_systems(Startup, spawn_camera);

    app.run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
