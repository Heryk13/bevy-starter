use std::path::{Path, PathBuf};

// ==== Data Folder ============================================================

/// Returns the folder this game keeps its files in, or `None` when the OS gives no home.
/// `var` reads an environment variable, so tests can hand in any OS's.
fn data_dir(os: &str, var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    match os {
        "windows" => var("APPDATA").map(PathBuf::from),
        "macos" => var("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
        }),
        "linux" => var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| var("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))),
        _ => None,
    }
    .map(|dir| dir.join(env!("CARGO_PKG_NAME")))
}

/// Returns this game's data folder on the machine it runs on.
pub(crate) fn game_data_dir() -> Option<PathBuf> {
    data_dir(std::env::consts::OS, |name| std::env::var(name).ok())
}

/// Writes a file through a temporary one and a rename, so a crash mid-write leaves the
/// old file whole. Creates the folder when it is missing, so the first write on a machine
/// needs no setup.
// ponytail: read and written on the main thread; move to IoTaskPool when saves grow
// past a few KB
pub(crate) fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("ron.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(tmp, path)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- data_dir: fake environments answer only the variables they name ----
    const GAME: &str = env!("CARGO_PKG_NAME");

    #[test]
    fn windows_data_lives_in_appdata() {
        let dir = data_dir("windows", |name| match name {
            "APPDATA" => Some(String::from("C:\\Roaming")),
            _ => None,
        });
        assert_eq!(dir, Some(PathBuf::from("C:\\Roaming").join(GAME)));
    }

    #[test]
    fn linux_data_follows_xdg_data_home() {
        let dir = data_dir("linux", |name| match name {
            "XDG_DATA_HOME" => Some(String::from("/xdg")),
            "HOME" => Some(String::from("/home/ana")),
            _ => None,
        });
        assert_eq!(dir, Some(PathBuf::from("/xdg").join(GAME)));
    }

    #[test]
    fn linux_data_falls_back_to_local_share() {
        let dir = data_dir("linux", |name| match name {
            "HOME" => Some(String::from("/home/ana")),
            _ => None,
        });
        let expected = PathBuf::from("/home/ana")
            .join(".local")
            .join("share")
            .join(GAME);
        assert_eq!(dir, Some(expected));
    }

    #[test]
    fn macos_data_lives_in_application_support() {
        let dir = data_dir("macos", |name| match name {
            "HOME" => Some(String::from("/Users/ana")),
            _ => None,
        });
        let expected = PathBuf::from("/Users/ana")
            .join("Library")
            .join("Application Support")
            .join(GAME);
        assert_eq!(dir, Some(expected));
    }

    #[test]
    fn data_has_no_folder_without_a_home() {
        assert_eq!(data_dir("linux", |_| None), None);
    }

    #[test]
    fn unknown_os_has_no_data_folder() {
        assert_eq!(data_dir("plan9", |_| Some(String::from("/anywhere"))), None);
    }

    // ---- write_atomic: a real folder, named after the test, never shared ----

    #[test]
    fn written_file_reads_back_whole() {
        let dir = std::env::temp_dir().join("bevy-starter-write-atomic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.ron");

        write_atomic(&path, "(volume: 0.5)").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "(volume: 0.5)");
        assert!(!path.with_extension("ron.tmp").exists());
    }
}
