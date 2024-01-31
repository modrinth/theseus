use serde::{Deserialize, Serialize};
use theseus::{
    handler,
    prelude::{CommandPayload, DirectoryInfo},
    State,
};

use crate::api::Result;
use std::{env, path::PathBuf, process::Command};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("utils")
        .invoke_handler(tauri::generate_handler![
            get_os,
            should_disable_mouseover,
            show_in_folder,
            show_launcher_logs_folder,
            progress_bars_list,
            safety_check_safe_loading_bars,
            get_opening_command,
            await_sync,
            is_offline,
            refresh_offline,
            test_command,
        ])
        .build()
}

/// Gets OS
#[tauri::command]
pub fn get_os() -> OS {
    #[cfg(target_os = "windows")]
    let os = OS::Windows;
    #[cfg(target_os = "linux")]
    let os = OS::Linux;
    #[cfg(target_os = "macos")]
    let os = OS::MacOS;
    os
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OS {
    Windows,
    Linux,
    MacOS,
}

// Lists active progress bars
// Create a new HashMap with the same keys
// Values provided should not be used directly, as they are not guaranteed to be up-to-date
#[tauri::command]
pub async fn progress_bars_list(
) -> Result<std::collections::HashMap<uuid::Uuid, theseus::LoadingBar>> {
    let res = theseus::EventState::list_progress_bars().await?;
    Ok(res)
}

// Check if there are any safe loading bars running
#[tauri::command]
pub async fn safety_check_safe_loading_bars() -> Result<bool> {
    Ok(theseus::safety::check_safe_loading_bars().await?)
}

// cfg only on mac os
// disables mouseover and fixes a random crash error only fixed by recent versions of macos
#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn should_disable_mouseover() -> bool {
    // We try to match version to 12.2 or higher. If unrecognizable to pattern or lower, we default to the css with disabled mouseover for safety
    let os = os_info::get();
    if let os_info::Version::Semantic(major, minor, _) = os.version() {
        if *major >= 12 && *minor >= 3 {
            // Mac os version is 12.3 or higher, we allow mouseover
            return false;
        }
    }
    true
}
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn should_disable_mouseover() -> bool {
    false
}

#[tauri::command]
pub fn show_in_folder(path: PathBuf) -> Result<()> {
    {
        #[cfg(target_os = "windows")]
        {
            if path.is_dir() {
                Command::new("explorer")
                    .args([&path]) // The comma after select is not a typo
                    .spawn()?;
            } else {
                Command::new("explorer")
                    .args(["/select,", &path.to_string_lossy()]) // The comma after select is not a typo
                    .spawn()?;
            }
        }

        #[cfg(target_os = "linux")]
        {
            use std::fs::metadata;
            use std::path::PathBuf;

            if path.to_string_lossy().to_string().contains(',') {
                // see https://gitlab.freedesktop.org/dbus/dbus/-/issues/76
                let new_path = match metadata(&path)?.is_dir() {
                    true => path,
                    false => {
                        let mut path2 = PathBuf::from(path);
                        path2.pop();
                        path2
                    }
                };
                Command::new("xdg-open").arg(&new_path).spawn()?;
            } else {
                Command::new("xdg-open").arg(&path).spawn()?;
            }
        }

        #[cfg(target_os = "macos")]
        {
            if path.is_dir() {
                Command::new("open").args([&path]).spawn()?;
            } else {
                Command::new("open")
                    .args(["-R", &path.as_os_str().to_string_lossy()])
                    .spawn()?;
            }
        }

        Ok::<(), theseus::Error>(())
    }?;

    Ok(())
}

#[tauri::command]
pub fn show_launcher_logs_folder() -> Result<()> {
    let path = DirectoryInfo::launcher_logs_dir().unwrap_or_default();
    // failure to get folder just opens filesystem
    // (ie: if in debug mode only and launcher_logs never created)
    show_in_folder(path)
}

// Get opening command
// For example, if a user clicks on an .mrpack to open the app.
// This should be called once and only when the app is done booting up and ready to receive a command
// Returns a Command struct- see events.js
#[tauri::command]
pub async fn get_opening_command() -> Result<Option<CommandPayload>> {
    // Tauri is not CLI, we use arguments as path to file to call
    let cmd_arg = env::args_os().nth(1);

    let cmd_arg = cmd_arg.map(|path| path.to_string_lossy().to_string());
    if let Some(cmd) = cmd_arg {
        tracing::debug!("Opening command: {:?}", cmd);
        return Ok(Some(handler::parse_command(&cmd).await?));
    }
    Ok(None)
}

#[tauri::command]
pub async fn test_command(command: String) -> Result<()> {
    handle_command(command).await
}

// helper function called when redirected by a weblink (ie: modrith://do-something) or when redirected by a .mrpack file (in which case its a filepath)
// We hijack the deep link library (which also contains functionality for instance-checking)
pub async fn handle_command(command: String) -> Result<()> {
    Ok(theseus::handler::parse_and_emit_command(&command).await?)
}

// Waits for state to be synced
#[tauri::command]
pub async fn await_sync() -> Result<()> {
    State::sync().await?;
    tracing::debug!("State synced");
    Ok(())
}

/// Check if theseus is currently in offline mode, without a refresh attempt
#[tauri::command]
pub async fn is_offline() -> Result<bool> {
    let state = State::get().await?;
    let offline = *state.offline.read().await;
    Ok(offline)
}

/// Refreshes whether or not theseus is in offline mode, and returns the new value
#[tauri::command]
pub async fn refresh_offline() -> Result<bool> {
    let state = State::get().await?;
    state.refresh_offline().await?;
    let offline = *state.offline.read().await;
    Ok(offline)
}
