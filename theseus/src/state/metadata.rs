//! Theseus metadata
use crate::data::DirectoryInfo;
use crate::state::CredentialsStore;
use crate::util::fetch::{
    fetch_json, read_json, write, FetchSemaphore, IoSemaphore,
};
use crate::State;
use daedalus::{
    minecraft::VersionManifest as MinecraftManifest,
    modded::Manifest as LoaderManifest,
};
use reqwest::Method;
use serde::{Deserialize, Serialize};

const METADATA_URL: &str = "https:/launcher-meta.modrinth.com";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub minecraft: MinecraftManifest,
    pub forge: LoaderManifest,
    pub fabric: LoaderManifest,
    pub quilt: LoaderManifest,
    pub neoforge: LoaderManifest,
}

impl Metadata {
    fn get_manifest(name: &str) -> String {
        format!("{METADATA_URL}/{name}/v0/manifest.json")
    }

    pub async fn fetch(semaphore: &FetchSemaphore) -> crate::Result<Self> {
        let (minecraft, forge, fabric, quilt, neoforge) = tokio::try_join! {
            async {
                let url = Self::get_manifest("minecraft");
                fetch_json(Method::GET, &url, None, None, semaphore, &CredentialsStore(None)).await
            },
            async {
                let url = Self::get_manifest("forge");
                fetch_json(Method::GET, &url, None, None, semaphore, &CredentialsStore(None)).await
            },
            async {
                let url = Self::get_manifest("fabric");
                fetch_json(Method::GET, &url, None, None, semaphore, &CredentialsStore(None)).await
            },
            async {
                let url = Self::get_manifest("quilt");
                fetch_json(Method::GET, &url, None, None, semaphore, &CredentialsStore(None)).await
            },
            async {
                let url = Self::get_manifest("neo");
                fetch_json(Method::GET, &url, None, None, semaphore, &CredentialsStore(None)).await
            }
        }?;

        Ok(Self {
            minecraft,
            forge,
            fabric,
            quilt,
            neoforge,
        })
    }

    // Attempt to fetch metadata and store in sled DB
    #[tracing::instrument(skip(io_semaphore))]
    #[theseus_macros::debug_pin]
    pub async fn init(
        dirs: &DirectoryInfo,
        fetch_online: bool,
        io_semaphore: &IoSemaphore,
        fetch_semaphore: &FetchSemaphore,
    ) -> crate::Result<Self> {
        let mut metadata = None;
        let metadata_path = dirs.caches_meta_dir().await.join("metadata.json");
        let metadata_backup_path =
            dirs.caches_meta_dir().await.join("metadata.json.bak");

        if let Ok(metadata_json) =
            read_json::<Metadata>(&metadata_path, io_semaphore).await
        {
            metadata = Some(metadata_json);
        } else if fetch_online {
            let res = async {
                let metadata_fetch = Self::fetch(fetch_semaphore).await?;

                write(
                    &metadata_path,
                    &serde_json::to_vec(&metadata_fetch).unwrap_or_default(),
                    io_semaphore,
                )
                .await?;

                write(
                    &metadata_backup_path,
                    &serde_json::to_vec(&metadata_fetch).unwrap_or_default(),
                    io_semaphore,
                )
                .await?;

                metadata = Some(metadata_fetch);
                Ok::<(), crate::Error>(())
            }
            .await;

            match res {
                Ok(()) => {}
                Err(err) => {
                    tracing::warn!("Unable to fetch launcher metadata: {err}")
                }
            }
        } else if let Ok(metadata_json) =
            read_json::<Metadata>(&metadata_backup_path, io_semaphore).await
        {
            metadata = Some(metadata_json);
            std::fs::copy(&metadata_backup_path, &metadata_path).map_err(
                |err| {
                    crate::ErrorKind::FSError(format!(
                        "Error restoring metadata backup: {err}"
                    ))
                    .as_error()
                },
            )?;
        }

        if let Some(meta) = metadata {
            Ok(meta)
        } else {
            Err(
                crate::ErrorKind::NoValueFor(String::from("launcher metadata"))
                    .as_error(),
            )
        }
    }

    pub async fn update() {
        let res = async {
            let state = State::get().await?;
            let metadata_fetch =
                Metadata::fetch(&state.fetch_semaphore).await?;

            let metadata_path = state
                .directories
                .caches_meta_dir()
                .await
                .join("metadata.json");
            let metadata_backup_path = state
                .directories
                .caches_meta_dir()
                .await
                .join("metadata.json.bak");

            if metadata_path.exists() {
                std::fs::copy(&metadata_path, &metadata_backup_path)?;
            }

            write(
                &metadata_path,
                &serde_json::to_vec(&metadata_fetch)?,
                &state.io_semaphore,
            )
            .await?;

            let mut old_metadata = state.metadata.write().await;
            *old_metadata = metadata_fetch;

            Ok::<(), crate::Error>(())
        }
        .await;

        match res {
            Ok(()) => {}
            Err(err) => {
                tracing::warn!("Unable to update launcher metadata: {err}")
            }
        };
    }
}
