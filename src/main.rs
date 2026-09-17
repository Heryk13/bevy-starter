#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

use bevy::prelude::*;

mod screens;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    app.add_plugins(screens::plugin);

    app.add_systems(Startup, spawn_camera);

    app.run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
