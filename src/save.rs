use bevy::prelude::*;

// ==== Config =================================================================

// play since the last save that quitting may lose without asking, so leaving
// right after starting or saving is not met with a question
const UNSAVED_GRACE_SECS: f32 = 30.0;

// ==== Session ================================================================

/// The game in progress: how long it has been played, and how long at its last save.
#[derive(Resource, Clone, Default)]
pub(crate) struct Session {
    pub(crate) play_secs: f32,
    pub(crate) saved_play_secs: f32,
}

/// Returns whether quitting now would lose play time that is in no save.
pub(crate) fn has_unsaved_progress(session: &Session) -> bool {
    session.play_secs - session.saved_play_secs > UNSAVED_GRACE_SECS
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_has_no_unsaved_progress() {
        let session = Session::default();
        assert!(!has_unsaved_progress(&session));
    }

    #[test]
    fn short_play_has_no_unsaved_progress() {
        let mut session = Session::default();
        session.play_secs = UNSAVED_GRACE_SECS / 2.0;
        assert!(!has_unsaved_progress(&session));
    }

    #[test]
    fn long_play_has_unsaved_progress() {
        let mut session = Session::default();
        session.play_secs = UNSAVED_GRACE_SECS * 2.0;
        assert!(has_unsaved_progress(&session));
    }

    #[test]
    fn saving_clears_unsaved_progress() {
        let mut session = Session::default();
        session.play_secs = UNSAVED_GRACE_SECS * 2.0;
        assert!(has_unsaved_progress(&session));
        session.saved_play_secs = session.play_secs;
        assert!(!has_unsaved_progress(&session));
    }
}
