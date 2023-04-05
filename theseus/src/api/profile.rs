//! Theseus profile management interface
pub use crate::{
    state::{JavaSettings, Profile},
    State,
};
use daedalus as d;
use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    process::{Child, Command},
    sync::RwLock,
};

/// Remove a profile
#[tracing::instrument]
pub async fn remove(path: &Path) -> crate::Result<()> {
    let state = State::get().await?;
    let mut profiles = state.profiles.write().await;
    profiles.remove(path).await?;

    Ok(())
}

/// Get a profile by path,
#[tracing::instrument]
pub async fn get(path: &Path) -> crate::Result<Option<Profile>> {
    let state = State::get().await?;
    let profiles = state.profiles.read().await;

    Ok(profiles.0.get(path).cloned())
}

/// Edit a profile using a given asynchronous closure
pub async fn edit<Fut>(
    path: &Path,
    action: impl Fn(&mut Profile) -> Fut,
) -> crate::Result<()>
where
    Fut: Future<Output = crate::Result<()>>,
{
    let state = State::get().await?;
    let mut profiles = state.profiles.write().await;

    match profiles.0.get_mut(path) {
        Some(ref mut profile) => action(profile).await,
        None => Err(crate::ErrorKind::UnmanagedProfileError(
            path.display().to_string(),
        )
        .as_error()),
    }
}

/// Get a copy of the profile set
#[tracing::instrument]
pub async fn list() -> crate::Result<std::collections::HashMap<PathBuf, Profile>>
{
    let state = State::get().await?;
    let profiles = state.profiles.read().await;
    Ok(profiles.0.clone())
}

/// Query + sync profile's projects with the UI from the FS
#[tracing::instrument]
pub async fn sync(path: &Path) -> crate::Result<()> {
    let state = State::get().await?;

    let mut profiles = state.profiles.write().await;

    if let Some(profile) = profiles.0.get_mut(path) {
        let paths = profile.get_profile_project_paths()?;
        let projects = crate::state::infer_data_from_files(
            paths,
            state.directories.caches_dir(),
            &state.io_semaphore,
        )
        .await?;

        profile.projects = projects;
        State::sync().await?;

        Ok(())
    } else {
        Err(
            crate::ErrorKind::UnmanagedProfileError(path.display().to_string())
                .as_error(),
        )
    }
}

/// Run Minecraft using a profile
/// Returns Arc pointer to RwLock to Child
#[tracing::instrument(skip_all)]
pub async fn run(
    path: &Path,
    credentials: &crate::auth::Credentials,
) -> crate::Result<Arc<RwLock<Child>>> {
    let state = State::get().await?;
    let settings = state.settings.read().await;
    let profile = get(path).await?.ok_or_else(|| {
        crate::ErrorKind::OtherError(format!(
            "Tried to run a nonexistent or unloaded profile at path {}!",
            path.display()
        ))
    })?;

    let version = state
        .metadata
        .minecraft
        .versions
        .iter()
        .find(|it| it.id == profile.metadata.game_version.as_ref())
        .ok_or_else(|| {
            crate::ErrorKind::LauncherError(format!(
                "Invalid or unknown Minecraft version: {}",
                profile.metadata.game_version
            ))
        })?;
    let version_info = d::minecraft::fetch_version_info(version).await?;

    let pre_launch_hooks =
        &profile.hooks.as_ref().unwrap_or(&settings.hooks).pre_launch;
    for hook in pre_launch_hooks.iter() {
        // TODO: hook parameters
        let mut cmd = hook.split(' ');
        if let Some(command) = cmd.next() {
            let result = Command::new(command)
                .args(&cmd.collect::<Vec<&str>>())
                .current_dir(path)
                .spawn()?
                .wait()
                .await?;

            if !result.success() {
                return Err(crate::ErrorKind::LauncherError(format!(
                    "Non-zero exit code for pre-launch hook: {}",
                    result.code().unwrap_or(-1)
                ))
                .as_error());
            }
        }
    }

    let java_install = match profile.java {
        Some(JavaSettings {
            install: Some(ref install),
            ..
        }) => install,
        _ => if version_info
            .java_version
            .as_ref()
            .filter(|it| it.major_version >= 16)
            .is_some()
        {
            settings.java_17_path.as_ref()
        } else {
            settings.java_8_path.as_ref()
        }
        .ok_or_else(|| {
            crate::ErrorKind::LauncherError(format!(
                "No Java installed for version {}",
                version_info.java_version.map_or(8, |it| it.major_version),
            ))
        })?,
    };

    if !java_install.exists() {
        return Err(crate::ErrorKind::LauncherError(format!(
            "Could not find Java install: {}",
            java_install.display()
        ))
        .as_error());
    }

    let java_args = profile
        .java
        .as_ref()
        .and_then(|it| it.extra_arguments.as_ref())
        .unwrap_or(&settings.custom_java_args);

    let wrapper = profile
        .hooks
        .as_ref()
        .map_or(&settings.hooks.wrapper, |it| &it.wrapper);

    let memory = profile.memory.unwrap_or(settings.memory);
    let resolution = profile.resolution.unwrap_or(settings.game_resolution);

    let mc_process = crate::launcher::launch_minecraft(
        &profile.metadata.game_version,
        &profile.metadata.loader_version,
        &profile.path,
        java_install,
        java_args,
        wrapper,
        &memory,
        &resolution,
        credentials,
    )
    .await?;

    // Insert child into state
    let mut state_children = state.children.write().await;
    let pid = mc_process.id().ok_or_else(|| {
        crate::ErrorKind::LauncherError(
            "Process failed to stay open.".to_string(),
        )
    })?;
    let child_arc = state_children.insert(pid, mc_process);

    Ok(child_arc)
}

#[tracing::instrument]
pub async fn kill(running: &mut Child) -> crate::Result<()> {
    running.kill().await?;
    wait_for(running).await
}

#[tracing::instrument]
pub async fn wait_for(running: &mut Child) -> crate::Result<()> {
    let result = running.wait().await.map_err(|err| {
        crate::ErrorKind::LauncherError(format!(
            "Error running minecraft: {err}"
        ))
    })?;

    match result.success() {
        false => Err(crate::ErrorKind::LauncherError(format!(
            "Minecraft exited with non-zero code {}",
            result.code().unwrap_or(-1)
        ))
        .as_error()),
        true => Ok(()),
    }
}
