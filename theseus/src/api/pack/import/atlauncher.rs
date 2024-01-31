use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    pack::{
        self,
        import::{self, copy_dotminecraft},
        install_from::CreatePackDescription,
    },
    prelude::{ModLoader, Profile, ProfilePathId},
    state::{LinkedData, ProfileInstallStage},
    util::io,
    State,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ATInstance {
    pub id: String, // minecraft version id ie: 1.12.1, not a name
    pub launcher: ATLauncher,
    pub java_version: ATJavaVersion,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ATLauncher {
    pub name: String,
    pub pack: String,
    pub version: String, // ie: 1.6
    pub loader_version: ATLauncherLoaderVersion,

    pub modrinth_project: Option<ATLauncherModrinthProject>,
    pub modrinth_version: Option<ATLauncherModrinthVersion>,
    pub modrinth_manifest: Option<pack::install_from::PackFormat>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ATJavaVersion {
    pub major_version: u8,
    pub component: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ATLauncherLoaderVersion {
    pub r#type: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ATLauncherModrinthProject {
    pub id: String,
    pub slug: String,
    pub project_type: String,
    pub team: String,
    pub client_side: Option<String>,
    pub server_side: Option<String>,
    pub categories: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ATLauncherModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ATLauncherModrinthVersionFile {
    pub hashes: HashMap<String, String>,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ATLauncherModrinthVersionDependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ATLauncherMod {
    pub name: String,
    pub version: String,
    pub file: String,

    pub modrinth_project: Option<ATLauncherModrinthProject>,
    pub modrinth_version: Option<ATLauncherModrinthVersion>,
}

// Check if folder has a instance.json that parses
pub async fn is_valid_atlauncher(instance_folder: PathBuf) -> bool {
    let instance: String =
        io::read_to_string(&instance_folder.join("instance.json"))
            .await
            .unwrap_or("".to_string());
    let instance: Result<ATInstance, serde_json::Error> =
        serde_json::from_str::<ATInstance>(&instance);
    if let Err(e) = instance {
        tracing::warn!(
            "Could not parse instance.json at {}: {}",
            instance_folder.display(),
            e
        );
        false
    } else {
        true
    }
}

#[tracing::instrument]
#[theseus_macros::debug_pin]
pub async fn import_atlauncher(
    atlauncher_base_path: PathBuf, // path to base atlauncher folder
    instance_folder: String,       // instance folder in atlauncher_base_path
    profile_path: ProfilePathId,   // path to profile
) -> crate::Result<()> {
    let atlauncher_instance_path = atlauncher_base_path
        .join("instances")
        .join(instance_folder.clone());

    // Load instance.json
    let atinstance: String =
        io::read_to_string(&atlauncher_instance_path.join("instance.json"))
            .await?;
    let atinstance: ATInstance =
        serde_json::from_str::<ATInstance>(&atinstance)?;

    // Icon path should be {instance_folder}/instance.png if it exists,
    // Second possibility is ATLauncher/configs/images/{safe_pack_name}.png (safe pack name is alphanumeric lowercase)
    let icon_path_primary = atlauncher_instance_path.join("instance.png");
    let safe_pack_name = atinstance
        .launcher
        .pack
        .replace(|c: char| !c.is_alphanumeric(), "")
        .to_lowercase();
    let icon_path_secondary = atlauncher_base_path
        .join("configs")
        .join("images")
        .join(safe_pack_name + ".png");
    let icon = match (icon_path_primary.exists(), icon_path_secondary.exists())
    {
        (true, _) => import::recache_icon(icon_path_primary).await?,
        (_, true) => import::recache_icon(icon_path_secondary).await?,
        _ => None,
    };

    // Create description from instance.cfg
    let description = CreatePackDescription {
        icon,
        override_title: Some(atinstance.launcher.name.clone()),
        project_id: None,
        version_id: None,
        existing_loading_bar: None,
        profile_path: profile_path.clone(),
    };

    let backup_name = format!("ATLauncher-{}", instance_folder);
    let minecraft_folder = atlauncher_instance_path;

    import_atlauncher_unmanaged(
        profile_path,
        minecraft_folder,
        backup_name,
        description,
        atinstance,
    )
    .await?;
    Ok(())
}

async fn import_atlauncher_unmanaged(
    profile_path: ProfilePathId,
    minecraft_folder: PathBuf,
    backup_name: String,
    description: CreatePackDescription,
    atinstance: ATInstance,
) -> crate::Result<()> {
    let mod_loader = format!(
        "\"{}\"",
        atinstance.launcher.loader_version.r#type.to_lowercase()
    );
    let mod_loader: ModLoader = serde_json::from_str::<ModLoader>(&mod_loader)
        .map_err(|_| {
            crate::ErrorKind::InputError(format!(
                "Could not parse mod loader type: {}",
                mod_loader
            ))
        })?;

    let game_version = atinstance.id;

    let loader_version = if mod_loader != ModLoader::Vanilla {
        crate::profile::create::get_loader_version_from_loader(
            game_version.clone(),
            mod_loader,
            Some(atinstance.launcher.loader_version.version.clone()),
        )
        .await?
    } else {
        None
    };

    // Set profile data to created default profile
    crate::api::profile::edit(&profile_path, |prof| {
        prof.metadata.name = description
            .override_title
            .clone()
            .unwrap_or_else(|| backup_name.to_string());
        prof.install_stage = ProfileInstallStage::PackInstalling;
        prof.metadata.linked_data = Some(LinkedData::ModrinthModpack {
            project_id: description.project_id.clone(),
            version_id: description.version_id.clone(),
            locked: Some(
                description.project_id.is_some()
                    && description.version_id.is_some(),
            ),
        });
        prof.metadata.icon = description.icon.clone();
        prof.metadata.game_version = game_version.clone();
        prof.metadata.loader_version = loader_version.clone();
        prof.metadata.loader = mod_loader;

        async { Ok(()) }
    })
    .await?;

    // Moves .minecraft folder over (ie: overrides such as resourcepacks, mods, etc)
    let state = State::get().await?;
    let loading_bar = copy_dotminecraft(
        profile_path.clone(),
        minecraft_folder,
        &state.io_semaphore,
        None,
    )
    .await?;

    if let Some(profile_val) =
        crate::api::profile::get(&profile_path, None).await?
    {
        crate::launcher::install_minecraft(
            &profile_val,
            Some(loading_bar),
            false,
        )
        .await?;
        {
            let state = State::get().await?;
            let mut file_watcher = state.file_watcher.write().await;
            Profile::watch_fs(
                &profile_val.get_profile_full_path().await?,
                &mut file_watcher,
            )
            .await?;
        }
        State::sync().await?;
    }
    Ok(())
}
