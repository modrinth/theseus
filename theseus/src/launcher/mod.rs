//! Logic for launching Minecraft
use crate::event::emit::{emit_loading, init_or_edit_loading};
use crate::event::{LoadingBarId, LoadingBarType};
use crate::jre::{self, JAVA_17_KEY, JAVA_18PLUS_KEY, JAVA_8_KEY};
use crate::launcher::io::IOError;
use crate::prelude::JavaVersion;
use crate::state::{Credentials, ProfileInstallStage};
use crate::util::io;
use crate::{
    process,
    state::{self as st, MinecraftChild},
    State,
};
use chrono::Utc;
use daedalus as d;
use daedalus::minecraft::{RuleAction, VersionInfo};
use st::Profile;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use uuid::Uuid;

mod args;

pub mod download;

// All nones -> disallowed
// 1+ true -> allowed
// 1+ false -> disallowed
#[tracing::instrument]
pub fn parse_rules(
    rules: &[d::minecraft::Rule],
    java_version: &str,
    minecraft_updated: bool,
) -> bool {
    let mut x = rules
        .iter()
        .map(|x| parse_rule(x, java_version, minecraft_updated))
        .collect::<Vec<Option<bool>>>();

    if rules
        .iter()
        .all(|x| matches!(x.action, RuleAction::Disallow))
    {
        x.push(Some(true))
    }

    !(x.iter().any(|x| x == &Some(false)) || x.iter().all(|x| x.is_none()))
}

// if anything is disallowed, it should NOT be included
// if anything is not disallowed, it shouldn't factor in final result
// if anything is not allowed, it shouldn't factor in final result
// if anything is allowed, it should be included
#[tracing::instrument]
pub fn parse_rule(
    rule: &d::minecraft::Rule,
    java_version: &str,
    minecraft_updated: bool,
) -> Option<bool> {
    use d::minecraft::{Rule, RuleAction};

    let res = match rule {
        Rule {
            os: Some(ref os), ..
        } => {
            crate::util::platform::os_rule(os, java_version, minecraft_updated)
        }
        Rule {
            features: Some(ref features),
            ..
        } => {
            !features.is_demo_user.unwrap_or(true)
                || features.has_custom_resolution.unwrap_or(false)
                || !features.has_quick_plays_support.unwrap_or(true)
                || !features.is_quick_play_multiplayer.unwrap_or(true)
                || !features.is_quick_play_realms.unwrap_or(true)
                || !features.is_quick_play_singleplayer.unwrap_or(true)
        }
        _ => return Some(true),
    };

    match rule.action {
        RuleAction::Allow => {
            if res {
                Some(true)
            } else {
                None
            }
        }
        RuleAction::Disallow => {
            if res {
                Some(false)
            } else {
                None
            }
        }
    }
}

macro_rules! processor_rules {
    ($dest:expr; $($name:literal : client => $client:expr, server => $server:expr;)+) => {
        $(std::collections::HashMap::insert(
            $dest,
            String::from($name),
            daedalus::modded::SidedDataEntry {
                client: String::from($client),
                server: String::from($server),
            },
        );)+
    }
}

pub async fn get_java_version_from_profile(
    profile: &Profile,
    version_info: &VersionInfo,
) -> crate::Result<Option<JavaVersion>> {
    if let Some(java) = profile.java.clone().and_then(|x| x.override_version) {
        Ok(Some(java))
    } else {
        let optimal_keys = match version_info
            .java_version
            .as_ref()
            .map(|it| it.major_version)
            .unwrap_or(8)
        {
            0..=15 => vec![JAVA_8_KEY, JAVA_17_KEY, JAVA_18PLUS_KEY],
            16..=17 => vec![JAVA_17_KEY, JAVA_18PLUS_KEY],
            _ => vec![JAVA_18PLUS_KEY],
        };

        let state = State::get().await?;
        let settings = state.settings.read().await;

        for key in optimal_keys {
            if let Some(java) = settings.java_globals.get(&key.to_string()) {
                return Ok(Some(java.clone()));
            }
        }

        Ok(None)
    }
}

#[tracing::instrument(skip(profile))]
#[theseus_macros::debug_pin]
pub async fn install_minecraft(
    profile: &Profile,
    existing_loading_bar: Option<LoadingBarId>,
    repairing: bool,
) -> crate::Result<()> {
    let sync_projects = existing_loading_bar.is_some();
    let loading_bar = init_or_edit_loading(
        existing_loading_bar,
        LoadingBarType::MinecraftDownload {
            // If we are downloading minecraft for a profile, provide its name and uuid
            profile_name: profile.metadata.name.clone(),
            profile_path: profile.get_profile_full_path().await?,
        },
        100.0,
        "Downloading Minecraft",
    )
    .await?;

    crate::api::profile::edit(&profile.profile_id(), |prof| {
        prof.install_stage = ProfileInstallStage::Installing;

        async { Ok(()) }
    })
    .await?;
    State::sync().await?;

    if sync_projects {
        Profile::sync_projects_task(profile.profile_id(), true);
    }

    let state = State::get().await?;
    let instance_path =
        &io::canonicalize(profile.get_profile_full_path().await?)?;
    let metadata = state.metadata.read().await;

    let version_index = metadata
        .minecraft
        .versions
        .iter()
        .position(|it| it.id == profile.metadata.game_version)
        .ok_or(crate::ErrorKind::LauncherError(format!(
            "Invalid game version: {}",
            profile.metadata.game_version
        )))?;
    let version = &metadata.minecraft.versions[version_index];
    let minecraft_updated = version_index
        <= metadata
            .minecraft
            .versions
            .iter()
            .position(|x| x.id == "22w16a")
            .unwrap_or(0);

    let version_jar = profile
        .metadata
        .loader_version
        .as_ref()
        .map_or(version.id.clone(), |it| {
            format!("{}-{}", version.id.clone(), it.id.clone())
        });

    // Download version info (5)
    let mut version_info = download::download_version_info(
        &state,
        version,
        profile.metadata.loader_version.as_ref(),
        Some(repairing),
        Some(&loading_bar),
    )
    .await?;

    let java_version = get_java_version_from_profile(profile, &version_info)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Missing correct java installation".to_string(),
            )
        })?;

    // Test jre version
    let java_version = jre::check_jre(java_version.path.clone().into())
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::LauncherError(format!(
                "Java path invalid or non-functional: {}",
                java_version.path
            ))
        })?;

    // Download minecraft (5-90)
    download::download_minecraft(
        &state,
        &version_info,
        &loading_bar,
        &java_version.architecture,
        repairing,
        minecraft_updated,
    )
    .await?;

    if let Some(processors) = &version_info.processors {
        let client_path = state
            .directories
            .version_dir(&version_jar)
            .await
            .join(format!("{version_jar}.jar"));

        let libraries_dir = state.directories.libraries_dir().await;

        if let Some(ref mut data) = version_info.data {
            processor_rules! {
                data;
                "SIDE":
                    client => "client",
                    server => "";
                "MINECRAFT_JAR" :
                    client => client_path.to_string_lossy(),
                    server => "";
                "MINECRAFT_VERSION":
                    client => profile.metadata.game_version.clone(),
                    server => "";
                "ROOT":
                    client => instance_path.to_string_lossy(),
                    server => "";
                "LIBRARY_DIR":
                    client => libraries_dir.to_string_lossy(),
                    server => "";
            }

            emit_loading(&loading_bar, 0.0, Some("Running forge processors"))
                .await?;
            let total_length = processors.len();

            // Forge processors (90-100)
            for (index, processor) in processors.iter().enumerate() {
                if let Some(sides) = &processor.sides {
                    if !sides.contains(&String::from("client")) {
                        continue;
                    }
                }

                let cp = wrap_ref_builder!(cp = processor.classpath.clone() => {
                    cp.push(processor.jar.clone())
                });

                let child = Command::new(&java_version.path)
                    .arg("-cp")
                    .arg(args::get_class_paths_jar(
                        &libraries_dir,
                        &cp,
                        &java_version.architecture,
                    )?)
                    .arg(
                        args::get_processor_main_class(args::get_lib_path(
                            &libraries_dir,
                            &processor.jar,
                            false,
                        )?)
                        .await?
                        .ok_or_else(|| {
                            crate::ErrorKind::LauncherError(format!(
                                "Could not find processor main class for {}",
                                processor.jar
                            ))
                        })?,
                    )
                    .args(args::get_processor_arguments(
                        &libraries_dir,
                        &processor.args,
                        data,
                    )?)
                    .output()
                    .await
                    .map_err(|e| IOError::with_path(e, &java_version.path))
                    .map_err(|err| {
                        crate::ErrorKind::LauncherError(format!(
                            "Error running processor: {err}",
                        ))
                    })?;

                if !child.status.success() {
                    return Err(crate::ErrorKind::LauncherError(format!(
                        "Processor error: {}",
                        String::from_utf8_lossy(&child.stderr)
                    ))
                    .as_error());
                }

                emit_loading(
                    &loading_bar,
                    30.0 / total_length as f64,
                    Some(&format!(
                        "Running forge processor {}/{}",
                        index, total_length
                    )),
                )
                .await?;
            }
        }
    }

    crate::api::profile::edit(&profile.profile_id(), |prof| {
        prof.install_stage = ProfileInstallStage::Installed;

        async { Ok(()) }
    })
    .await?;
    State::sync().await?;
    emit_loading(&loading_bar, 1.0, Some("Finished installing")).await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
#[theseus_macros::debug_pin]
#[allow(clippy::too_many_arguments)]
pub async fn launch_minecraft(
    java_args: &[String],
    env_args: &[(String, String)],
    mc_set_options: &[(String, String)],
    wrapper: &Option<String>,
    memory: &st::MemorySettings,
    resolution: &st::WindowSize,
    credentials: &Credentials,
    post_exit_hook: Option<String>,
    profile: &Profile,
) -> crate::Result<Arc<tokio::sync::RwLock<MinecraftChild>>> {
    if profile.install_stage == ProfileInstallStage::PackInstalling
        || profile.install_stage == ProfileInstallStage::Installing
    {
        return Err(crate::ErrorKind::LauncherError(
            "Profile is still installing".to_string(),
        )
        .into());
    }

    if profile.install_stage != ProfileInstallStage::Installed {
        install_minecraft(profile, None, false).await?;
    }

    let state = State::get().await?;
    let metadata = state.metadata.read().await;

    let instance_path = profile.get_profile_full_path().await?;
    let instance_path = &io::canonicalize(instance_path)?;

    let version_index = metadata
        .minecraft
        .versions
        .iter()
        .position(|it| it.id == profile.metadata.game_version)
        .ok_or(crate::ErrorKind::LauncherError(format!(
            "Invalid game version: {}",
            profile.metadata.game_version
        )))?;
    let version = &metadata.minecraft.versions[version_index];
    let minecraft_updated = version_index
        <= metadata
            .minecraft
            .versions
            .iter()
            .position(|x| x.id == "22w16a")
            .unwrap_or(0);

    let version_jar = profile
        .metadata
        .loader_version
        .as_ref()
        .map_or(version.id.clone(), |it| {
            format!("{}-{}", version.id.clone(), it.id.clone())
        });

    let version_info = download::download_version_info(
        &state,
        version,
        profile.metadata.loader_version.as_ref(),
        None,
        None,
    )
    .await?;

    let java_version = get_java_version_from_profile(profile, &version_info)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::LauncherError(
                "Missing correct java installation".to_string(),
            )
        })?;

    // Test jre version
    let java_version = jre::check_jre(java_version.path.clone().into())
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::LauncherError(format!(
                "Java path invalid or non-functional: {}",
                java_version.path
            ))
        })?;

    let client_path = state
        .directories
        .version_dir(&version_jar)
        .await
        .join(format!("{version_jar}.jar"));

    let args = version_info.arguments.clone().unwrap_or_default();
    let mut command = match wrapper {
        Some(hook) => {
            wrap_ref_builder!(it = Command::new(hook) => {it.arg(&java_version.path)})
        }
        None => Command::new(&java_version.path),
    };

    let env_args = Vec::from(env_args);

    // Check if profile has a running profile, and reject running the command if it does
    // Done late so a quick double call doesn't launch two instances
    let existing_processes =
        process::get_uuids_by_profile_path(profile.profile_id()).await?;
    if let Some(uuid) = existing_processes.first() {
        return Err(crate::ErrorKind::LauncherError(format!(
            "Profile {} is already running at UUID: {uuid}",
            profile.profile_id()
        ))
        .as_error());
    }
    command
        .args(
            args::get_jvm_arguments(
                args.get(&d::minecraft::ArgumentType::Jvm)
                    .map(|x| x.as_slice()),
                &state.directories.version_natives_dir(&version_jar).await,
                &state.directories.libraries_dir().await,
                &args::get_class_paths(
                    &state.directories.libraries_dir().await,
                    version_info.libraries.as_slice(),
                    &client_path,
                    &java_version.architecture,
                    minecraft_updated,
                )?,
                &version_jar,
                *memory,
                Vec::from(java_args),
                &java_version.architecture,
            )?
            .into_iter()
            .collect::<Vec<_>>(),
        )
        .arg(version_info.main_class.clone())
        .args(
            args::get_minecraft_arguments(
                args.get(&d::minecraft::ArgumentType::Game)
                    .map(|x| x.as_slice()),
                version_info.minecraft_arguments.as_deref(),
                credentials,
                &version.id,
                &version_info.asset_index.id,
                instance_path,
                &state.directories.assets_dir().await,
                &version.type_,
                *resolution,
                &java_version.architecture,
            )?
            .into_iter()
            .collect::<Vec<_>>(),
        )
        .current_dir(instance_path.clone());

    // CARGO-set DYLD_LIBRARY_PATH breaks Minecraft on macOS during testing on playground
    #[cfg(target_os = "macos")]
    if std::env::var("CARGO").is_ok() {
        command.env_remove("DYLD_FALLBACK_LIBRARY_PATH");
    }
    // Java options should be set in instance options (the existence of _JAVA_OPTIONS overwites them)
    command.env_remove("_JAVA_OPTIONS");

    command.envs(env_args);

    // Overwrites the minecraft options.txt file with the settings from the profile
    // Uses 'a:b' syntax which is not quite yaml
    use regex::Regex;

    if !mc_set_options.is_empty() {
        let options_path = instance_path.join("options.txt");
        let mut options_string = String::new();
        if options_path.exists() {
            options_string = io::read_to_string(&options_path).await?;
        }
        for (key, value) in mc_set_options {
            let re = Regex::new(&format!(r"(?m)^{}:.*$", regex::escape(key)))?;
            // check if the regex exists in the file
            if !re.is_match(&options_string) {
                // The key was not found in the file, so append it
                options_string.push_str(&format!("\n{}:{}", key, value));
            } else {
                let replaced_string = re
                    .replace_all(&options_string, &format!("{}:{}", key, value))
                    .to_string();
                options_string = replaced_string;
            }
        }

        io::write(&options_path, options_string).await?;
    }

    crate::api::profile::edit(&profile.profile_id(), |prof| {
        prof.metadata.last_played = Some(Utc::now());

        async { Ok(()) }
    })
    .await?;
    State::sync().await?;

    let mut censor_strings = HashMap::new();
    let username = whoami::username();
    censor_strings.insert(
        format!("/{}/", username),
        "/{COMPUTER_USERNAME}/".to_string(),
    );
    censor_strings.insert(
        format!("\\{}\\", username),
        "\\{COMPUTER_USERNAME}\\".to_string(),
    );
    censor_strings.insert(
        credentials.access_token.clone(),
        "{MINECRAFT_ACCESS_TOKEN}".to_string(),
    );
    censor_strings.insert(
        credentials.username.clone(),
        "{MINECRAFT_USERNAME}".to_string(),
    );
    censor_strings.insert(
        credentials.id.as_simple().to_string(),
        "{MINECRAFT_UUID}".to_string(),
    );
    censor_strings.insert(
        credentials.id.as_hyphenated().to_string(),
        "{MINECRAFT_UUID}".to_string(),
    );

    // If in tauri, and the 'minimize on launch' setting is enabled, minimize the window
    #[cfg(feature = "tauri")]
    {
        use crate::EventState;

        let window = EventState::get_main_window().await?;
        if let Some(window) = window {
            let settings = state.settings.read().await;
            if settings.hide_on_process {
                window.minimize()?;
            }
        }
    }

    if !*state.offline.read().await {
        // Add game played to discord rich presence
        let _ = state
            .discord_rpc
            .set_activity(&format!("Playing {}", profile.metadata.name), true)
            .await;
    }

    // Create Minecraft child by inserting it into the state
    // This also spawns the process and prepares the subsequent processes
    let mut state_children = state.children.write().await;
    state_children
        .insert_new_process(
            Uuid::new_v4(),
            profile.profile_id(),
            command,
            post_exit_hook,
            censor_strings,
        )
        .await
}
