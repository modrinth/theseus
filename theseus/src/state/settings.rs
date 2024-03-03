//! Theseus settings file
use crate::{
    jre::{self, autodetect_java_globals, find_filtered_jres},
    State,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

use super::{DirectoryInfo, JavaGlobals};

// TODO: convert to semver?
const CURRENT_FORMAT_VERSION: u32 = 1;

// Types
/// Global Theseus settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub theme: Theme,
    pub memory: MemorySettings,
    #[serde(default)]
    pub force_fullscreen: bool,
    pub game_resolution: WindowSize,
    pub custom_java_args: Vec<String>,
    pub custom_env_args: Vec<(String, String)>,
    pub java_globals: JavaGlobals,
    pub hooks: Hooks,
    pub max_concurrent_downloads: usize,
    pub max_concurrent_writes: usize,
    pub version: u32,
    pub collapsed_navigation: bool,
    #[serde(default)]
    pub disable_discord_rpc: bool,
    #[serde(default)]
    pub hide_on_process: bool,
    #[serde(default)]
    pub native_decorations: bool,
    #[serde(default)]
    pub default_page: DefaultPage,
    #[serde(default)]
    pub developer_mode: bool,
    #[serde(default)]
    pub opt_out_analytics: bool,
    #[serde(default)]
    pub advanced_rendering: bool,
    #[serde(default)]
    pub fully_onboarded: bool,
    #[serde(default = "DirectoryInfo::get_initial_settings_dir")]
    pub loaded_config_dir: Option<PathBuf>,
}

impl Settings {
    #[tracing::instrument]
    pub async fn init(file: &Path) -> crate::Result<Self> {
        let mut rescued = false;

        let settings = if file.exists() {
            let loaded_settings = fs::read(&file)
                .await
                .map_err(|err| {
                    crate::ErrorKind::FSError(format!(
                        "Error reading settings file: {err}"
                    ))
                    .as_error()
                })
                .and_then(|it| {
                    serde_json::from_slice::<Settings>(&it)
                        .map_err(crate::Error::from)
                });
            // settings is corrupted. Back up the file and create a new one
            if let Err(ref err) = loaded_settings {
                tracing::error!("Failed to load settings file: {err}. ");
                let backup_file = file.with_extension("json.bak");
                tracing::error!("Corrupted settings file will be backed up as {}, and a new settings file will be created.", backup_file.display());
                let _ = fs::rename(file, backup_file).await;
                rescued = true;
            }
            loaded_settings.ok()
        } else {
            None
        };

        if let Some(settings) = settings {
            Ok(settings)
        } else {
            // Create new settings file
            let settings = Self {
                theme: Theme::Dark,
                memory: MemorySettings::default(),
                force_fullscreen: false,
                game_resolution: WindowSize::default(),
                custom_java_args: Vec::new(),
                custom_env_args: Vec::new(),
                java_globals: JavaGlobals::new(),
                hooks: Hooks::default(),
                max_concurrent_downloads: 10,
                max_concurrent_writes: 10,
                version: CURRENT_FORMAT_VERSION,
                collapsed_navigation: false,
                disable_discord_rpc: false,
                hide_on_process: false,
                native_decorations: false,
                default_page: DefaultPage::Home,
                developer_mode: false,
                opt_out_analytics: false,
                advanced_rendering: true,
                fully_onboarded: rescued, // If we rescued the settings file, we should consider the user fully onboarded

                // By default, the config directory is the same as the settings directory
                loaded_config_dir: DirectoryInfo::get_initial_settings_dir(),
            };
            if rescued {
                settings.sync(file).await?;
            }
            Ok(settings)
        }
    }

    #[tracing::instrument]
    #[theseus_macros::debug_pin]
    pub async fn update_java() {
        let res = async {
            let state = State::get().await?;
            let settings_read = state.settings.write().await;

            if settings_read.java_globals.count() == 0 {
                drop(settings_read);
                let jres = jre::get_all_jre().await?;
                let java_8 =
                    find_filtered_jres("1.8", jres.clone(), false).await?;
                let java_17 =
                    find_filtered_jres("1.17", jres.clone(), false).await?;
                let java_18plus =
                    find_filtered_jres("1.18", jres.clone(), true).await?;
                let java_globals =
                    autodetect_java_globals(java_8, java_17, java_18plus)
                        .await?;
                state.settings.write().await.java_globals = java_globals;
            }

            Ok::<(), crate::Error>(())
        }
        .await;

        match res {
            Ok(()) => {}
            Err(err) => {
                tracing::warn!("Unable to update launcher java: {err}")
            }
        };
    }

    #[tracing::instrument(skip(self))]
    pub async fn sync(&self, to: &Path) -> crate::Result<()> {
        fs::write(to, serde_json::to_vec(self)?)
            .await
            .map_err(|err| {
                crate::ErrorKind::FSError(format!(
                    "Error saving settings to file: {err}"
                ))
                .as_error()
            })?;
        Ok(())
    }
}

/// Theseus theme
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    Oled,
}

/// Minecraft memory settings
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct MemorySettings {
    pub maximum: u32,
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self { maximum: 2048 }
    }
}

/// Game window size
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct WindowSize(pub u16, pub u16);

impl Default for WindowSize {
    fn default() -> Self {
        Self(854, 480)
    }
}

/// Game initialization hooks
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct Hooks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_launch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapper: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_exit: Option<String>,
}

/// Opening window to start with
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum DefaultPage {
    Home,
    Library,
}

impl Default for DefaultPage {
    fn default() -> Self {
        Self::Home
    }
}
