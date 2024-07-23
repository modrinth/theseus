//! Theseus state management system
use crate::event::emit::{emit_loading, init_loading_unsafe};

use crate::event::LoadingBarType;

use crate::util::fetch::{FetchSemaphore, IoSemaphore};
use std::sync::Arc;
use tokio::sync::{OnceCell, Semaphore};

use crate::state::fs_watcher::FileWatcher;
use sqlx::SqlitePool;

// Submodules
mod dirs;
pub use self::dirs::*;

mod profiles;
pub use self::profiles::*;

mod settings;
pub use self::settings::*;

mod process;
pub use self::process::*;

mod java_globals;
pub use self::java_globals::*;

mod discord;
pub use self::discord::*;

mod minecraft_auth;
pub use self::minecraft_auth::*;

mod cache;
pub use self::cache::*;

mod db;
pub mod fs_watcher;
mod mr_auth;

pub use self::mr_auth::*;

// TODO: pass credentials to modrinth cdn
// TODO: fix empty teams not caching
// TODO: optimize file hashing
// TODO: make cache key / api requests ignore fetch semaphore (causes freezing in app)∑

// Global state
// RwLock on state only has concurrent reads, except for config dir change which takes control of the State
static LAUNCHER_STATE: OnceCell<Arc<State>> = OnceCell::const_new();
pub struct State {
    /// Information on the location of files used in the launcher
    pub directories: DirectoryInfo,

    /// Semaphore used to limit concurrent network requests and avoid errors
    pub fetch_semaphore: FetchSemaphore,
    /// Semaphore used to limit concurrent I/O and avoid errors
    pub io_semaphore: IoSemaphore,

    /// Discord RPC
    pub discord_rpc: DiscordGuard,

    pub(crate) pool: SqlitePool,

    pub(crate) file_watcher: FileWatcher,
}

impl State {
    pub async fn init() -> crate::Result<()> {
        let state = LAUNCHER_STATE
            .get_or_try_init(Self::initialize_state)
            .await?;

        Process::garbage_collect(&state.pool).await?;

        Ok(())
    }

    /// Get the current launcher state, waiting for initialization
    pub async fn get() -> crate::Result<Arc<Self>> {
        if !LAUNCHER_STATE.initialized() {
            while !LAUNCHER_STATE.initialized() {}
        }

        Ok(Arc::clone(
            LAUNCHER_STATE.get().expect("State is not initialized!"),
        ))
    }

    pub fn initialized() -> bool {
        LAUNCHER_STATE.initialized()
    }

    #[tracing::instrument]

    async fn initialize_state() -> crate::Result<Arc<Self>> {
        let loading_bar = init_loading_unsafe(
            LoadingBarType::StateInit,
            100.0,
            "Initializing launcher",
        )
        .await?;

        let directories = DirectoryInfo::init()?;

        let pool = db::connect().await?;

        let settings = Settings::get(&pool).await?;

        emit_loading(&loading_bar, 10.0, None).await?;

        let fetch_semaphore =
            FetchSemaphore(Semaphore::new(settings.max_concurrent_downloads));
        let io_semaphore =
            IoSemaphore(Semaphore::new(settings.max_concurrent_writes));
        emit_loading(&loading_bar, 10.0, None).await?;

        let discord_rpc = DiscordGuard::init().await?;
        if settings.discord_rpc {
            // Add default Idling to discord rich presence
            // Force add to avoid recursion
            let _ = discord_rpc.force_set_activity("Idling...", true).await;
        }

        let file_watcher = fs_watcher::init_watcher().await?;
        fs_watcher::watch_profiles_init(&file_watcher, &directories).await?;

        emit_loading(&loading_bar, 10.0, None).await?;

        Ok(Arc::new(Self {
            directories,
            fetch_semaphore,
            io_semaphore,
            discord_rpc,
            pool,
            file_watcher,
        }))
    }
}
