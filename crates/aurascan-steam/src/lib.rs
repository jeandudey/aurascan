use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::sync::LazyLock;

mod library_folders;

use eyre::{Context, OptionExt};
pub use library_folders::{LibraryFolder, LibraryFolders};

const KNOWN_PATHS: &[&str] = &[
    ".steam/steam/",
    ".local/share/Steam/",
    ".steam/debian-installation/",
];

static STEAM_PATHS: LazyLock<Vec<PathBuf>> = LazyLock::new(|| {
    let Some(home) = env::home_dir() else {
        return Vec::new();
    };

    KNOWN_PATHS
        .iter()
        .map(|path| home.join(path))
        .filter(|path| path.exists())
        .collect()
});

/// Parses the libraryfolders.vdf from the first Steam installation found.
pub fn library_folders() -> eyre::Result<LibraryFolders> {
    let library_folders_vdf = STEAM_PATHS
        .iter()
        .map(|path| path.join("steamapps/libraryfolders.vdf"))
        .filter(|path| path.exists())
        .next()
        .ok_or_eyre("libraryfolders.vdf not found")?;

    let file = File::open(library_folders_vdf).wrap_err("Failed to open libraryfolders.vdf")?;
    let mut library_folders: LibraryFolders =
        keyvalues_serde::from_reader(file).wrap_err("Failed to parse libraryfolders.vdf")?;
    library_folders
        .folders
        .retain(|_, folder| folder.path.exists());
    Ok(library_folders)
}

/*
pub fn wine_prefix(application_id: u64) -> eyre::Result<PathBuf> {
    let home =
        env::var("HOME").wrap_err("Failed to retrieve home directory environment variable")?;

    for steam_path in STEAM_PATHS {
        let mut path = PathBuf::from(&home);
        path.push(steam_path);
        path.push(application_id.to_string());
        path.push("pfx");

        if path.exists() {
            return Ok(path);
        }
    }

    eyre::bail!("Wine prefix for application ID {application_id} not found")
}
*/
