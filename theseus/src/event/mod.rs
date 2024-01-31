//! Theseus state management system
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::OnceCell;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::prelude::ProfilePathId;
use crate::state::SafeProcesses;

pub mod emit;

// Global event state
// Stores the Tauri app handle and other event-related state variables
static EVENT_STATE: OnceCell<Arc<EventState>> = OnceCell::const_new();
pub struct EventState {
    /// Tauri app
    #[cfg(feature = "tauri")]
    pub app: tauri::AppHandle,
    pub loading_bars: RwLock<HashMap<Uuid, LoadingBar>>,
}

impl EventState {
    #[cfg(feature = "tauri")]
    pub async fn init(app: tauri::AppHandle) -> crate::Result<Arc<Self>> {
        EVENT_STATE
            .get_or_try_init(|| async {
                Ok(Arc::new(Self {
                    app,
                    loading_bars: RwLock::new(HashMap::new()),
                }))
            })
            .await
            .map(Arc::clone)
    }

    #[cfg(not(feature = "tauri"))]
    pub async fn init() -> crate::Result<Arc<Self>> {
        EVENT_STATE
            .get_or_try_init(|| async {
                Ok(Arc::new(Self {
                    loading_bars: RwLock::new(HashMap::new()),
                }))
            })
            .await
            .map(Arc::clone)
    }

    #[cfg(feature = "tauri")]
    pub async fn get() -> crate::Result<Arc<Self>> {
        Ok(EVENT_STATE.get().ok_or(EventError::NotInitialized)?.clone())
    }

    // Initialization requires no app handle in non-tauri mode, so we can just use the same function
    #[cfg(not(feature = "tauri"))]
    pub async fn get() -> crate::Result<Arc<Self>> {
        Self::init().await
    }

    // Values provided should not be used directly, as they are clones and are not guaranteed to be up-to-date
    pub async fn list_progress_bars() -> crate::Result<HashMap<Uuid, LoadingBar>>
    {
        let value = Self::get().await?;
        let read = value.loading_bars.read().await;

        let mut display_list: HashMap<Uuid, LoadingBar> = HashMap::new();
        for (uuid, loading_bar) in read.iter() {
            display_list.insert(*uuid, loading_bar.clone());
        }

        Ok(display_list)
    }

    #[cfg(feature = "tauri")]
    pub async fn get_main_window() -> crate::Result<Option<tauri::Window>> {
        use tauri::Manager;
        let value = Self::get().await?;
        Ok(value.app.get_window("main"))
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct LoadingBar {
    // loading_bar_uuid not be used directly by external functions as it may not reflect the current state of the loading bar/hashmap
    pub loading_bar_uuid: Uuid,
    pub message: String,
    pub total: f64,
    pub current: f64,
    #[serde(skip)]
    pub last_sent: f64,
    pub bar_type: LoadingBarType,
    #[cfg(feature = "cli")]
    #[serde(skip)]
    pub cli_progress_bar: indicatif::ProgressBar,
}

#[derive(Serialize, Debug, Clone)]
pub struct LoadingBarId(Uuid);

// When Loading bar id is dropped, we should remove it from the hashmap
impl Drop for LoadingBarId {
    fn drop(&mut self) {
        let loader_uuid = self.0;
        tokio::spawn(async move {
            if let Ok(event_state) = EventState::get().await {
                let mut bars = event_state.loading_bars.write().await;

                #[cfg(any(feature = "tauri", feature = "cli"))]
                if let Some(bar) = bars.remove(&loader_uuid) {
                    #[cfg(feature = "tauri")]
                    {
                        let loader_uuid = bar.loading_bar_uuid;
                        let event = bar.bar_type.clone();
                        let fraction = bar.current / bar.total;

                        use tauri::Manager;
                        let _ = event_state.app.emit_all(
                            "loading",
                            LoadingPayload {
                                fraction: None,
                                message: "Completed".to_string(),
                                event,
                                loader_uuid,
                            },
                        );
                        tracing::trace!(
                            "Exited at {fraction} for loading bar: {:?}",
                            loader_uuid
                        );
                    }

                    // Emit event to indicatif progress bar arc
                    #[cfg(feature = "cli")]
                    {
                        let cli_progress_bar = bar.cli_progress_bar;
                        cli_progress_bar.finish();
                    }
                }

                #[cfg(not(any(feature = "tauri", feature = "cli")))]
                bars.remove(&loader_uuid);
            }
            // complete calls state, and since a  LoadingBarId is created in state initialization, we only complete if its already initializaed
            // to avoid an infinite loop.
            if crate::State::initialized() {
                let _ = SafeProcesses::complete(
                    crate::state::ProcessType::LoadingBar,
                    loader_uuid,
                )
                .await;
            }
        });
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum LoadingBarType {
    StateInit,
    JavaDownload {
        version: u32,
    },
    PackFileDownload {
        profile_path: PathBuf,
        pack_name: String,
        icon: Option<String>,
        pack_version: String,
    },
    PackDownload {
        profile_path: PathBuf,
        pack_name: String,
        icon: Option<PathBuf>,
        pack_id: Option<String>,
        pack_version: Option<String>,
    },
    MinecraftDownload {
        profile_path: PathBuf,
        profile_name: String,
    },
    ProfileUpdate {
        profile_path: PathBuf,
        profile_name: String,
    },
    ZipExtract {
        profile_path: PathBuf,
        profile_name: String,
    },
    ConfigChange {
        new_path: PathBuf,
    },
    CopyProfile {
        import_location: PathBuf,
        profile_name: String,
    },
}

#[derive(Serialize, Clone)]
pub struct LoadingPayload {
    pub event: LoadingBarType,
    pub loader_uuid: Uuid,
    pub fraction: Option<f64>, // by convention, if optional, it means the loading is done
    pub message: String,
}

#[derive(Serialize, Clone)]
pub struct OfflinePayload {
    pub offline: bool,
}

#[derive(Serialize, Clone)]
pub struct WarningPayload {
    pub message: String,
}

#[derive(Serialize, Clone)]
#[serde(tag = "event")]
pub enum CommandPayload {
    InstallMod {
        id: String,
    },
    InstallVersion {
        id: String,
    },
    InstallModpack {
        id: String,
    },
    RunMRPack {
        // run or install .mrpack
        path: PathBuf,
    },
    OpenSharedProfile {
        link: String,
    },
}

#[derive(Serialize, Clone)]
pub struct ProcessPayload {
    pub uuid: Uuid, // processes in state are going to be identified by UUIDs, as they might change to different processes
    pub pid: u32,
    pub event: ProcessPayloadType,
    pub message: String,
}
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProcessPayloadType {
    Launched,
    Updated, // eg: if the MinecraftChild changes to its post-command process instead of the Minecraft process
    Finished,
}

#[derive(Serialize, Clone)]
pub struct ProfilePayload {
    pub uuid: Uuid,
    pub profile_path_id: ProfilePathId,
    pub path: PathBuf,
    pub name: String,
    pub event: ProfilePayloadType,
}
#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ProfilePayloadType {
    Created,
    Added, // also triggered when Created
    Synced,
    Edited,
    Removed,
}

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("Event state was not properly initialized")]
    NotInitialized,

    #[error("Non-existent loading bar of key: {0}")]
    NoLoadingBar(Uuid),

    #[cfg(feature = "tauri")]
    #[error("Tauri error: {0}")]
    TauriError(#[from] tauri::Error),
}
