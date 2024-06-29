//! Downloader for Minecraft data

use crate::launcher::parse_rules;
use crate::state::CredentialsStore;
use crate::{
    event::{
        emit::{emit_loading, loading_try_for_each_concurrent},
        LoadingBarId,
    },
    state::State,
    util::{fetch::*, io, platform::OsExt},
};
use daedalus::{
    self as d,
    minecraft::{
        Asset, AssetsIndex, Library, Os, Version as GameVersion,
        VersionInfo as GameVersionInfo,
    },
    modded::LoaderVersion,
};
use futures::prelude::*;
use reqwest::Method;
use tokio::sync::OnceCell;

#[tracing::instrument(skip(st, version))]
pub async fn download_minecraft(
    st: &State,
    version: &GameVersionInfo,
    loading_bar: &LoadingBarId,
    java_arch: &str,
    force: bool,
    minecraft_updated: bool,
) -> crate::Result<()> {
    tracing::info!("Downloading Minecraft version {}", version.id);
    // 5
    let assets_index =
        download_assets_index(st, version, Some(loading_bar), force).await?;

    let amount = if version
        .processors
        .as_ref()
        .map(|x| !x.is_empty())
        .unwrap_or(false)
    {
        25.0
    } else {
        40.0
    };

    tokio::try_join! {
        // Total loading sums to 90/60
        download_client(st, version, Some(loading_bar), force), // 10
        download_assets(st, version.assets == "legacy", &assets_index, Some(loading_bar), amount, force), // 40
        download_libraries(st, version.libraries.as_slice(), &version.id, Some(loading_bar), amount, java_arch, force, minecraft_updated) // 40
    }?;

    tracing::info!("Done downloading Minecraft!");
    Ok(())
}

#[tracing::instrument(skip_all, fields(version = version.id.as_str(), loader = ?loader))]
#[theseus_macros::debug_pin]
pub async fn download_version_info(
    st: &State,
    version: &GameVersion,
    loader: Option<&LoaderVersion>,
    force: Option<bool>,
    loading_bar: Option<&LoadingBarId>,
) -> crate::Result<GameVersionInfo> {
    let version_id = loader
        .map_or(version.id.clone(), |it| format!("{}-{}", version.id, it.id));
    tracing::debug!("Loading version info for Minecraft {version_id}");
    let path = st
        .directories
        .version_dir(&version_id)
        .await
        .join(format!("{version_id}.json"));

    let res = if path.exists() && !force.unwrap_or(false) {
        io::read(path)
            .err_into::<crate::Error>()
            .await
            .and_then(|ref it| Ok(serde_json::from_slice(it)?))
    } else {
        tracing::info!("Downloading version info for version {}", &version.id);
        let mut info = fetch_json(
            Method::GET,
            &version.url,
            None,
            None,
            &st.fetch_semaphore,
            &CredentialsStore(None),
        )
        .await?;

        if let Some(loader) = loader {
            let partial: d::modded::PartialVersionInfo = fetch_json(
                Method::GET,
                &loader.url,
                None,
                None,
                &st.fetch_semaphore,
                &CredentialsStore(None),
            )
            .await?;

            info = d::modded::merge_partial_version(partial, info);
        }
        info.id = version_id.clone();

        write(&path, &serde_json::to_vec(&info)?, &st.io_semaphore).await?;
        Ok(info)
    }?;

    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 5.0, None).await?;
    }

    tracing::debug!("Loaded version info for Minecraft {version_id}");
    Ok(res)
}

#[tracing::instrument(skip_all)]
#[theseus_macros::debug_pin]
pub async fn download_client(
    st: &State,
    version_info: &GameVersionInfo,
    loading_bar: Option<&LoadingBarId>,
    force: bool,
) -> crate::Result<()> {
    let version = &version_info.id;
    tracing::debug!("Locating client for version {version}");
    let client_download = version_info
        .downloads
        .get(&d::minecraft::DownloadType::Client)
        .ok_or(
            crate::ErrorKind::LauncherError(format!(
                "No client downloads exist for version {version}"
            ))
            .as_error(),
        )?;
    let path = st
        .directories
        .version_dir(version)
        .await
        .join(format!("{version}.jar"));

    if !path.exists() || force {
        let bytes = fetch(
            &client_download.url,
            Some(&client_download.sha1),
            &st.fetch_semaphore,
            &CredentialsStore(None),
        )
        .await?;
        write(&path, &bytes, &st.io_semaphore).await?;
        tracing::trace!("Fetched client version {version}");
    }
    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 9.0, None).await?;
    }

    tracing::debug!("Client loaded for version {version}!");
    Ok(())
}

#[tracing::instrument(skip_all)]
#[theseus_macros::debug_pin]
pub async fn download_assets_index(
    st: &State,
    version: &GameVersionInfo,
    loading_bar: Option<&LoadingBarId>,
    force: bool,
) -> crate::Result<AssetsIndex> {
    tracing::debug!("Loading assets index");
    let path = st
        .directories
        .assets_index_dir()
        .await
        .join(format!("{}.json", &version.asset_index.id));

    let res = if path.exists() && !force {
        io::read(path)
            .err_into::<crate::Error>()
            .await
            .and_then(|ref it| Ok(serde_json::from_slice(it)?))
    } else {
        let index = fetch_json(
            Method::GET,
            &version.asset_index.url,
            None,
            None,
            &st.fetch_semaphore,
            &CredentialsStore(None),
        )
        .await?;
        write(&path, &serde_json::to_vec(&index)?, &st.io_semaphore).await?;
        tracing::info!("Fetched assets index");
        Ok(index)
    }?;

    if let Some(loading_bar) = loading_bar {
        emit_loading(loading_bar, 5.0, None).await?;
    }
    tracing::debug!("Assets index successfully loaded!");
    Ok(res)
}

#[tracing::instrument(skip(st, index))]
#[theseus_macros::debug_pin]
pub async fn download_assets(
    st: &State,
    with_legacy: bool,
    index: &AssetsIndex,
    loading_bar: Option<&LoadingBarId>,
    loading_amount: f64,
    force: bool,
) -> crate::Result<()> {
    tracing::debug!("Loading assets");
    let num_futs = index.objects.len();
    let assets = stream::iter(index.objects.iter())
        .map(Ok::<(&String, &Asset), crate::Error>);

    loading_try_for_each_concurrent(assets,
            None,
            loading_bar,
            loading_amount,
            num_futs,
            None,
            |(name, asset)| async move {
                let hash = &asset.hash;
                let resource_path = st.directories.object_dir(hash).await;
                let url = format!(
                    "https://resources.download.minecraft.net/{sub_hash}/{hash}",
                    sub_hash = &hash[..2]
                );

                let fetch_cell = OnceCell::<bytes::Bytes>::new();
                tokio::try_join! {
                    async {
                        if !resource_path.exists() || force {
                            let resource = fetch_cell
                                .get_or_try_init(|| fetch(&url, Some(hash), &st.fetch_semaphore, &CredentialsStore(None)))
                                .await?;
                            write(&resource_path, resource, &st.io_semaphore).await?;
                            tracing::trace!("Fetched asset with hash {hash}");
                        }
                        Ok::<_, crate::Error>(())
                    },
                    async {
                        let resource_path = st.directories.legacy_assets_dir().await.join(
                            name.replace('/', &String::from(std::path::MAIN_SEPARATOR))
                        );

                        if with_legacy && !resource_path.exists() || force {
                            let resource = fetch_cell
                                .get_or_try_init(|| fetch(&url, Some(hash), &st.fetch_semaphore, &CredentialsStore(None)))
                                .await?;
                            write(&resource_path, resource, &st.io_semaphore).await?;
                            tracing::trace!("Fetched legacy asset with hash {hash}");
                        }
                        Ok::<_, crate::Error>(())
                    },
                }?;

                tracing::trace!("Loaded asset with hash {hash}");
                Ok(())
            }).await?;
    tracing::debug!("Done loading assets!");
    Ok(())
}

#[tracing::instrument(skip(st, libraries))]
#[theseus_macros::debug_pin]
#[allow(clippy::too_many_arguments)]
pub async fn download_libraries(
    st: &State,
    libraries: &[Library],
    version: &str,
    loading_bar: Option<&LoadingBarId>,
    loading_amount: f64,
    java_arch: &str,
    force: bool,
    minecraft_updated: bool,
) -> crate::Result<()> {
    tracing::debug!("Loading libraries");

    tokio::try_join! {
        io::create_dir_all(st.directories.libraries_dir().await),
        io::create_dir_all(st.directories.version_natives_dir(version).await)
    }?;
    let num_files = libraries.len();
    loading_try_for_each_concurrent(
        stream::iter(libraries.iter())
            .map(Ok::<&Library, crate::Error>), None, loading_bar,loading_amount,num_files, None,|library| async move {
                if let Some(rules) = &library.rules {
                    if !parse_rules(rules, java_arch, minecraft_updated) {
                        tracing::trace!("Skipped library {}", &library.name);
                        return Ok(());
                    }
                }

                if !library.downloadable {
                    tracing::trace!("Skipped non-downloadable library {}", &library.name);
                    return Ok(());
                }

                tokio::try_join! {
                    async {
                        let artifact_path = d::get_path_from_artifact(&library.name)?;
                        let path = st.directories.libraries_dir().await.join(&artifact_path);

                        match library.downloads {
                            _ if path.exists() && !force => Ok(()),
                            Some(d::minecraft::LibraryDownloads {
                                artifact: Some(ref artifact),
                                ..
                            }) => {
                                let bytes = fetch(&artifact.url, Some(&artifact.sha1), &st.fetch_semaphore, &CredentialsStore(None))
                                    .await?;
                                write(&path, &bytes, &st.io_semaphore).await?;
                                tracing::trace!("Fetched library {} to path {:?}", &library.name, &path);
                                Ok::<_, crate::Error>(())
                            }
                            _ => {
                                let url = [
                                    library
                                        .url
                                        .as_deref()
                                        .unwrap_or("https://libraries.minecraft.net/"),
                                    &artifact_path
                                ].concat();

                                let bytes = fetch(&url, None, &st.fetch_semaphore, &CredentialsStore(None)).await?;
                                write(&path, &bytes, &st.io_semaphore).await?;
                                tracing::trace!("Fetched library {} to path {:?}", &library.name, &path);
                                Ok::<_, crate::Error>(())
                            }
                        }
                    },
                    async {
                        // HACK: pseudo try block using or else
                        if let Some((os_key, classifiers)) = None.or_else(|| Some((
                            library
                                .natives
                                .as_ref()?
                                .get(&Os::native_arch(java_arch))?,
                            library
                                .downloads
                                .as_ref()?
                                .classifiers
                                .as_ref()?
                        ))) {
                            let parsed_key = os_key.replace(
                                "${arch}",
                                crate::util::platform::ARCH_WIDTH,
                            );

                            if let Some(native) = classifiers.get(&parsed_key) {
                                let data = fetch(&native.url, Some(&native.sha1), &st.fetch_semaphore, &CredentialsStore(None)).await?;
                                let reader = std::io::Cursor::new(&data);
                                if let Ok(mut archive) = zip::ZipArchive::new(reader) {
                                    match archive.extract(st.directories.version_natives_dir(version).await) {
                                        Ok(_) => tracing::debug!("Fetched native {}", &library.name),
                                        Err(err) => tracing::error!("Failed extracting native {}. err: {}", &library.name, err)
                                    }
                                } else {
                                    tracing::error!("Failed extracting native {}", &library.name)
                                }
                            }
                        }

                        Ok(())
                    }
                }?;

                tracing::debug!("Loaded library {}", library.name);
                Ok(())
            }
        ).await?;

    tracing::debug!("Done loading libraries!");
    Ok(())
}
