//! Theseus directory information
use std::fs;
use std::path::PathBuf;

use tokio::sync::RwLock;

use super::{ProfilePathId, Settings};

pub const SETTINGS_FILE_NAME: &str = "settings.json";
pub const CACHES_FOLDER_NAME: &str = "caches";
pub const LAUNCHER_LOGS_FOLDER_NAME: &str = "launcher_logs";
pub const PROFILES_FOLDER_NAME: &str = "profiles";
pub const METADATA_FOLDER_NAME: &str = "meta";

#[derive(Debug)]
pub struct DirectoryInfo {
    pub settings_dir: PathBuf, // Base settings directory- settings.json and icon cache.
    pub config_dir: RwLock<PathBuf>, // Base config directory- instances, minecraft downloads, etc. Changeable as a setting.
    pub working_dir: PathBuf,
}

impl DirectoryInfo {
    // Get the settings directory
    // init() is not needed for this function
    pub fn get_initial_settings_dir() -> Option<PathBuf> {
        Self::env_path("THESEUS_CONFIG_DIR")
            .or_else(|| Some(dirs::config_dir()?.join("com.modrinth.theseus")))
    }

    #[inline]
    pub fn get_initial_settings_file() -> crate::Result<PathBuf> {
        let settings_dir = Self::get_initial_settings_dir().ok_or(
            crate::ErrorKind::FSError(
                "Could not find valid config dir".to_string(),
            ),
        )?;
        Ok(settings_dir.join("settings.json"))
    }

    /// Get all paths needed for Theseus to operate properly
    #[tracing::instrument]
    pub fn init(settings: &Settings) -> crate::Result<Self> {
        // Working directory
        let working_dir = std::env::current_dir().map_err(|err| {
            crate::ErrorKind::FSError(format!(
                "Could not open working directory: {err}"
            ))
        })?;

        let settings_dir = Self::get_initial_settings_dir().ok_or(
            crate::ErrorKind::FSError(
                "Could not find valid settings dir".to_string(),
            ),
        )?;

        fs::create_dir_all(&settings_dir).map_err(|err| {
            crate::ErrorKind::FSError(format!(
                "Error creating Theseus config directory: {err}"
            ))
        })?;

        // config directory (for instances, etc.)
        // by default this is the same as the settings directory
        let config_dir = settings.loaded_config_dir.clone().ok_or(
            crate::ErrorKind::FSError(
                "Could not find valid config dir".to_string(),
            ),
        )?;

        Ok(Self {
            settings_dir,
            config_dir: RwLock::new(config_dir),
            working_dir,
        })
    }

    /// Get the Minecraft instance metadata directory
    #[inline]
    pub async fn metadata_dir(&self) -> PathBuf {
        self.config_dir.read().await.join(METADATA_FOLDER_NAME)
    }

    /// Get the Minecraft java versions metadata directory
    #[inline]
    pub async fn java_versions_dir(&self) -> PathBuf {
        self.metadata_dir().await.join("java_versions")
    }

    /// Get the Minecraft versions metadata directory
    #[inline]
    pub async fn versions_dir(&self) -> PathBuf {
        self.metadata_dir().await.join("versions")
    }

    /// Get the metadata directory for a given version
    #[inline]
    pub async fn version_dir(&self, version: &str) -> PathBuf {
        self.versions_dir().await.join(version)
    }

    /// Get the Minecraft libraries metadata directory
    #[inline]
    pub async fn libraries_dir(&self) -> PathBuf {
        self.metadata_dir().await.join("libraries")
    }

    /// Get the Minecraft assets metadata directory
    #[inline]
    pub async fn assets_dir(&self) -> PathBuf {
        self.metadata_dir().await.join("assets")
    }

    /// Get the assets index directory
    #[inline]
    pub async fn assets_index_dir(&self) -> PathBuf {
        self.assets_dir().await.join("indexes")
    }

    /// Get the assets objects directory
    #[inline]
    pub async fn objects_dir(&self) -> PathBuf {
        self.assets_dir().await.join("objects")
    }

    /// Get the directory for a specific object
    #[inline]
    pub async fn object_dir(&self, hash: &str) -> PathBuf {
        self.objects_dir().await.join(&hash[..2]).join(hash)
    }

    /// Get the Minecraft legacy assets metadata directory
    #[inline]
    pub async fn legacy_assets_dir(&self) -> PathBuf {
        self.metadata_dir().await.join("resources")
    }

    /// Get the Minecraft legacy assets metadata directory
    #[inline]
    pub async fn natives_dir(&self) -> PathBuf {
        self.metadata_dir().await.join("natives")
    }

    /// Get the natives directory for a version of Minecraft
    #[inline]
    pub async fn version_natives_dir(&self, version: &str) -> PathBuf {
        self.natives_dir().await.join(version)
    }

    /// Get the directory containing instance icons
    #[inline]
    pub async fn icon_dir(&self) -> PathBuf {
        self.config_dir.read().await.join("icons")
    }

    /// Get the profiles directory for created profiles
    #[inline]
    pub async fn profiles_dir(&self) -> PathBuf {
        self.config_dir.read().await.join(PROFILES_FOLDER_NAME)
    }

    /// Gets the logs dir for a given profile
    #[inline]
    pub async fn profile_logs_dir(
        profile_id: &ProfilePathId,
    ) -> crate::Result<PathBuf> {
        Ok(profile_id.get_full_path().await?.join("logs"))
    }

    /// Gets the crash reports dir for a given profile
    #[inline]
    pub async fn crash_reports_dir(
        profile_id: &ProfilePathId,
    ) -> crate::Result<PathBuf> {
        Ok(profile_id.get_full_path().await?.join("crash-reports"))
    }

    #[inline]
    pub fn launcher_logs_dir() -> Option<PathBuf> {
        Self::get_initial_settings_dir()
            .map(|d| d.join(LAUNCHER_LOGS_FOLDER_NAME))
    }

    /// Get the file containing the global database
    #[inline]
    pub async fn database_file(&self) -> PathBuf {
        self.config_dir.read().await.join("data.bin")
    }

    /// Get the settings file for Theseus
    #[inline]
    pub fn settings_file(&self) -> PathBuf {
        self.settings_dir.join(SETTINGS_FILE_NAME)
    }

    /// Get the cache directory for Theseus
    #[inline]
    pub fn caches_dir(&self) -> PathBuf {
        self.settings_dir.join(CACHES_FOLDER_NAME)
    }

    #[inline]
    pub async fn caches_meta_dir(&self) -> PathBuf {
        self.caches_dir().join("metadata")
    }

    /// Get path from environment variable
    #[inline]
    fn env_path(name: &str) -> Option<PathBuf> {
        std::env::var_os(name).map(PathBuf::from)
    }
}
