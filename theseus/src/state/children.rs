use super::Profile;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::{collections::HashMap, sync::Arc};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::process::Command;
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use uuid::Uuid;

// Child processes (instances of Minecraft)
// A wrapper over a Hashmap connecting PID -> MinecraftChild
pub struct Children(HashMap<Uuid, Arc<RwLock<MinecraftChild>>>);

// Minecraft Child, bundles together the PID, the actual Child, and the easily queryable stdout and stderr streams
#[derive(Debug)]
pub struct MinecraftChild {
    pub profile_path: PathBuf, //todo: make UUID when profiles are recognized by UUID
    pub uuid: Uuid,
    pub manager: Option<JoinHandle<crate::Result<ExitStatus>>>, // None when future has completed and been handled
    pub current_child: Arc<RwLock<Child>>,
    pub stdout: SharedOutput,
    pub stderr: SharedOutput,
}

impl Children {
    pub fn new() -> Children {
        Children(HashMap::new())
    }

    // Runs the command in process, inserts a child process to keep track of, and returns a reference to the container struct MinecraftChild
    // The threads for stdout and stderr are spawned here
    // Unlike a Hashmap's 'insert', this directly returns the reference to the obj rather than any previously stored obj that may exist
    pub fn insert_process(
        &mut self,
        uuid: Uuid,
        profile_path: PathBuf,
        mut mc_command: Command,
        post_command: Option<Command>, // Commands to run after minecraft. It's plural in case we want to run more than one command in the future
    ) -> crate::Result<Arc<RwLock<MinecraftChild>>> {
        // Takes the first element of the commands vector and spawns it
        let mut child = mc_command.spawn()?;

        // Create std watcher threads for stdout and stderr
        let stdout = SharedOutput::new();
        if let Some(child_stdout) = child.stdout.take() {
            let stdout_clone = stdout.clone();
            tokio::spawn(async move {
                if let Err(e) = stdout_clone.read_stdout(child_stdout).await {
                    eprintln!("Stdout process died with error: {}", e);
                }
            });
        }
        let stderr = SharedOutput::new();
        if let Some(child_stderr) = child.stderr.take() {
            let stderr_clone = stderr.clone();
            tokio::spawn(async move {
                if let Err(e) = stderr_clone.read_stderr(child_stderr).await {
                    eprintln!("Stderr process died with error: {}", e);
                }
            });
        }

        // Slots child into manager
        let current_child = Arc::new(RwLock::new(child));
        let manager = Some(tokio::spawn(Self::sequential_process_manager(
            vec![post_command].into_iter().flatten().collect(), // left as a vector in case we want to add more commands in the future
            current_child.clone(),
        )));

        // Create MinecraftChild
        let mchild = MinecraftChild {
            uuid,
            profile_path,
            current_child,
            stdout,
            stderr,
            manager,
        };

        let mchild = Arc::new(RwLock::new(mchild));
        self.0.insert(uuid, mchild.clone());
        Ok(mchild)
    }

    // Spawns a new child process and inserts it into the hashmap
    // Also, as the process ends, it spawns the next command in the vector (hooked post-minecraft functions)
    // By convention, ExitStatus is last command's exit status, and we exit on the first non-zero exit status
    async fn sequential_process_manager(
        minecraft_commands: Vec<Command>,
        current_child: Arc<RwLock<Child>>,
    ) -> crate::Result<ExitStatus> {
        let current_child = current_child.clone();
        let mut mc_exit_status;
        loop {
            if let Some(t) = current_child.write().await.try_wait()? {
                mc_exit_status = t;
                break;
            }
        }
        if !mc_exit_status.success() {
            return Ok(mc_exit_status); // Err for a non-zero exit is handled in helper
        }
        // Now, similarly, spawn and use each subsequent command process to create after minecraft exits
        for mut m_command in minecraft_commands.into_iter() {
            {
                let mut current_child = current_child.write().await;
                let new_child = m_command.spawn()?;
                *current_child = new_child;
            }
            loop {
                if let Some(t) = current_child.write().await.try_wait()? {
                    mc_exit_status = t;
                    break;
                }
            }
            if !mc_exit_status.success() {
                return Ok(mc_exit_status); // Err for a non-zero exit is handled in helper
            }
        }
        Ok(mc_exit_status)
    }

    // Returns a ref to the child
    pub fn get(&self, uuid: &Uuid) -> Option<Arc<RwLock<MinecraftChild>>> {
        self.0.get(uuid).cloned()
    }

    // Gets all PID keys
    pub fn keys(&self) -> Vec<Uuid> {
        self.0.keys().cloned().collect()
    }

    // Get exit status of a child by PID
    // Returns None if the child is still running
    pub async fn exit_status(
        &self,
        uuid: &Uuid,
    ) -> crate::Result<Option<std::process::ExitStatus>> {
        if let Some(child) = self.get(uuid) {
            let child = child.write().await;
            let status = child.current_child.write().await.try_wait()?;
            Ok(status)
        } else {
            Ok(None)
        }
    }

    // Gets all PID keys of running children
    pub async fn running_keys(&self) -> crate::Result<Vec<Uuid>> {
        let mut keys = Vec::new();
        for key in self.keys() {
            if let Some(child) = self.get(&key) {
                let child = child.clone();
                let child = child.write().await;
                if child.current_child.write().await.try_wait()?.is_none() {
                    keys.push(key);
                }
            }
        }
        Ok(keys)
    }

    // Gets all PID keys of running children with a given profile path
    pub async fn running_keys_with_profile(
        &self,
        profile_path: &Path,
    ) -> crate::Result<Vec<Uuid>> {
        let running_keys = self.running_keys().await?;
        let mut keys = Vec::new();
        for key in running_keys {
            if let Some(child) = self.get(&key) {
                let child = child.clone();
                let child = child.read().await;
                if child.profile_path == profile_path {
                    keys.push(key);
                }
            }
        }
        Ok(keys)
    }

    // Gets all profiles of running children
    pub async fn running_profile_paths(&self) -> crate::Result<Vec<PathBuf>> {
        let mut profiles = Vec::new();
        for key in self.keys() {
            if let Some(child) = self.get(&key) {
                let child = child.clone();
                let child = child.write().await;
                if child.current_child.write().await.try_wait()?.is_none() {
                    profiles.push(child.profile_path.clone());
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
            if let Some(child) = self.get(&key) {
                let child = child.clone();
                let child = child.write().await;
                if child.current_child.write().await.try_wait()?.is_none() {
                    if let Some(prof) =
                        crate::api::profile::get(&child.profile_path.clone())
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

// SharedOutput, a wrapper around a String that can be read from and written to concurrently
// Designed to be used with ChildStdout and ChildStderr in a tokio thread to have a simple String storage for the output of a child process
#[derive(Clone, Debug)]
pub struct SharedOutput {
    output: Arc<RwLock<String>>,
}

impl SharedOutput {
    fn new() -> Self {
        SharedOutput {
            output: Arc::new(RwLock::new(String::new())),
        }
    }

    // Main entry function to a created SharedOutput, returns the log as a String
    pub async fn get_output(&self) -> crate::Result<String> {
        let output = self.output.read().await;
        Ok(output.clone())
    }

    async fn read_stdout(
        &self,
        child_stdout: ChildStdout,
    ) -> crate::Result<()> {
        let mut buf_reader = BufReader::new(child_stdout);
        let mut line = String::new();

        while buf_reader.read_line(&mut line).await? > 0 {
            {
                let mut output = self.output.write().await;
                output.push_str(&line);
            }
            line.clear();
        }
        Ok(())
    }

    async fn read_stderr(
        &self,
        child_stderr: ChildStderr,
    ) -> crate::Result<()> {
        let mut buf_reader = BufReader::new(child_stderr);
        let mut line = String::new();

        while buf_reader.read_line(&mut line).await? > 0 {
            {
                let mut output = self.output.write().await;
                output.push_str(&line);
            }
            line.clear();
        }
        Ok(())
    }
}
