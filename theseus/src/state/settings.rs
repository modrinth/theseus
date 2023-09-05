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
    pub show_username_rpc: bool,
    pub game_resolution: WindowSize,
    pub custom_java_args: Vec<String>,
    pub custom_env_args: Vec<(String, String)>,
    pub java_globals: JavaGlobals,
    pub default_user: Option<uuid::Uuid>,
    pub hooks: Hooks,
    pub max_concurrent_downloads: usize,
    pub max_concurrent_writes: usize,
    pub version: u32,
    pub collapsed_navigation: bool,
    #[serde(default)]
    pub hide_on_process: bool,
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
        if file.exists() {
            fs::read(&file)
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
                })
        } else {
            Ok(Self {
                theme: Theme::Dark,
                memory: MemorySettings::default(),
                force_fullscreen: false,
                show_username_rpc: false,
                game_resolution: WindowSize::default(),
                custom_java_args: Vec::new(),
                custom_env_args: Vec::new(),
                java_globals: JavaGlobals::new(),
                default_user: None,
                hooks: Hooks::default(),
                max_concurrent_downloads: 10,
                max_concurrent_writes: 10,
                version: CURRENT_FORMAT_VERSION,
                collapsed_navigation: false,
                hide_on_process: false,
                default_page: DefaultPage::Home,
                developer_mode: false,
                opt_out_analytics: false,
                advanced_rendering: true,
                fully_onboarded: false,

                // By default, the config directory is the same as the settings directory
                loaded_config_dir: DirectoryInfo::get_initial_settings_dir(),
            })
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
