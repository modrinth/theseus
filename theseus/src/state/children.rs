use super::{Profile, ProfilePathId};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use std::process::Stdio;
use tokio::process::Child;
use tokio::process::Command;
use tokio::sync::RwLock;

use crate::event::emit::emit_process;
use crate::event::ProcessPayloadType;
use crate::util::fetch::read_json;
use crate::util::io::IOError;
use crate::{profile, ErrorKind};

use tokio::task::JoinHandle;
use uuid::Uuid;

const PROCESSES_JSON: &str = "processes.json";

// Child processes (instances of Minecraft)
// A wrapper over a Hashmap connecting PID -> MinecraftChild
pub struct Children(HashMap<Uuid, Arc<RwLock<MinecraftChild>>>);

#[derive(Debug)]
pub enum ChildType {
    // A child process that is being managed by tokio
    TokioChild(Child),
    // A child process that was rescued from a cache (e.g. a process that was launched by theseus before the launcher was restarted)
    // This may not have all the same functionality as a TokioChild
    RescuedPID(u32),
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessCache {
    pub pid: u32,
    pub uuid: Uuid,
    pub start_time: u64,
    pub name: String,
    pub exe: String,
    pub profile_relative_path: ProfilePathId,
    pub post_command: Option<String>,
}
impl ChildType {
    pub async fn try_wait(&mut self) -> crate::Result<Option<i32>> {
        match self {
            ChildType::TokioChild(child) => Ok(child
                .try_wait()
                .map_err(IOError::from)?
                .map(|x| x.code().unwrap_or(0))),
            ChildType::RescuedPID(pid) => {
                let mut system = sysinfo::System::new();
                if !system.refresh_process(sysinfo::Pid::from_u32(*pid)) {
                    return Ok(Some(0));
                }
                let process = system.process(sysinfo::Pid::from_u32(*pid));
                if let Some(process) = process {
                    if process.status() == sysinfo::ProcessStatus::Run {
                        Ok(None)
                    } else {
                        Ok(Some(0))
                    }
                } else {
                    Ok(Some(0))
                }
            }
        }
    }
    pub async fn kill(&mut self) -> crate::Result<()> {
        match self {
            ChildType::TokioChild(child) => {
                Ok(child.kill().await.map_err(IOError::from)?)
            }
            ChildType::RescuedPID(pid) => {
                let mut system = sysinfo::System::new();
                if system.refresh_process(sysinfo::Pid::from_u32(*pid)) {
                    let process = system.process(sysinfo::Pid::from_u32(*pid));
                    if let Some(process) = process {
                        process.kill();
                    }
                }
                Ok(())
            }
        }
    }
    pub fn id(&self) -> Option<u32> {
        match self {
            ChildType::TokioChild(child) => child.id(),
            ChildType::RescuedPID(pid) => Some(*pid),
        }
    }

    // Caches the process so that it can be restored if the launcher is restarted
    // Stored in the caches/metadata/processes.json file
    pub async fn cache_process(
        &self,
        uuid: uuid::Uuid,
        profile_path_id: ProfilePathId,
        post_command: Option<String>,
    ) -> crate::Result<()> {
        let pid = match self {
            ChildType::TokioChild(child) => child.id().unwrap_or(0),
            ChildType::RescuedPID(pid) => *pid,
        };

        let state = crate::State::get().await?;

        let mut system = sysinfo::System::new();
        system.refresh_processes();
        let process =
            system.process(sysinfo::Pid::from_u32(pid)).ok_or_else(|| {
                crate::ErrorKind::LauncherError(format!(
                    "Could not find process {}",
                    pid
                ))
            })?;
        let start_time = process.start_time();
        let name = process.name().to_string();

        let Some(path) = process.exe() else {
            return Err(ErrorKind::LauncherError(format!(
                "Cached process {} has no accessable path",
                pid
            ))
            .into());
        };

        let exe = path.to_string_lossy().to_string();

        let cached_process = ProcessCache {
            pid,
            start_time,
            name,
            exe,
            post_command,
            uuid,
            profile_relative_path: profile_path_id,
        };

        let children_path = state
            .directories
            .caches_meta_dir()
            .await
            .join(PROCESSES_JSON);
        let mut children_caches = if let Ok(children_json) =
            read_json::<HashMap<uuid::Uuid, ProcessCache>>(
                &children_path,
                &state.io_semaphore,
            )
            .await
        {
            children_json
        } else {
            HashMap::new()
        };
        children_caches.insert(uuid, cached_process);
        crate::util::fetch::write(
            &children_path,
            &serde_json::to_vec(&children_caches)?,
            &state.io_semaphore,
        )
        .await?;

        Ok(())
    }

    // Removes the process from the cache (ie: on process exit)
    pub async fn remove_cache(&self, uuid: uuid::Uuid) -> crate::Result<()> {
        let state = crate::State::get().await?;
        let children_path = state
            .directories
            .caches_meta_dir()
            .await
            .join(PROCESSES_JSON);
        let mut children_caches = if let Ok(children_json) =
            read_json::<HashMap<uuid::Uuid, ProcessCache>>(
                &children_path,
                &state.io_semaphore,
            )
            .await
        {
            children_json
        } else {
            HashMap::new()
        };
        children_caches.remove(&uuid);
        crate::util::fetch::write(
            &children_path,
            &serde_json::to_vec(&children_caches)?,
            &state.io_semaphore,
        )
        .await?;

        Ok(())
    }
}

// Minecraft Child, bundles together the PID, the actual Child, and the easily queryable stdout and stderr streams (if needed)
#[derive(Debug)]
pub struct MinecraftChild {
    pub uuid: Uuid,
    pub profile_relative_path: ProfilePathId,
    pub manager: Option<JoinHandle<crate::Result<i32>>>, // None when future has completed and been handled
    pub current_child: Arc<RwLock<ChildType>>,
    pub last_updated_playtime: DateTime<Utc>, // The last time we updated the playtime for the associated profile
}

impl Children {
    pub fn new() -> Self {
        Children(HashMap::new())
    }

    // Loads cached processes from the caches/metadata/processes.json file, re-inserts them into the hashmap, and removes them from the file
    // This will only be called once, on startup. Only processes who match a cached process (name, time started, pid, etc) will be re-inserted
    pub async fn rescue_cache(&mut self) -> crate::Result<()> {
        let state = crate::State::get().await?;
        let children_path = state
            .directories
            .caches_meta_dir()
            .await
            .join(PROCESSES_JSON);

        let mut children_caches = if let Ok(children_json) =
            read_json::<HashMap<uuid::Uuid, ProcessCache>>(
                &children_path,
                &state.io_semaphore,
            )
            .await
        {
            // Overwrite the file with an empty hashmap- we will re-insert the cached processes
            let empty = HashMap::<uuid::Uuid, ProcessCache>::new();
            crate::util::fetch::write(
                &children_path,
                &serde_json::to_vec(&empty)?,
                &state.io_semaphore,
            )
            .await?;

            // Return the cached processes
            children_json
        } else {
            HashMap::new()
        };

        for (_, cache) in children_caches.drain() {
            let uuid = cache.uuid;
            match self.insert_cached_process(cache).await {
                Ok(child) => {
                    self.0.insert(uuid, child);
                }
                Err(e) => tracing::warn!(
                    "Failed to rescue cached process {}: {}",
                    uuid,
                    e
                ),
            }
        }
        Ok(())
    }

    // Runs the command in process, inserts a child process to keep track of, and returns a reference to the container struct MinecraftChild
    // The threads for stdout and stderr are spawned here
    // Unlike a Hashmap's 'insert', this directly returns the reference to the MinecraftChild rather than any previously stored MinecraftChild that may exist
    #[tracing::instrument(skip(
        self,
        uuid,
        mc_command,
        post_command,
        censor_strings
    ))]
    #[tracing::instrument(level = "trace", skip(self))]
    #[theseus_macros::debug_pin]
    pub async fn insert_new_process(
        &mut self,
        uuid: Uuid,
        profile_relative_path: ProfilePathId,
        mut mc_command: Command,
        post_command: Option<String>, // Command to run after minecraft.
        censor_strings: HashMap<String, String>,
    ) -> crate::Result<Arc<RwLock<MinecraftChild>>> {
        // Takes the first element of the commands vector and spawns it
        let mc_proc = mc_command.stdout(Stdio::inherit())
            .stderr(Stdio::inherit())

            .spawn().map_err(IOError::from)?;

        let child = ChildType::TokioChild(mc_proc);

        // Slots child into manager
        let pid = child.id().ok_or_else(|| {
            crate::ErrorKind::LauncherError(
                "Process immediately failed, could not get PID".to_string(),
            )
        })?;

        // Caches process so that it can be restored if the launcher is restarted
        child
            .cache_process(
                uuid,
                profile_relative_path.clone(),
                post_command.clone(),
            )
            .await?;
        let current_child = Arc::new(RwLock::new(child));
        let manager = Some(tokio::spawn(Self::sequential_process_manager(
            uuid,
            post_command,
            pid,
            current_child.clone(),
            profile_relative_path.clone(),
        )));

        emit_process(
            uuid,
            pid,
            ProcessPayloadType::Launched,
            "Launched Minecraft",
        )
        .await?;

        let last_updated_playtime = Utc::now();

        // Create MinecraftChild
        let mchild = MinecraftChild {
            uuid,
            profile_relative_path,
            current_child,
            manager,
            last_updated_playtime,
        };

        let mchild = Arc::new(RwLock::new(mchild));
        self.0.insert(uuid, mchild.clone());
        Ok(mchild)
    }

    // Rescues a cached process, inserts a child process to keep track of, and returns a reference to the container struct MinecraftChild
    // Essentially 'reconnects' to a process that was launched by theseus before the launcher was restarted
    // However, this may not have all the same functionality as a TokioChild, as we only have the PID and not the actual Child
    // Only processes who match a cached process (name, time started, pid, etc) will be re-inserted. The function fails with an error if the process is notably different.
    #[tracing::instrument(skip(self, cached_process,))]
    #[tracing::instrument(level = "trace", skip(self))]
    #[theseus_macros::debug_pin]
    pub async fn insert_cached_process(
        &mut self,
        cached_process: ProcessCache,
    ) -> crate::Result<Arc<RwLock<MinecraftChild>>> {
        let _state = crate::State::get().await?;

        // Takes the first element of the commands vector and spawns it
        // Checks processes, compares cached process to actual process
        // Fails if notably different (meaning that the PID was reused, and we shouldn't reconnect to it)
        {
            let mut system = sysinfo::System::new();
            system.refresh_processes();
            let process = system
                .process(sysinfo::Pid::from_u32(cached_process.pid))
                .ok_or_else(|| {
                    crate::ErrorKind::LauncherError(format!(
                        "Could not find process {}",
                        cached_process.pid
                    ))
                })?;

            if cached_process.start_time != process.start_time() {
                return Err(ErrorKind::LauncherError(format!("Cached process {} has different start time than actual process {}", cached_process.pid, process.start_time())).into());
            }
            if cached_process.name != process.name() {
                return Err(ErrorKind::LauncherError(format!("Cached process {} has different name than actual process {}", cached_process.pid, process.name())).into());
            }
            if let Some(path) = process.exe() {
                if cached_process.exe != path.to_string_lossy() {
                    return Err(ErrorKind::LauncherError(format!("Cached process {} has different exe than actual process {}", cached_process.pid, path.to_string_lossy())).into());
                }
            } else {
                return Err(ErrorKind::LauncherError(format!(
                    "Cached process {} has no accessable path",
                    cached_process.pid
                ))
                .into());
            }
        }

        let child = ChildType::RescuedPID(cached_process.pid);

        // Slots child into manager
        let pid = child.id().ok_or_else(|| {
            crate::ErrorKind::LauncherError(
                "Process immediately failed, could not get PID".to_string(),
            )
        })?;

        // Re-caches process so that it can be restored if the launcher is restarted
        child
            .cache_process(
                cached_process.uuid,
                cached_process.profile_relative_path.clone(),
                cached_process.post_command.clone(),
            )
            .await?;

        let current_child = Arc::new(RwLock::new(child));
        let manager = Some(tokio::spawn(Self::sequential_process_manager(
            cached_process.uuid,
            cached_process.post_command,
            pid,
            current_child.clone(),
            cached_process.profile_relative_path.clone(),
        )));

        emit_process(
            cached_process.uuid,
            pid,
            ProcessPayloadType::Launched,
            "Launched Minecraft",
        )
        .await?;

        let last_updated_playtime = Utc::now();

        // Create MinecraftChild
        let mchild = MinecraftChild {
            uuid: cached_process.uuid,
            profile_relative_path: cached_process.profile_relative_path,
            current_child,
            manager,
            last_updated_playtime,
        };

        let mchild = Arc::new(RwLock::new(mchild));
        self.0.insert(cached_process.uuid, mchild.clone());
        Ok(mchild)
    }

    // Spawns a new child process and inserts it into the hashmap
    // Also, as the process ends, it spawns the follow-up process if it exists
    // By convention, ExitStatus is last command's exit status, and we exit on the first non-zero exit status
    #[tracing::instrument(skip(current_child))]
    #[theseus_macros::debug_pin]
    async fn sequential_process_manager(
        uuid: Uuid,
        post_command: Option<String>,
        mut current_pid: u32,
        current_child: Arc<RwLock<ChildType>>,
        associated_profile: ProfilePathId,
    ) -> crate::Result<i32> {
        let current_child = current_child.clone();

        // Wait on current Minecraft Child
        let mut mc_exit_status;
        let mut last_updated_playtime = Utc::now();
        loop {
            if let Some(t) = current_child.write().await.try_wait().await? {
                mc_exit_status = t;
                break;
            }
            // sleep for 10ms
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // Auto-update playtime every minute
            let diff = Utc::now()
                .signed_duration_since(last_updated_playtime)
                .num_seconds();
            if diff >= 60 {
                if let Err(e) = profile::edit(&associated_profile, |prof| {
                    prof.metadata.recent_time_played += diff as u64;
                    async { Ok(()) }
                })
                .await
                {
                    tracing::warn!(
                        "Failed to update playtime for profile {}: {}",
                        &associated_profile,
                        e
                    );
                }
                last_updated_playtime = Utc::now();
            }
        }

        // Now fully complete- update playtime one last time
        let diff = Utc::now()
            .signed_duration_since(last_updated_playtime)
            .num_seconds();
        if let Err(e) = profile::edit(&associated_profile, |prof| {
            prof.metadata.recent_time_played += diff as u64;
            async { Ok(()) }
        })
        .await
        {
            tracing::warn!(
                "Failed to update playtime for profile {}: {}",
                &associated_profile,
                e
            );
        }

        // Publish play time update
        // Allow failure, it will be stored locally and sent next time
        // Sent in another thread as first call may take a couple seconds and hold up process ending
        let associated_profile_clone = associated_profile.clone();
        tokio::spawn(async move {
            if let Err(e) =
                profile::try_update_playtime(&associated_profile_clone.clone())
                    .await
            {
                tracing::warn!(
                    "Failed to update playtime for profile {}: {}",
                    &associated_profile_clone,
                    e
                );
            }
        });

        {
            // Clear game played for Discord RPC
            // May have other active processes, so we clear to the next running process
            let state = crate::State::get().await?;
            let _ = state.discord_rpc.clear_to_default(true).await;
        }

        // If in tauri, window should show itself again after process exists if it was hidden
        #[cfg(feature = "tauri")]
        {
            let window = crate::EventState::get_main_window().await?;
            if let Some(window) = window {
                window.unminimize()?;
            }
        }

        {
            let current_child = current_child.write().await;
            current_child.remove_cache(uuid).await?;
        }

        if !mc_exit_status == 0 {
            emit_process(
                uuid,
                current_pid,
                ProcessPayloadType::Finished,
                "Exited process",
            )
            .await?;

            return Ok(mc_exit_status); // Err for a non-zero exit is handled in helper
        }

        // If a post-command exist, switch to it and wait on it
        // First, create the command by splitting arguments
        let post_command = if let Some(hook) = post_command {
            let mut cmd = hook.split(' ');
            if let Some(command) = cmd.next() {
                let mut command = Command::new(command);
                command
                    .args(&cmd.collect::<Vec<&str>>())
                    .current_dir(associated_profile.get_full_path().await?);
                Some(command)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(mut m_command) = post_command {
            {
                let mut current_child: tokio::sync::RwLockWriteGuard<
                    '_,
                    ChildType,
                > = current_child.write().await;
                let new_child = m_command.spawn().map_err(IOError::from)?;
                current_pid = new_child.id().ok_or_else(|| {
                    crate::ErrorKind::LauncherError(
                        "Process immediately failed, could not get PID"
                            .to_string(),
                    )
                })?;
                *current_child = ChildType::TokioChild(new_child);
            }
            emit_process(
                uuid,
                current_pid,
                ProcessPayloadType::Updated,
                "Completed Minecraft, switching to post-commands",
            )
            .await?;

            loop {
                if let Some(t) = current_child.write().await.try_wait().await? {
                    mc_exit_status = t;
                    break;
                }
                // sleep for 10ms
                tokio::time::sleep(tokio::time::Duration::from_millis(10))
                    .await;
            }
        }

        emit_process(
            uuid,
            current_pid,
            ProcessPayloadType::Finished,
            "Exited process",
        )
        .await?;

        Ok(mc_exit_status)
    }

    // Returns a ref to the child
    pub fn get(&self, uuid: Uuid) -> Option<Arc<RwLock<MinecraftChild>>> {
        self.0.get(&uuid).cloned()
    }

    // Gets all PID keys
    pub fn keys(&self) -> Vec<Uuid> {
        self.0.keys().cloned().collect()
    }

    // Get exit status of a child by PID
    // Returns None if the child is still running
    pub async fn exit_status(&self, uuid: Uuid) -> crate::Result<Option<i32>> {
        if let Some(child) = self.get(uuid) {
            let child = child.write().await;
            let status = child.current_child.write().await.try_wait().await?;
            Ok(status)
        } else {
            Ok(None)
        }
    }

    // Gets all PID keys of running children
    pub async fn running_keys(&self) -> crate::Result<Vec<Uuid>> {
        let mut keys = Vec::new();
        for key in self.keys() {
            if let Some(child) = self.get(key) {
                let child = child.clone();
                let child = child.write().await;
                if child
                    .current_child
                    .write()
                    .await
                    .try_wait()
                    .await?
                    .is_none()
                {
                    keys.push(key);
                }
            }
        }
        Ok(keys)
    }

    // Gets all PID keys of running children with a given profile path
    pub async fn running_keys_with_profile(
        &self,
        profile_path: ProfilePathId,
    ) -> crate::Result<Vec<Uuid>> {
        let running_keys = self.running_keys().await?;
        let mut keys = Vec::new();
        for key in running_keys {
            if let Some(child) = self.get(key) {
                let child = child.clone();
                let child = child.read().await;
                if child.profile_relative_path == profile_path {
                    keys.push(key);
                }
            }
        }
        Ok(keys)
    }

    // Gets all profiles of running children
    pub async fn running_profile_paths(
        &self,
    ) -> crate::Result<Vec<ProfilePathId>> {
        let mut profiles = Vec::new();
        for key in self.keys() {
            if let Some(child) = self.get(key) {
                let child = child.clone();
                let child = child.write().await;
                if child
                    .current_child
                    .write()
                    .await
                    .try_wait()
                    .await?
                    .is_none()
                {
                    profiles.push(child.profile_relative_path.clone());
                }
            }
        }
        Ok(profiles)
    }

    // Gets all profiles of running children
    // Returns clones because it would be serialized anyway
    pub async fn running_profiles(&self) -> crate::Result<Vec<Profile>> {
        let mut profiles = Vec::new();
        for key in self.keys() {
            if let Some(child) = self.get(key) {
                let child = child.clone();
                let child = child.write().await;
                if child
                    .current_child
                    .write()
                    .await
                    .try_wait()
                    .await?
                    .is_none()
                {
                    if let Some(prof) = crate::api::profile::get(
                        &child.profile_relative_path.clone(),
                        None,
                    )
                    .await?
                    {
                        profiles.push(prof);
                    }
                }
            }
        }
        Ok(profiles)
    }
}

impl Default for Children {
    fn default() -> Self {
        Self::new()
    }
}
