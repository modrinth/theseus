//! Theseus profile management interface

use std::path::{Path, PathBuf};
use tokio::fs;

use io::IOError;
use tokio::sync::RwLock;

use crate::{
    event::emit::{emit_loading, init_loading},
    prelude::DirectoryInfo,
    state::{self, Profiles},
    util::{fetch, io},
};
pub use crate::{
    state::{
        Hooks, JavaSettings, MemorySettings, Profile, Settings, WindowSize,
    },
    State,
};

/// Gets entire settings
#[tracing::instrument]
pub async fn get() -> crate::Result<Settings> {
    let state = State::get().await?;
    let settings = state.settings.read().await;
    Ok(settings.clone())
}

/// Sets entire settings
#[tracing::instrument]
pub async fn set(settings: Settings) -> crate::Result<()> {
    let state = State::get().await?;

    if settings.loaded_config_dir
        != state.settings.read().await.loaded_config_dir
    {
        return Err(crate::ErrorKind::OtherError(
            "Cannot change config directory as setting".to_string(),
        )
        .as_error());
    }

    let (reset_io, reset_fetch) = async {
        let read = state.settings.read().await;
        (
            settings.max_concurrent_writes != read.max_concurrent_writes,
            settings.max_concurrent_downloads != read.max_concurrent_downloads,
        )
    }
    .await;

    let updated_discord_rpc = {
        let read = state.settings.read().await;
        settings.disable_discord_rpc != read.disable_discord_rpc
    };

    {
        *state.settings.write().await = settings;
    }

    if updated_discord_rpc {
        state.discord_rpc.clear_to_default(true).await?;
    }

    if reset_io {
        state.reset_io_semaphore().await;
    }
    if reset_fetch {
        state.reset_fetch_semaphore().await;
    }

    State::sync().await?;
    Ok(())
}

/// Sets the new config dir, the location of all Theseus data except for the settings.json and caches
/// Takes control of the entire state and blocks until completion
pub async fn set_config_dir(new_config_dir: PathBuf) -> crate::Result<()> {
    tracing::trace!("Changing config dir to: {}", new_config_dir.display());
    if !new_config_dir.is_dir() {
        return Err(crate::ErrorKind::FSError(format!(
            "New config dir is not a folder: {}",
            new_config_dir.display()
        ))
        .as_error());
    }

    if !is_dir_writeable(new_config_dir.clone()).await? {
        return Err(crate::ErrorKind::FSError(format!(
            "New config dir is not writeable: {}",
            new_config_dir.display()
        ))
        .as_error());
    }

    let loading_bar = init_loading(
        crate::LoadingBarType::ConfigChange {
            new_path: new_config_dir.clone(),
        },
        100.0,
        "Changing configuration directory",
    )
    .await?;

    tracing::trace!("Changing config dir, taking control of the state");
    // Take control of the state
    let mut state_write = State::get_write().await?;
    let old_config_dir =
        state_write.directories.config_dir.read().await.clone();

    // Reset file watcher
    tracing::trace!("Reset file watcher");
    let file_watcher = state::init_watcher().await?;
    state_write.file_watcher = RwLock::new(file_watcher);

    // Getting files to be moved
    let mut config_entries = io::read_dir(&old_config_dir).await?;
    let across_drives = is_different_drive(&old_config_dir, &new_config_dir);
    let mut entries = vec![];
    let mut deletable_entries = vec![];
    while let Some(entry) = config_entries
        .next_entry()
        .await
        .map_err(|e| IOError::with_path(e, &old_config_dir))?
    {
        let entry_path = entry.path();
        if let Some(file_name) = entry_path.file_name() {
            // We are only moving the profiles and metadata folders
            if file_name == state::PROFILES_FOLDER_NAME
                || file_name == state::METADATA_FOLDER_NAME
            {
                if across_drives {
                    entries.extend(
                        crate::pack::import::get_all_subfiles(&entry_path)
                            .await?,
                    );
                    deletable_entries.push(entry_path.clone());
                } else {
                    entries.push(entry_path.clone());
                }
            }
        }
    }

    tracing::trace!("Moving files");
    let semaphore = &state_write.io_semaphore;
    let num_entries = entries.len() as f64;
    for entry_path in entries {
        let relative_path = entry_path.strip_prefix(&old_config_dir)?;
        let new_path = new_config_dir.join(relative_path);
        if across_drives {
            fetch::copy(&entry_path, &new_path, semaphore).await?;
        } else {
            io::rename(entry_path.clone(), new_path.clone()).await?;
        }
        emit_loading(&loading_bar, 80.0 * (1.0 / num_entries), None).await?;
    }

    tracing::trace!("Setting configuration setting");
    // Set load config dir setting
    let settings = {
        let mut settings = state_write.settings.write().await;
        settings.loaded_config_dir = Some(new_config_dir.clone());

        // Some java paths are hardcoded to within our config dir, so we need to update them
        tracing::trace!("Updating java keys");
        for key in settings.java_globals.keys() {
            if let Some(java) = settings.java_globals.get_mut(&key) {
                // If the path is within the old config dir path, update it to the new config dir
                if let Ok(relative_path) = PathBuf::from(java.path.clone())
                    .strip_prefix(&old_config_dir)
                {
                    java.path = new_config_dir
                        .join(relative_path)
                        .to_string_lossy()
                        .to_string();
                }
            }
        }
        tracing::trace!("Syncing settings");

        settings
            .sync(&state_write.directories.settings_file())
            .await?;
        settings.clone()
    };

    tracing::trace!("Reinitializing directory");
    // Set new state information
    state_write.directories = DirectoryInfo::init(&settings)?;

    // Delete entries that were from a different drive
    let deletable_entries_len = deletable_entries.len();
    if deletable_entries_len > 0 {
        tracing::trace!("Deleting old files");
    }
    for entry in deletable_entries {
        io::remove_dir_all(entry).await?;
        emit_loading(
            &loading_bar,
            10.0 * (1.0 / deletable_entries_len as f64),
            None,
        )
        .await?;
    }

    // Reset file watcher
    tracing::trace!("Reset file watcher");
    let mut file_watcher = state::init_watcher().await?;

    // Reset profiles (for filepaths, file watcher, etc)
    state_write.profiles = RwLock::new(
        Profiles::init(&state_write.directories, &mut file_watcher).await?,
    );
    state_write.file_watcher = RwLock::new(file_watcher);

    emit_loading(&loading_bar, 10.0, None).await?;

    tracing::info!(
        "Successfully switched config folder to: {}",
        new_config_dir.display()
    );
    Ok(())
}

// Function to check if two paths are on different drives/roots
fn is_different_drive(path1: &Path, path2: &Path) -> bool {
    let root1 = path1.components().next();
    let root2 = path2.components().next();
    root1 != root2
}

pub async fn is_dir_writeable(new_config_dir: PathBuf) -> crate::Result<bool> {
    let temp_path = new_config_dir.join(".tmp");
    match fs::write(temp_path.clone(), "test").await {
        Ok(_) => {
            fs::remove_file(temp_path).await?;
            Ok(true)
        }
        Err(e) => {
            tracing::error!("Error writing to new config dir: {}", e);
            Ok(false)
        }
    }
}
