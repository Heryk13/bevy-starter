use bevy::prelude::*;

mod splash;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum Screen {
    #[default]
    Splash,
    Menu,
}

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Screen>();
    app.add_plugins(splash::plugin);
}
