use crate::config::MODRINTH_API_URL;
use crate::data::ModLoader;
use crate::event::emit::{emit_loading, init_loading};
use crate::event::{LoadingBarId, LoadingBarType};
use crate::prelude::ProfilePathId;
use crate::state::{
    LinkedData, ModrinthProject, ModrinthVersion, ProfileInstallStage, SideType,
};
use crate::util::fetch::{
    fetch, fetch_advanced, fetch_json, write_cached_icon,
};
use crate::util::io;
use crate::{InnerProjectPathUnix, State};

use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::path::PathBuf;

#[derive(Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackFormat {
    pub game: String,
    pub format_version: i32,
    pub version_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub files: Vec<PackFile>,
    pub dependencies: HashMap<PackDependency, String>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    pub path: InnerProjectPathUnix,
    pub hashes: HashMap<PackFileHash, String>,
    pub env: Option<HashMap<EnvType, SideType>>,
    pub downloads: Vec<String>,
    pub file_size: u32,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase", from = "String")]
pub enum PackFileHash {
    Sha1,
    Sha512,
    Unknown(String),
}

impl From<String> for PackFileHash {
    fn from(s: String) -> Self {
        return match s.as_str() {
            "sha1" => PackFileHash::Sha1,
            "sha512" => PackFileHash::Sha512,
            _ => PackFileHash::Unknown(s),
        };
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum EnvType {
    Client,
    Server,
}

#[derive(Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum PackDependency {
    #[serde(rename = "forge")]
    Forge,

    #[serde(rename = "neoforge")]
    #[serde(alias = "neo-forge")]
    NeoForge,

    #[serde(rename = "fabric-loader")]
    FabricLoader,

    #[serde(rename = "quilt-loader")]
    QuiltLoader,

    #[serde(rename = "minecraft")]
    Minecraft,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum CreatePackLocation {
    // Create a pack from a modrinth version ID (such as a modpack)
    FromVersionId {
        project_id: String,
        version_id: String,
        title: String,
        icon_url: Option<String>,
    },
    // Create a pack from a file (such as an .mrpack for installing from a file, or a folder name for importing)
    FromFile {
        path: PathBuf,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePackProfile {
    pub name: String, // the name of the profile, and relative path
    pub game_version: String, // the game version of the profile
    pub modloader: ModLoader, // the modloader to use
    pub loader_version: Option<String>, // the modloader version to use, set to "latest", "stable", or the ID of your chosen loader. defaults to latest
    pub icon: Option<PathBuf>,          // the icon for the profile
    pub icon_url: Option<String>, // the URL icon for a profile (ONLY USED FOR TEMPORARY PROFILES)
    pub linked_data: Option<LinkedData>, // the linked project ID (mainly for modpacks)- used for updating
    pub skip_install_profile: Option<bool>,
    pub no_watch: Option<bool>,
}

// default
impl Default for CreatePackProfile {
    fn default() -> Self {
        CreatePackProfile {
            name: "Untitled".to_string(),
            game_version: "1.19.4".to_string(),
            modloader: ModLoader::Vanilla,
            loader_version: None,
            icon: None,
            icon_url: None,
            linked_data: None,
            skip_install_profile: Some(true),
            no_watch: Some(false),
        }
    }
}

#[derive(Clone)]
pub struct CreatePack {
    pub file: bytes::Bytes,
    pub description: CreatePackDescription,
}

#[derive(Clone, Debug)]
pub struct CreatePackDescription {
    pub icon: Option<PathBuf>,
    pub override_title: Option<String>,
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub existing_loading_bar: Option<LoadingBarId>,
    pub profile_path: ProfilePathId,
}

pub fn get_profile_from_pack(
    location: CreatePackLocation,
) -> CreatePackProfile {
    match location {
        CreatePackLocation::FromVersionId {
            project_id,
            version_id,
            title,
            icon_url,
        } => CreatePackProfile {
            name: title,
            icon_url,
            linked_data: Some(LinkedData::ModrinthModpack {
                project_id: Some(project_id),
                version_id: Some(version_id),
                locked: Some(true),
            }),
            ..Default::default()
        },
        CreatePackLocation::FromFile { path } => {
            let file_name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            CreatePackProfile {
                name: file_name,
                ..Default::default()
            }
        }
    }
}

#[tracing::instrument]
#[theseus_macros::debug_pin]
pub async fn generate_pack_from_version_id(
    project_id: String,
    version_id: String,
    title: String,
    icon_url: Option<String>,
    profile_path: ProfilePathId,

    // Existing loading bar. Unlike when existing_loading_bar is used, this one is pre-initialized with PackFileDownload
    // For example, you might use this if multiple packs are being downloaded at once and you want to use the same loading bar
    initialized_loading_bar: Option<LoadingBarId>,
) -> crate::Result<CreatePack> {
    let state = State::get().await?;

    let loading_bar = if let Some(bar) = initialized_loading_bar {
        emit_loading(&bar, 0.0, Some("Downloading pack file")).await?;
        bar
    } else {
        init_loading(
            LoadingBarType::PackFileDownload {
                profile_path: profile_path.get_full_path().await?,
                pack_name: title,
                icon: icon_url,
                pack_version: version_id.clone(),
            },
            100.0,
            "Downloading pack file",
        )
        .await?
    };

    emit_loading(&loading_bar, 0.0, Some("Fetching version")).await?;
    let creds = state.credentials.read().await;
    let version: ModrinthVersion = fetch_json(
        Method::GET,
        &format!("{}version/{}", MODRINTH_API_URL, version_id),
        None,
        None,
        &state.fetch_semaphore,
        &creds,
    )
    .await?;
    emit_loading(&loading_bar, 10.0, None).await?;

    let (url, hash) =
        if let Some(file) = version.files.iter().find(|x| x.primary) {
            Some((file.url.clone(), file.hashes.get("sha1")))
        } else {
            version
                .files
                .first()
                .map(|file| (file.url.clone(), file.hashes.get("sha1")))
        }
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "Specified version has no files".to_string(),
            )
        })?;

    let file = fetch_advanced(
        Method::GET,
        &url,
        hash.map(|x| &**x),
        None,
        None,
        Some((&loading_bar, 70.0)),
        &state.fetch_semaphore,
        &creds,
    )
    .await?;
    emit_loading(&loading_bar, 0.0, Some("Fetching project metadata")).await?;

    let project: ModrinthProject = fetch_json(
        Method::GET,
        &format!("{}project/{}", MODRINTH_API_URL, version.project_id),
        None,
        None,
        &state.fetch_semaphore,
        &creds,
    )
    .await?;

    emit_loading(&loading_bar, 10.0, Some("Retrieving icon")).await?;
    let icon = if let Some(icon_url) = project.icon_url {
        let state = State::get().await?;
        let icon_bytes =
            fetch(&icon_url, None, &state.fetch_semaphore, &creds).await?;
        drop(creds);

        let filename = icon_url.rsplit('/').next();

        if let Some(filename) = filename {
            Some(
                write_cached_icon(
                    filename,
                    &state.directories.caches_dir(),
                    icon_bytes,
                    &state.io_semaphore,
                )
                .await?,
            )
        } else {
            None
        }
    } else {
        None
    };
    emit_loading(&loading_bar, 10.0, None).await?;

    Ok(CreatePack {
        file,
        description: CreatePackDescription {
            icon,
            override_title: None,
            project_id: Some(project_id),
            version_id: Some(version_id),
            existing_loading_bar: Some(loading_bar),
            profile_path,
        },
    })
}

#[tracing::instrument]
#[theseus_macros::debug_pin]
pub async fn generate_pack_from_file(
    path: PathBuf,
    profile_path: ProfilePathId,
) -> crate::Result<CreatePack> {
    let file = io::read(&path).await?;
    Ok(CreatePack {
        file: bytes::Bytes::from(file),
        description: CreatePackDescription {
            icon: None,
            override_title: None,
            project_id: None,
            version_id: None,
            existing_loading_bar: None,
            profile_path,
        },
    })
}

/// Sets generated profile attributes to the pack ones (using profile::edit)
/// This includes the pack name, icon, game version, loader version, and loader
#[theseus_macros::debug_pin]
pub async fn set_profile_information(
    profile_path: ProfilePathId,
    description: &CreatePackDescription,
    backup_name: &str,
    dependencies: &HashMap<PackDependency, String>,
    ignore_lock: bool, // do not change locked status
) -> crate::Result<()> {
    let mut game_version: Option<&String> = None;
    let mut mod_loader = None;
    let mut loader_version = None;

    for (key, value) in dependencies {
        match key {
            PackDependency::Forge => {
                mod_loader = Some(ModLoader::Forge);
                loader_version = Some(value);
            }
            PackDependency::NeoForge => {
                mod_loader = Some(ModLoader::NeoForge);
                loader_version = Some(value);
            }
            PackDependency::FabricLoader => {
                mod_loader = Some(ModLoader::Fabric);
                loader_version = Some(value);
            }
            PackDependency::QuiltLoader => {
                mod_loader = Some(ModLoader::Quilt);
                loader_version = Some(value);
            }
            PackDependency::Minecraft => game_version = Some(value),
        }
    }

    let game_version = if let Some(game_version) = game_version {
        game_version
    } else {
        return Err(crate::ErrorKind::InputError(
            "Pack did not specify Minecraft version".to_string(),
        )
        .into());
    };

    let mod_loader = mod_loader.unwrap_or(ModLoader::Vanilla);
    let loader_version = if mod_loader != ModLoader::Vanilla {
        crate::profile::create::get_loader_version_from_loader(
            game_version.clone(),
            mod_loader,
            loader_version.cloned(),
        )
        .await?
    } else {
        None
    };
    // Sets values in profile
    crate::api::profile::edit(&profile_path, |prof| {
        prof.metadata.name = description
            .override_title
            .clone()
            .unwrap_or_else(|| backup_name.to_string());
        prof.install_stage = ProfileInstallStage::PackInstalling;

        let project_id = description.project_id.clone();
        let version_id = description.version_id.clone();

        prof.metadata.linked_data =
            if project_id.is_some() && version_id.is_some() {
                Some(LinkedData::ModrinthModpack {
                    project_id,
                    version_id,
                    locked: if !ignore_lock {
                        Some(true)
                    } else {
                        prof.metadata.linked_data.as_ref().and_then(|x| {
                            if let LinkedData::ModrinthModpack {
                                locked: Some(locked),
                                ..
                            } = x
                            {
                                Some(*locked)
                            } else {
                                None
                            }
                        })
                    },
                })
            } else {
                None
            };

        prof.metadata.icon = description.icon.clone();
        prof.metadata.game_version = game_version.clone();
        prof.metadata.loader_version = loader_version.clone();
        prof.metadata.loader = mod_loader;

        async { Ok(()) }
    })
    .await?;
    Ok(())
}
