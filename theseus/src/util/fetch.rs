//! Functions for fetching infromation from the Internet
use crate::event::emit::emit_loading;
use crate::event::LoadingBarId;
use crate::state::CredentialsStore;
use bytes::Bytes;
use lazy_static::lazy_static;
use reqwest::Method;
use serde::de::DeserializeOwned;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::{self, Duration};
use tokio::sync::{RwLock, Semaphore};
use tokio::{fs::File, io::AsyncWriteExt};

use super::io::{self, IOError};

#[derive(Debug)]
pub struct IoSemaphore(pub RwLock<Semaphore>);
#[derive(Debug)]
pub struct FetchSemaphore(pub RwLock<Semaphore>);

lazy_static! {
    pub static ref REQWEST_CLIENT: reqwest::Client = {
        let mut headers = reqwest::header::HeaderMap::new();
        let header = reqwest::header::HeaderValue::from_str(&format!(
            "modrinth/theseus/{} (support@modrinth.com)",
            env!("CARGO_PKG_VERSION")
        ))
        .unwrap();
        headers.insert(reqwest::header::USER_AGENT, header);
        reqwest::Client::builder()
            .tcp_keepalive(Some(time::Duration::from_secs(10)))
            .default_headers(headers)
            .build()
            .expect("Reqwest Client Building Failed")
    };
}
const FETCH_ATTEMPTS: usize = 3;

#[tracing::instrument(skip(semaphore))]
pub async fn fetch(
    url: &str,
    sha1: Option<&str>,
    semaphore: &FetchSemaphore,
    credentials: &CredentialsStore,
) -> crate::Result<Bytes> {
    fetch_advanced(
        Method::GET,
        url,
        sha1,
        None,
        None,
        None,
        semaphore,
        credentials,
    )
    .await
}

#[tracing::instrument(skip(json_body, semaphore))]
pub async fn fetch_json<T>(
    method: Method,
    url: &str,
    sha1: Option<&str>,
    json_body: Option<serde_json::Value>,
    semaphore: &FetchSemaphore,
    credentials: &CredentialsStore,
) -> crate::Result<T>
where
    T: DeserializeOwned,
{
    let result = fetch_advanced(
        method,
        url,
        sha1,
        json_body,
        None,
        None,
        semaphore,
        credentials,
    )
    .await?;
    let value = serde_json::from_slice(&result)?;
    Ok(value)
}

/// Downloads a file with retry and checksum functionality
#[tracing::instrument(skip(json_body, semaphore))]
#[theseus_macros::debug_pin]
#[allow(clippy::too_many_arguments)]
pub async fn fetch_advanced(
    method: Method,
    url: &str,
    sha1: Option<&str>,
    json_body: Option<serde_json::Value>,
    header: Option<(&str, &str)>,
    loading_bar: Option<(&LoadingBarId, f64)>,
    semaphore: &FetchSemaphore,
    credentials: &CredentialsStore,
) -> crate::Result<Bytes> {
    let io_semaphore = semaphore.0.read().await;
    let _permit = io_semaphore.acquire().await?;

    for attempt in 1..=(FETCH_ATTEMPTS + 1) {
        let mut req = REQWEST_CLIENT.request(method.clone(), url);

        if let Some(body) = json_body.clone() {
            req = req.json(&body);
        }

        if let Some(header) = header {
            req = req.header(header.0, header.1);
        }

        if url.starts_with("https://cdn.modrinth.com") {
            if let Some(creds) = &credentials.0 {
                req = req.header("Authorization", &creds.session);
            }
        }

        let result = req.send().await;
        match result {
            Ok(x) => {
                let bytes = if let Some((bar, total)) = &loading_bar {
                    let length = x.content_length();
                    if let Some(total_size) = length {
                        use futures::StreamExt;
                        let mut stream = x.bytes_stream();
                        let mut bytes = Vec::new();
                        while let Some(item) = stream.next().await {
                            let chunk = item.or(Err(
                                crate::error::ErrorKind::NoValueFor(
                                    "fetch bytes".to_string(),
                                ),
                            ))?;
                            bytes.append(&mut chunk.to_vec());
                            emit_loading(
                                bar,
                                (chunk.len() as f64 / total_size as f64)
                                    * total,
                                None,
                            )
                            .await?;
                        }

                        Ok(bytes::Bytes::from(bytes))
                    } else {
                        x.bytes().await
                    }
                } else {
                    x.bytes().await
                };

                if let Ok(bytes) = bytes {
                    if let Some(sha1) = sha1 {
                        let hash = sha1_async(bytes.clone()).await?;
                        if &*hash != sha1 {
                            if attempt <= 3 {
                                continue;
                            } else {
                                return Err(crate::ErrorKind::HashError(
                                    sha1.to_string(),
                                    hash,
                                )
                                .into());
                            }
                        }
                    }

                    tracing::trace!("Done downloading URL {url}");
                    return Ok(bytes);
                } else if attempt <= 3 {
                    continue;
                } else if let Err(err) = bytes {
                    return Err(err.into());
                }
            }
            Err(_) if attempt <= 3 => continue,
            Err(err) => {
                return Err(err.into());
            }
        }
    }

    unreachable!()
}

/// Downloads a file from specified mirrors
#[tracing::instrument(skip(semaphore))]
#[theseus_macros::debug_pin]
pub async fn fetch_mirrors(
    mirrors: &[&str],
    sha1: Option<&str>,
    semaphore: &FetchSemaphore,
    credentials: &CredentialsStore,
) -> crate::Result<Bytes> {
    if mirrors.is_empty() {
        return Err(crate::ErrorKind::InputError(
            "No mirrors provided!".to_string(),
        )
        .into());
    }

    for (index, mirror) in mirrors.iter().enumerate() {
        let result = fetch(mirror, sha1, semaphore, credentials).await;

        if result.is_ok() || (result.is_err() && index == (mirrors.len() - 1)) {
            return result;
        }
    }

    unreachable!()
}

/// Using labrinth API, checks if an internet response can be found, with a timeout in seconds
#[tracing::instrument]
#[theseus_macros::debug_pin]
pub async fn check_internet(timeout: u64) -> bool {
    REQWEST_CLIENT
        .get("https://launcher-files.modrinth.com/detect.txt")
        .timeout(Duration::from_secs(timeout))
        .send()
        .await
        .is_ok()
}

/// Posts a JSON to a URL
#[tracing::instrument(skip(json_body, semaphore))]
#[theseus_macros::debug_pin]
pub async fn post_json<T>(
    url: &str,
    json_body: serde_json::Value,
    semaphore: &FetchSemaphore,
    credentials: &CredentialsStore,
) -> crate::Result<T>
where
    T: DeserializeOwned,
{
    let io_semaphore = semaphore.0.read().await;
    let _permit = io_semaphore.acquire().await?;

    let mut req = REQWEST_CLIENT.post(url).json(&json_body);
    if let Some(creds) = &credentials.0 {
        req = req.header("Authorization", &creds.session);
    }

    let result = req.send().await?.error_for_status()?;

    let value = result.json().await?;
    Ok(value)
}

pub async fn read_json<T>(
    path: &Path,
    semaphore: &IoSemaphore,
) -> crate::Result<T>
where
    T: DeserializeOwned,
{
    let io_semaphore = semaphore.0.read().await;
    let _permit = io_semaphore.acquire().await?;

    let json = io::read(path).await?;
    let json = serde_json::from_slice::<T>(&json)?;

    Ok(json)
}

#[tracing::instrument(skip(bytes, semaphore))]
pub async fn write<'a>(
    path: &Path,
    bytes: &[u8],
    semaphore: &IoSemaphore,
) -> crate::Result<()> {
    let io_semaphore = semaphore.0.read().await;
    let _permit = io_semaphore.acquire().await?;

    if let Some(parent) = path.parent() {
        io::create_dir_all(parent).await?;
    }

    let mut file = File::create(path)
        .await
        .map_err(|e| IOError::with_path(e, path))?;
    file.write_all(bytes)
        .await
        .map_err(|e| IOError::with_path(e, path))?;
    tracing::trace!("Done writing file {}", path.display());
    Ok(())
}

pub async fn copy(
    src: impl AsRef<std::path::Path>,
    dest: impl AsRef<std::path::Path>,
    semaphore: &IoSemaphore,
) -> crate::Result<()> {
    let src: &Path = src.as_ref();
    let dest = dest.as_ref();

    let io_semaphore = semaphore.0.read().await;
    let _permit = io_semaphore.acquire().await?;

    if let Some(parent) = dest.parent() {
        io::create_dir_all(parent).await?;
    }

    io::copy(src, dest).await?;
    tracing::trace!(
        "Done copying file {} to {}",
        src.display(),
        dest.display()
    );
    Ok(())
}

// Writes a icon to the cache and returns the absolute path of the icon within the cache directory
#[tracing::instrument(skip(bytes, semaphore))]
pub async fn write_cached_icon(
    icon_path: &str,
    cache_dir: &Path,
    bytes: Bytes,
    semaphore: &IoSemaphore,
) -> crate::Result<PathBuf> {
    let extension = Path::new(&icon_path).extension().and_then(OsStr::to_str);
    let hash = sha1_async(bytes.clone()).await?;
    let path = cache_dir.join("icons").join(if let Some(ext) = extension {
        format!("{hash}.{ext}")
    } else {
        hash
    });

    write(&path, &bytes, semaphore).await?;

    let path = io::canonicalize(path)?;
    Ok(path)
}

pub async fn sha1_async(bytes: Bytes) -> crate::Result<String> {
    let hash = tokio::task::spawn_blocking(move || {
        sha1_smol::Sha1::from(bytes).hexdigest()
    })
    .await?;

    Ok(hash)
}
