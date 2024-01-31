use crate::config::MODRINTH_API_URL;
use crate::state::DirectoryInfo;
use crate::util::fetch::{
    fetch_advanced, read_json, write, FetchSemaphore, IoSemaphore,
};
use crate::State;
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

const AUTH_JSON: &str = "auth.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModrinthUser {
    pub id: String,
    pub username: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created: DateTime<Utc>,
    pub role: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModrinthCredentials {
    pub session: String,
    pub expires_at: DateTime<Utc>,
    pub user: ModrinthUser,
}

#[derive(Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ModrinthCredentialsResult {
    TwoFactorRequired { flow: String },
    Credentials(ModrinthCredentials),
}

#[derive(Debug)]
pub struct CredentialsStore(pub Option<ModrinthCredentials>);

impl CredentialsStore {
    pub async fn init(
        dirs: &DirectoryInfo,
        io_semaphore: &IoSemaphore,
    ) -> crate::Result<Self> {
        let auth_path = dirs.caches_meta_dir().await.join(AUTH_JSON);
        let user = read_json(&auth_path, io_semaphore).await.ok();

        if let Some(user) = user {
            Ok(Self(Some(user)))
        } else {
            Ok(Self(None))
        }
    }

    pub async fn save(&self) -> crate::Result<()> {
        let state = State::get().await?;
        let auth_path =
            state.directories.caches_meta_dir().await.join(AUTH_JSON);

        if let Some(creds) = &self.0 {
            write(&auth_path, &serde_json::to_vec(creds)?, &state.io_semaphore)
                .await?;
        }

        Ok(())
    }

    pub async fn login(
        &mut self,
        credentials: ModrinthCredentials,
    ) -> crate::Result<&Self> {
        self.0 = Some(credentials);
        self.save().await?;
        Ok(self)
    }

    #[tracing::instrument]
    pub async fn update_creds() {
        let res = async {
            let state = State::get().await?;
            let mut creds_write = state.credentials.write().await;

            refresh_credentials(&mut creds_write, &state.fetch_semaphore)
                .await?;

            Ok::<(), crate::Error>(())
        }
        .await;

        match res {
            Ok(()) => {}
            Err(err) => {
                tracing::warn!("Unable to update credentials: {err}")
            }
        };
    }

    pub async fn logout(&mut self) -> crate::Result<&Self> {
        self.0 = None;
        self.save().await?;
        Ok(self)
    }
}

pub struct ModrinthAuthFlow {
    socket: async_tungstenite::WebSocketStream<
        async_tungstenite::tokio::ConnectStream,
    >,
}

impl ModrinthAuthFlow {
    pub async fn new(provider: &str) -> crate::Result<Self> {
        let (socket, _) = async_tungstenite::tokio::connect_async(format!(
            "wss://staging-api.modrinth.com/v2/auth/ws?provider={provider}"
        ))
        .await?;
        Ok(Self { socket })
    }

    pub async fn prepare_login_url(&mut self) -> crate::Result<String> {
        let code_resp = self
            .socket
            .try_next()
            .await?
            .ok_or(
                crate::ErrorKind::WSClosedError(String::from(
                    "login socket URL",
                ))
                .as_error(),
            )?
            .into_data();

        #[derive(Deserialize)]
        struct Url {
            url: String,
        }

        let response = serde_json::from_slice::<Url>(&code_resp)?;

        Ok(response.url)
    }

    pub async fn extract_credentials(
        &mut self,
        semaphore: &FetchSemaphore,
    ) -> crate::Result<ModrinthCredentialsResult> {
        // Minecraft bearer token
        let token_resp = self
            .socket
            .try_next()
            .await?
            .ok_or(
                crate::ErrorKind::WSClosedError(String::from(
                    "login socket URL",
                ))
                .as_error(),
            )?
            .into_data();

        let response =
            serde_json::from_slice::<HashMap<String, Value>>(&token_resp)?;

        get_result_from_res("code", response, semaphore).await
    }

    pub async fn close(&mut self) -> crate::Result<()> {
        self.socket.close(None).await?;

        Ok(())
    }
}

async fn get_result_from_res(
    code_key: &str,
    response: HashMap<String, Value>,
    semaphore: &FetchSemaphore,
) -> crate::Result<ModrinthCredentialsResult> {
    if let Some(flow) = response.get("flow").and_then(|x| x.as_str()) {
        Ok(ModrinthCredentialsResult::TwoFactorRequired {
            flow: flow.to_string(),
        })
    } else if let Some(code) = response.get(code_key).and_then(|x| x.as_str()) {
        let info = fetch_info(code, semaphore).await?;

        Ok(ModrinthCredentialsResult::Credentials(
            ModrinthCredentials {
                session: code.to_string(),
                expires_at: Utc::now() + Duration::weeks(2),
                user: info,
            },
        ))
    } else if let Some(error) =
        response.get("description").and_then(|x| x.as_str())
    {
        Err(crate::ErrorKind::OtherError(format!(
            "Failed to login with error {error}"
        ))
        .as_error())
    } else {
        Err(crate::ErrorKind::OtherError(String::from(
            "Flow/code/error not found in response!",
        ))
        .as_error())
    }
}

#[derive(Deserialize, Debug)]
struct Session {
    session: String,
}

pub async fn login_password(
    username: &str,
    password: &str,
    challenge: &str,
    semaphore: &FetchSemaphore,
) -> crate::Result<ModrinthCredentialsResult> {
    let resp = fetch_advanced(
        Method::POST,
        &format!("{MODRINTH_API_URL}auth/login"),
        None,
        Some(serde_json::json!({
            "username": username,
            "password": password,
            "challenge": challenge,
        })),
        None,
        None,
        semaphore,
        &CredentialsStore(None),
    )
    .await?;
    let value = serde_json::from_slice::<HashMap<String, Value>>(&resp)?;

    get_result_from_res("session", value, semaphore).await
}

async fn get_creds_from_res(
    response: HashMap<String, Value>,
    semaphore: &FetchSemaphore,
) -> crate::Result<ModrinthCredentials> {
    if let Some(code) = response.get("session").and_then(|x| x.as_str()) {
        let info = fetch_info(code, semaphore).await?;

        Ok(ModrinthCredentials {
            session: code.to_string(),
            expires_at: Utc::now() + Duration::weeks(2),
            user: info,
        })
    } else if let Some(error) =
        response.get("description").and_then(|x| x.as_str())
    {
        Err(crate::ErrorKind::OtherError(format!(
            "Failed to login with error {error}"
        ))
        .as_error())
    } else {
        Err(crate::ErrorKind::OtherError(String::from(
            "Flow/code/error not found in response!",
        ))
        .as_error())
    }
}

pub async fn login_2fa(
    code: &str,
    flow: &str,
    semaphore: &FetchSemaphore,
) -> crate::Result<ModrinthCredentials> {
    let resp = fetch_advanced(
        Method::POST,
        &format!("{MODRINTH_API_URL}auth/login/2fa"),
        None,
        Some(serde_json::json!({
            "code": code,
            "flow": flow,
        })),
        None,
        None,
        semaphore,
        &CredentialsStore(None),
    )
    .await?;

    let response = serde_json::from_slice::<HashMap<String, Value>>(&resp)?;

    get_creds_from_res(response, semaphore).await
}

pub async fn create_account(
    username: &str,
    email: &str,
    password: &str,
    challenge: &str,
    sign_up_newsletter: bool,
    semaphore: &FetchSemaphore,
) -> crate::Result<ModrinthCredentials> {
    let resp = fetch_advanced(
        Method::POST,
        &format!("{MODRINTH_API_URL}auth/create"),
        None,
        Some(serde_json::json!({
            "username": username,
            "email": email,
            "password": password,
            "challenge": challenge,
            "sign_up_newsletter": sign_up_newsletter,
        })),
        None,
        None,
        semaphore,
        &CredentialsStore(None),
    )
    .await?;
    let response = serde_json::from_slice::<HashMap<String, Value>>(&resp)?;

    get_creds_from_res(response, semaphore).await
}

pub async fn refresh_credentials(
    credentials_store: &mut CredentialsStore,
    semaphore: &FetchSemaphore,
) -> crate::Result<()> {
    if let Some(ref mut credentials) = credentials_store.0 {
        let token = &credentials.session;
        let resp = fetch_advanced(
            Method::POST,
            &format!("{MODRINTH_API_URL}session/refresh"),
            None,
            None,
            Some(("Authorization", token)),
            None,
            semaphore,
            &CredentialsStore(None),
        )
        .await
        .ok()
        .and_then(|resp| serde_json::from_slice::<Session>(&resp).ok());

        if let Some(value) = resp {
            credentials.user = fetch_info(&value.session, semaphore).await?;
            credentials.session = value.session;
            credentials.expires_at = Utc::now() + Duration::weeks(2);
        } else if credentials.expires_at < Utc::now() {
            credentials_store.0 = None;
        }
    }

    credentials_store.save().await?;
    Ok(())
}

async fn fetch_info(
    token: &str,
    semaphore: &FetchSemaphore,
) -> crate::Result<ModrinthUser> {
    let result = fetch_advanced(
        Method::GET,
        &format!("{MODRINTH_API_URL}user"),
        None,
        None,
        Some(("Authorization", token)),
        None,
        semaphore,
        &CredentialsStore(None),
    )
    .await?;
    let value = serde_json::from_slice(&result)?;

    Ok(value)
}
