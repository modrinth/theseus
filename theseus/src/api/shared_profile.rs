use std::path::PathBuf;

use crate::{
    config::MODRINTH_API_URL_INTERNAL,
    prelude::{
        LinkedData, ModLoader, ProfilePathId, ProjectMetadata, ProjectPathId,
    },
    profile,
    state::{Profile, Profiles},
    util::{
        fetch::{fetch_advanced, REQWEST_CLIENT},
        io,
    },
};
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct SharedProfile {
    pub id: String,
    pub name: String,
    pub is_owned: bool, // Whether we are the owner (intentionally redundant)
    pub owner_id: String,
    pub icon_url: Option<String>,
    pub loader: ModLoader,
    pub loader_version: String,
    pub game_version: String,

    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,

    pub versions: Vec<String>,
    pub overrides: Vec<SharedProfileOverride>,

    pub share_links: Option<Vec<SharedProfileLink>>,
    pub users: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SharedProfileLink {
    pub id: String,
    pub created: DateTime<Utc>,
    pub expires: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SharedProfileOverride {
    pub url: String,
    pub install_path: PathBuf,
    pub hashes: SharedProfileOverrideHashes,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SharedProfileOverrideHashes {
    pub sha1: String,
    pub sha512: String,
}

// Simplified version of SharedProfile- this is what is returned from the Labrinth API
// This is not used, except for requests where we are not a member of the shared profile
// (ie: previewing a shared profile from a link, before accepting it)
#[derive(Deserialize, Serialize, Debug)]
pub struct SharedProfileResponse {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub icon_url: Option<String>,

    pub loader: ModLoader,
    pub game: String,

    pub loader_version: String,
    pub game_version: String,

    // Present only if we are the owner
    pub share_links: Option<Vec<SharedProfileLink>>,
    pub users: Option<Vec<String>>,
}

// Create a new shared profile from ProfilePathId
// This converts the LinkedData to a SharedProfile and uploads it to the Labrinth API
#[tracing::instrument]
pub async fn create(profile_id: ProfilePathId) -> crate::Result<()> {
    let state = crate::State::get().await?;

    let profile: Profile =
        profile::get(&profile_id, None).await?.ok_or_else(|| {
            crate::ErrorKind::UnmanagedProfileError(profile_id.to_string())
        })?;
    let creds = state.credentials.read().await;
    let creds = creds
        .0
        .as_ref()
        .ok_or_else(|| crate::ErrorKind::NoCredentialsError)?;

    // Currently existing linked data should fail
    match profile.metadata.linked_data {
        Some(LinkedData::SharedProfile { .. }) => {
            return Err(crate::ErrorKind::OtherError(
                "Profile already linked to a shared profile".to_string(),
            )
            .as_error());
        }
        Some(LinkedData::ModrinthModpack { .. }) => {
            return Err(crate::ErrorKind::OtherError(
                "Profile already linked to a modrinth project".to_string(),
            )
            .as_error());
        }
        None => {}
    };

    let name = profile.metadata.name;
    let loader = profile.metadata.loader;
    let loader_version = profile.metadata.loader_version;
    let game_version = profile.metadata.game_version;

    let modrinth_projects: Vec<_> = profile
        .projects
        .iter()
        .filter_map(|(_, project)| {
            if let ProjectMetadata::Modrinth { ref version, .. } =
                project.metadata
            {
                Some(&version.id)
            } else {
                None
            }
        })
        .collect();

    let override_files: Vec<_> = profile
        .projects
        .iter()
        .filter_map(|(id, project)| {
            if let ProjectMetadata::Inferred { .. } = project.metadata {
                Some(id)
            } else {
                None
            }
        })
        .collect();

    // Create the profile on the Labrinth API
    let response = REQWEST_CLIENT
        .post(format!("{MODRINTH_API_URL_INTERNAL}client/profile"))
        .header("Authorization", &creds.session)
        .json(&serde_json::json!({
            "name":  name,
            "loader": loader.as_api_str(),
            "loader_version": loader_version.map(|x| x.id).unwrap_or_default(),
            "game": "minecraft-java",
            "game_version": game_version,
            "versions": modrinth_projects,
        }))
        .send()
        .await?;

    let profile_response = response.json::<serde_json::Value>().await?;

    // Extract the profile ID from the response
    let shared_profile_id = profile_response["id"]
        .as_str()
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Could not parse response from Labrinth API".to_string(),
            )
        })?
        .to_string();

    // Unmanaged projects
    let mut data = vec![]; // 'data' field, giving installation context to labrinth
    let mut parts = vec![]; // 'parts' field, containing the actual files

    for override_file in override_files {
        let path = override_file.get_inner_path_unix();
        let Some(name) = path.0.split('/').last().map(|x| x.to_string()) else {
            continue;
        };

        // Load override to file
        let full_path = &override_file.get_full_path(&profile_id).await?;
        let file_bytes = io::read(full_path).await?;
        let ext = full_path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or_default();
        let mime = project_file_type(ext).ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Could not determine file type for {}",
                ext
            ))
        })?;

        data.push(serde_json::json!({
            "file_name": name.clone(),
            "install_path": path
        }));

        let part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(name.clone())
            .mime_str(mime)?;
        parts.push((name.clone(), part));
    }

    // Build multipart with 'data' field first
    let mut multipart = reqwest::multipart::Form::new().percent_encode_noop();
    let json_part =
        reqwest::multipart::Part::text(serde_json::to_string(&data)?); //mime_str("application/json")?;
    multipart = multipart.part("data", json_part);
    for (name, part) in parts {
        multipart = multipart.part(name, part);
    }
    let response = REQWEST_CLIENT.post(
        format!("{MODRINTH_API_URL_INTERNAL}client/profile/{shared_profile_id}/override"),
    )
    .header("Authorization", &creds.session)
    .multipart(multipart);

    response.send().await?.error_for_status()?;

    // Update the profile with the new linked data
    profile::edit(&profile_id, |profile| {
        let shared_profile_id = shared_profile_id.clone();
        profile.metadata.linked_data = Some(LinkedData::SharedProfile {
            profile_id: shared_profile_id,
            is_owner: true,
        });
        async { Ok(()) }
    })
    .await?;

    // Sync
    crate::State::sync().await?;

    Ok(())
}

pub fn project_file_type(ext: &str) -> Option<&str> {
    match ext {
        "jar" => Some("application/java-archive"),
        "zip" | "litemod" => Some("application/zip"),
        "mrpack" => Some("application/x-modrinth-modpack+zip"),
        _ => None,
    }
}

#[tracing::instrument]
pub async fn get_all() -> crate::Result<Vec<SharedProfile>> {
    let state = crate::State::get().await?;
    let creds = state.credentials.read().await;
    let creds = creds
        .0
        .as_ref()
        .ok_or_else(|| crate::ErrorKind::NoCredentialsError)?;

    let response = REQWEST_CLIENT
        .get(format!("{MODRINTH_API_URL_INTERNAL}client/user"))
        .header("Authorization", &creds.session)
        .send()
        .await?
        .error_for_status()?;

    // First, get list of shared profiles the user has access to
    let profiles = response.json::<Vec<SharedProfileResponse>>().await?;

    // Next, get files for each shared profile
    // TODO: concurrent requests
    #[derive(Serialize, Deserialize)]
    pub struct SharedFiles {
        pub version_ids: Vec<String>,
        pub override_cdns: Vec<SharedProfileOverride>,
    }

    let mut shared_profiles = vec![];
    for profile in profiles.into_iter() {
        if profile.game != "minecraft-java" {
            continue;
        }

        let id = profile.id;
        let response = REQWEST_CLIENT
            .get(format!(
                "{MODRINTH_API_URL_INTERNAL}client/profile/{id}/files"
            ))
            .header("Authorization", &creds.session)
            .send()
            .await?
            .error_for_status()?;

        let files = response.json::<SharedFiles>().await?;

        shared_profiles.push(SharedProfile {
            id,
            name: profile.name,
            is_owned: profile.owner_id
                == state
                    .credentials
                    .read()
                    .await
                    .0
                    .as_ref()
                    .map(|x| x.user.id.as_str())
                    .unwrap_or_default(),
            owner_id: profile.owner_id,
            loader: profile.loader,
            loader_version: profile.loader_version,
            game_version: profile.game_version,
            icon_url: profile.icon_url,
            versions: files.version_ids,
            overrides: files.override_cdns,
            share_links: profile.share_links,
            users: profile.users,
            updated_at: profile.updated,
            created_at: profile.created,
        });
    }

    Ok(shared_profiles)
}

#[tracing::instrument]
pub async fn install(
    shared_profile_id: String,
) -> crate::Result<ProfilePathId> {
    let state = crate::State::get().await?;
    let shared_profile = get_all()
        .await?
        .into_iter()
        .find(|x| x.id == shared_profile_id)
        .ok_or_else(|| {
            crate::ErrorKind::OtherError("Profile not found".to_string())
        })?;

    let linked_data = LinkedData::SharedProfile {
        profile_id: shared_profile.id,
        is_owner: shared_profile.is_owned,
    };

    // Create new profile
    let profile_id = crate::profile::create::profile_create(
        shared_profile.name,
        shared_profile.game_version,
        shared_profile.loader,
        Some(shared_profile.loader_version),
        None,
        shared_profile.icon_url,
        Some(linked_data),
        None,
        None,
    )
    .await?;

    // Get the profile
    let profile: Profile =
        profile::get(&profile_id, None).await?.ok_or_else(|| {
            crate::ErrorKind::UnmanagedProfileError(profile_id.to_string())
        })?;
    let creds = state.credentials.read().await;

    // TODO: concurrent requests
    // Add projects
    for version in shared_profile.versions {
        profile.add_project_version(version).await?;
    }

    for file_override in shared_profile.overrides {
        let file = fetch_advanced(
            Method::GET,
            &file_override.url,
            Some(file_override.hashes.sha1.as_str()),
            None,
            None,
            None,
            &state.fetch_semaphore,
            &creds,
        )
        .await?;

        profile
            .add_project_bytes_directly(&file_override.install_path, file)
            .await?;
    }

    Ok(profile_id)
}

// Structure repesenting a synchronization difference between a local profile and a shared profile
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct SharedModpackFileUpdate {
    // Can be false if all other fields are empty
    // if the metadata is different
    pub is_synced: bool,

    // Projects that are in the local profile but not in the shared profile
    pub unsynced_projects: Vec<ProjectPathId>,

    // Projects that are in the shared profile but not in the local profile
    pub missing_versions: Vec<String>,
    pub missing_overrides: Vec<SharedProfileOverride>,
}

#[tracing::instrument]
pub async fn check_updated(
    profile_id: &ProfilePathId,
    shared_profile: &SharedProfile,
) -> crate::Result<SharedModpackFileUpdate> {
    let profile: Profile =
        profile::get(profile_id, None).await?.ok_or_else(|| {
            crate::ErrorKind::UnmanagedProfileError(profile_id.to_string())
        })?;

    // Check if the metadata is the same- if different, we return false with no file updates
    if profile.metadata.name != shared_profile.name
        || profile.metadata.loader != shared_profile.loader
        || profile
            .metadata
            .loader_version
            .map(|x| x.id)
            .unwrap_or_default()
            != shared_profile.loader_version
        || profile.metadata.game_version != shared_profile.game_version
    {
        return Ok(SharedModpackFileUpdate::default());
    }

    // Check if the projects are the same- we check each override by hash and each modrinth project by version id
    let mut modrinth_projects = shared_profile.versions.clone();
    let mut overrides = shared_profile.overrides.clone();
    let unsynced_projects: Vec<_> = profile
        .projects
        .into_iter()
        .filter_map(|(id, project)| {
            match project.metadata {
                ProjectMetadata::Modrinth { ref version, .. } => {
                    if modrinth_projects.contains(&version.id) {
                        modrinth_projects.retain(|x| x != &version.id);
                    } else {
                        return Some(id);
                    }
                }
                ProjectMetadata::Inferred { .. } => {
                    let Some(matching_override) =
                        overrides.iter().position(|o| {
                            o.install_path.to_string_lossy()
                                == id.get_inner_path_unix().0
                        })
                    else {
                        return Some(id);
                    };

                    if let Some(o) = overrides.get(matching_override) {
                        if o.hashes.sha512 != project.sha512 {
                            return Some(id);
                        }
                    } else {
                        return Some(id);
                    }
                    overrides.remove(matching_override);
                }
                ProjectMetadata::Unknown => {
                    // TODO: What to do for unknown projects?
                    return Some(id);
                }
            }
            None
        })
        .collect();

    Ok(SharedModpackFileUpdate {
        is_synced: modrinth_projects.is_empty()
            && overrides.is_empty()
            && unsynced_projects.is_empty(),
        unsynced_projects,
        missing_versions: modrinth_projects,
        missing_overrides: overrides,
    })
}

// Updates projects for a given ProfilePathId from a SharedProfile
// This updates the local profile to match the shared profile on the Labrinth API
#[tracing::instrument]
pub async fn inbound_sync(profile_id: ProfilePathId) -> crate::Result<()> {
    let state = crate::State::get().await?;

    let profile: Profile =
        profile::get(&profile_id, None).await?.ok_or_else(|| {
            crate::ErrorKind::UnmanagedProfileError(profile_id.to_string())
        })?;
    let creds = state.credentials.read().await;

    // Get linked
    let shared_profile = match profile.metadata.linked_data {
        Some(LinkedData::SharedProfile { ref profile_id, .. }) => profile_id,
        _ => {
            return Err(crate::ErrorKind::OtherError(
                "Profile is not linked to a shared profile".to_string(),
            )
            .as_error())
        }
    };

    // Get updated shared profile
    let shared_profile = get_all()
        .await?
        .into_iter()
        .find(|x| &x.id == shared_profile)
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Profile is not linked to a shared profile".to_string(),
            )
        })?;

    let update_data = check_updated(&profile_id, &shared_profile).await?;
    if update_data.is_synced {
        return Ok(());
    }

    // Remove projects- unsynced projects need to be removed
    for project in update_data.unsynced_projects {
        profile.remove_project(&project, None).await?;
    }

    // TODO: concurrent requests
    // Add projects- missing projects need to be added
    for version in update_data.missing_versions {
        profile.add_project_version(version).await?;
    }

    for file_override in update_data.missing_overrides {
        let file = fetch_advanced(
            Method::GET,
            &file_override.url,
            Some(file_override.hashes.sha1.as_str()),
            None,
            None,
            None,
            &state.fetch_semaphore,
            &creds,
        )
        .await?;

        profile
            .add_project_bytes_directly(&file_override.install_path, file)
            .await?;
    }

    Ok(())
}

// Updates metadata for a given ProfilePathId to the Labrinth API
// Must be an owner of the shared profile
#[tracing::instrument]
pub async fn outbound_sync(profile_id: ProfilePathId) -> crate::Result<()> {
    let state = crate::State::get().await?;

    let profile: Profile =
        profile::get(&profile_id, None).await?.ok_or_else(|| {
            crate::ErrorKind::UnmanagedProfileError(profile_id.to_string())
        })?;
    let creds = state.credentials.read().await;
    let creds = creds
        .0
        .as_ref()
        .ok_or_else(|| crate::ErrorKind::NoCredentialsError)?;

    // Get linked
    let shared_profile = match profile.metadata.linked_data {
        Some(LinkedData::SharedProfile { profile_id, .. }) => profile_id,
        _ => {
            return Err(crate::ErrorKind::OtherError(
                "Profile is not linked to a shared profile".to_string(),
            )
            .as_error())
        }
    };

    // Get updated shared profile
    let shared_profile = get_all()
        .await?
        .into_iter()
        .find(|x| x.id == shared_profile)
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Profile is not linked to a shared profile".to_string(),
            )
        })?;

    // Check owner
    if !shared_profile.is_owned {
        return Err(crate::ErrorKind::OtherError(
            "Profile is not owned by the current user".to_string(),
        )
        .as_error());
    }

    // Check if we are synced
    let update_data = check_updated(&profile_id, &shared_profile).await?;
    let id = shared_profile.id;
    if update_data.is_synced {
        return Ok(());
    }

    let unsynced = update_data.unsynced_projects;
    let projects: Vec<_> = profile
        .projects
        .clone()
        .into_iter()
        .filter(|(id, _)| unsynced.contains(id))
        .collect();
    let unsynced_modrinth_projects: Vec<_> = projects
        .iter()
        .filter_map(|(_, project)| {
            if let ProjectMetadata::Modrinth { ref version, .. } =
                project.metadata
            {
                Some(&version.id)
            } else {
                None
            }
        })
        .collect();

    let unsynced_override_files: Vec<_> = projects
        .iter()
        .filter_map(|(id, project)| {
            if let ProjectMetadata::Inferred { .. } = project.metadata {
                Some(id)
            } else {
                None
            }
        })
        .collect();

    // Generate new version set
    let mut new_version_set = shared_profile.versions;
    for version in update_data.missing_versions {
        new_version_set.retain(|x| x != &version);
    }
    for version in unsynced_modrinth_projects {
        new_version_set.push(version.to_string());
    }

    // Update metadata + versions
    REQWEST_CLIENT
    .patch(
        format!("{MODRINTH_API_URL_INTERNAL}client/profile/{id}"),
    )
    .header("Authorization", &creds.session)
    .json(&serde_json::json!({
        "name": profile.metadata.name,
        "loader": profile.metadata.loader.as_api_str(),
        "loader_version": profile.metadata.loader_version.map(|x| x.id).unwrap_or_default(),
        "game": "minecraft-java",
        "game_version": profile.metadata.game_version,
        "versions": new_version_set,
    }))
    .send().await?.error_for_status()?;

    // Create multipart for uploading new overrides
    let mut parts = vec![]; // 'parts' field, containing the actual files
    let mut data = vec![]; // 'data' field, giving installation context to labrinth
    for override_file in unsynced_override_files {
        let path = override_file.get_inner_path_unix();
        let Some(name) = path.0.split('/').last().map(|x| x.to_string()) else {
            continue;
        };

        // Load override to file
        let full_path = &override_file.get_full_path(&profile_id).await?;
        let file_bytes = io::read(full_path).await?;
        let ext = full_path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or_default();
        let mime = project_file_type(ext).ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Could not determine file type for {}",
                ext
            ))
        })?;

        data.push(serde_json::json!({
            "file_name": name.clone(),
            "install_path": path
        }));

        let part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(name.clone())
            .mime_str(mime)?;
        parts.push((name.clone(), part));
    }

    // Build multipart with 'data' field first
    let mut multipart = reqwest::multipart::Form::new().percent_encode_noop();
    let json_part =
        reqwest::multipart::Part::text(serde_json::to_string(&data)?); //mime_str("application/json")?;
    multipart = multipart.part("data", json_part);
    for (name, part) in parts {
        multipart = multipart.part(name, part);
    }
    let response = REQWEST_CLIENT
        .post(format!(
            "{MODRINTH_API_URL_INTERNAL}client/profile/{id}/override"
        ))
        .header("Authorization", &creds.session)
        .multipart(multipart);

    response.send().await?.error_for_status()?;

    // Cannot fail, simply re-checks its synced with the shared profile
    Profiles::update_shared_projects().await;

    Ok(())
}

pub async fn remove_shared_profile_users(
    profile_id: ProfilePathId,
    users: Vec<String>,
) -> crate::Result<()> {
    let state = crate::State::get().await?;

    let profile: Profile =
        profile::get(&profile_id, None).await?.ok_or_else(|| {
            crate::ErrorKind::UnmanagedProfileError(profile_id.to_string())
        })?;
    let creds = state.credentials.read().await;
    let creds = creds
        .0
        .as_ref()
        .ok_or_else(|| crate::ErrorKind::NoCredentialsError)?;

    let shared_profile = match profile.metadata.linked_data {
        Some(LinkedData::SharedProfile { profile_id, .. }) => profile_id,
        _ => {
            return Err(crate::ErrorKind::OtherError(
                "Profile is not linked to a shared profile".to_string(),
            )
            .as_error())
        }
    };

    REQWEST_CLIENT
        .patch(format!(
            "{MODRINTH_API_URL_INTERNAL}client/profile/{shared_profile}"
        ))
        .header("Authorization", &creds.session)
        .json(&serde_json::json!({
            "remove_users": users,
        }))
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

pub async fn remove_shared_profile_links(
    profile_id: ProfilePathId,
    links: Vec<String>,
) -> crate::Result<()> {
    let state = crate::State::get().await?;

    let profile: Profile =
        profile::get(&profile_id, None).await?.ok_or_else(|| {
            crate::ErrorKind::UnmanagedProfileError(profile_id.to_string())
        })?;
    let creds = state.credentials.read().await;
    let creds = creds
        .0
        .as_ref()
        .ok_or_else(|| crate::ErrorKind::NoCredentialsError)?;

    let shared_profile = match profile.metadata.linked_data {
        Some(LinkedData::SharedProfile { profile_id, .. }) => profile_id,
        _ => {
            return Err(crate::ErrorKind::OtherError(
                "Profile is not linked to a shared profile".to_string(),
            )
            .as_error())
        }
    };

    REQWEST_CLIENT
        .patch(format!(
            "{MODRINTH_API_URL_INTERNAL}client/profile/{shared_profile}"
        ))
        .header("Authorization", &creds.session)
        .json(&serde_json::json!({
            "remove_links": links,
        }))
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

pub async fn generate_share_link(
    profile_id: ProfilePathId,
) -> crate::Result<String> {
    let state = crate::State::get().await?;

    let profile: Profile =
        profile::get(&profile_id, None).await?.ok_or_else(|| {
            crate::ErrorKind::UnmanagedProfileError(profile_id.to_string())
        })?;
    let creds = state.credentials.read().await;
    let creds = creds
        .0
        .as_ref()
        .ok_or_else(|| crate::ErrorKind::NoCredentialsError)?;

    let shared_profile = match profile.metadata.linked_data {
        Some(LinkedData::SharedProfile { profile_id, .. }) => profile_id,
        _ => {
            return Err(crate::ErrorKind::OtherError(
                "Profile is not linked to a shared profile".to_string(),
            )
            .as_error())
        }
    };

    let response = REQWEST_CLIENT
        .post(format!(
            "{MODRINTH_API_URL_INTERNAL}client/profile/{shared_profile}/share"
        ))
        .header("Authorization", &creds.session)
        .send()
        .await?
        .error_for_status()?;

    let link = response.json::<SharedProfileLink>().await?;

    Ok(generate_deep_link(&link))
}

fn generate_deep_link(link: &SharedProfileLink) -> String {
    format!("modrinth://shared_profile/{}", link.id)
}

pub async fn accept_share_link(link: String) -> crate::Result<()> {
    let state = crate::State::get().await?;

    let creds = state.credentials.read().await;
    let creds = creds
        .0
        .as_ref()
        .ok_or_else(|| crate::ErrorKind::NoCredentialsError)?;

    REQWEST_CLIENT
        .post(format!(
            "{MODRINTH_API_URL_INTERNAL}client/share/{link}/accept"
        ))
        .header("Authorization", &creds.session)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

// Gets a shared profile from a share link
// This is done without accepting it- so would not include any link information, and is only usable for basic info
pub async fn get_from_link(
    link: String,
) -> crate::Result<SharedProfileResponse> {
    let state = crate::State::get().await?;

    let creds = state.credentials.read().await;
    let creds = creds
        .0
        .as_ref()
        .ok_or_else(|| crate::ErrorKind::NoCredentialsError)?;

    let response = REQWEST_CLIENT
        .get(format!("{MODRINTH_API_URL_INTERNAL}client/share/{link}"))
        .header("Authorization", &creds.session)
        .send()
        .await?
        .error_for_status()?;

    let profile = response.json::<SharedProfileResponse>().await?;

    Ok(profile)
}
