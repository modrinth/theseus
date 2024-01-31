use crate::{
    event::{
        emit::{emit_profile, init_loading, loading_try_for_each_concurrent},
        ProfilePayloadType,
    },
    pack::{self, install_from::generate_pack_from_version_id},
    prelude::{ProfilePathId, ProjectPathId},
    profile::get,
    state::{LinkedData, ProfileInstallStage, Project},
    LoadingBarType, State,
};
use futures::try_join;

/// Updates a managed modrinth pack to the version specified by new_version_id
#[tracing::instrument]
#[theseus_macros::debug_pin]
pub async fn update_managed_modrinth_version(
    profile_path: &ProfilePathId,
    new_version_id: &String,
) -> crate::Result<()> {
    let profile = get(profile_path, None).await?.ok_or_else(|| {
        crate::ErrorKind::UnmanagedProfileError(profile_path.to_string())
            .as_error()
    })?;

    let unmanaged_err = || {
        crate::ErrorKind::InputError(
            format!("Profile at {} is not a managed modrinth pack, or has been disconnected.", profile_path),
        )
    };

    // Extract modrinth pack information, if appropriate
    let Some(LinkedData::ModrinthModpack {
        project_id: Some(ref project_id),
        version_id: Some(ref version_id),
        ..
    }) = profile.metadata.linked_data
    else {
        return Err(unmanaged_err().into());
    };

    // Replace the pack with the new version
    replace_managed_modrinth(
        profile_path,
        &profile,
        project_id,
        version_id,
        Some(new_version_id),
        true, // switching versions should ignore the lock
    )
    .await?;

    emit_profile(
        profile.uuid,
        profile_path,
        &profile.metadata.name,
        ProfilePayloadType::Edited,
    )
    .await?;

    State::sync().await?;
    Ok(())
}

/// Repair a managed modrinth pack by 'updating' it to the current version
#[tracing::instrument]
#[theseus_macros::debug_pin]
pub async fn repair_managed_modrinth(
    profile_path: &ProfilePathId,
) -> crate::Result<()> {
    let profile = get(profile_path, None).await?.ok_or_else(|| {
        crate::ErrorKind::UnmanagedProfileError(profile_path.to_string())
            .as_error()
    })?;

    let unmanaged_err = || {
        crate::ErrorKind::InputError(
            format!("Profile at {} is not a managed modrinth pack, or has been disconnected.", profile_path),
        )
    };

    // For repairing specifically, first we remove all installed projects (to ensure we do remove ones that aren't in the pack)
    // We do a project removal followed by removing everything in the .mrpack, to ensure we only
    // remove relevant projects and not things like save files
    let projects_map = profile.projects.clone();
    let stream = futures::stream::iter(
        projects_map
            .into_iter()
            .map(Ok::<(ProjectPathId, Project), crate::Error>),
    );
    loading_try_for_each_concurrent(
        stream,
        None,
        None,
        0.0,
        0,
        None,
        |(project_id, _)| {
            let profile = profile.clone();
            async move {
                profile.remove_project(&project_id, Some(true)).await?;
                Ok(())
            }
        },
    )
    .await?;

    // Extract modrinth pack information, if appropriate
    let Some(LinkedData::ModrinthModpack {
        project_id: Some(ref project_id),
        version_id: Some(ref version_id),
        ..
    }) = profile.metadata.linked_data
    else {
        return Err(unmanaged_err().into());
    };

    // Replace the pack with the same version
    replace_managed_modrinth(
        profile_path,
        &profile,
        project_id,
        version_id,
        None,
        false, // do not ignore lock, as repairing can reset the lock
    )
    .await?;

    emit_profile(
        profile.uuid,
        profile_path,
        &profile.metadata.name,
        ProfilePayloadType::Edited,
    )
    .await?;

    State::sync().await?;
    Ok(())
}

/// Replace a managed modrinth pack with a new version
/// If new_version_id is None, the pack is 'reinstalled' in-place
#[tracing::instrument(skip(profile))]
#[theseus_macros::debug_pin]
async fn replace_managed_modrinth(
    profile_path: &ProfilePathId,
    profile: &crate::state::Profile,
    project_id: &String,
    version_id: &String,
    new_version_id: Option<&String>,
    ignore_lock: bool,
) -> crate::Result<()> {
    crate::profile::edit(profile_path, |profile| {
        profile.install_stage = ProfileInstallStage::Installing;
        async { Ok(()) }
    })
    .await?;

    // Fetch .mrpacks for both old and new versions
    // TODO: this will need to be updated if we revert the hacky pack method we needed for compiler speed

    let (old_pack_creator, new_pack_creator) =
        if let Some(new_version_id) = new_version_id {
            let shared_loading_bar = init_loading(
                LoadingBarType::PackFileDownload {
                    profile_path: profile_path.get_full_path().await?,
                    pack_name: profile.metadata.name.clone(),
                    icon: None,
                    pack_version: version_id.clone(),
                },
                200.0, // These two downloads will share the same loading bar
                "Downloading pack file",
            )
            .await?;

            // download in parallel, then join.
            try_join!(
                generate_pack_from_version_id(
                    project_id.clone(),
                    version_id.clone(),
                    profile.metadata.name.clone(),
                    None,
                    profile_path.clone(),
                    Some(shared_loading_bar.clone())
                ),
                generate_pack_from_version_id(
                    project_id.clone(),
                    new_version_id.clone(),
                    profile.metadata.name.clone(),
                    None,
                    profile_path.clone(),
                    Some(shared_loading_bar)
                )
            )?
        } else {
            // If new_version_id is None, we don't need to download the new pack, so we clone the old one
            let mut old_pack_creator = generate_pack_from_version_id(
                project_id.clone(),
                version_id.clone(),
                profile.metadata.name.clone(),
                None,
                profile_path.clone(),
                None,
            )
            .await?;
            old_pack_creator.description.existing_loading_bar = None;
            (old_pack_creator.clone(), old_pack_creator)
        };

    // Removal - remove all files that were added by the old pack
    // - remove all installed projects
    // - remove all overrides
    pack::install_mrpack::remove_all_related_files(
        profile_path.clone(),
        old_pack_creator.file,
    )
    .await?;

    // Reinstallation - install all files that are added by the new pack
    // - install all projects
    // - install all overrides
    // - edits the profile to update the new data
    // - (functionals almost identically to rteinstalling the pack 'in-place')
    pack::install_mrpack::install_zipped_mrpack_files(
        new_pack_creator,
        ignore_lock,
    )
    .await?;

    Ok(())
}
