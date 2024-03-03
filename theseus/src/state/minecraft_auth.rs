use crate::data::DirectoryInfo;
use crate::util::fetch::{read_json, write, IoSemaphore, REQWEST_CLIENT};
use crate::State;
use base64::prelude::{BASE64_STANDARD, BASE64_URL_SAFE_NO_PAD};
use base64::Engine;
use byteorder::BigEndian;
use chrono::{DateTime, Duration, Utc};
use openssl::bn::{BigNum, BigNumContext};
use openssl::ec::{EcGroup, EcKey, PointConversionForm};
use openssl::ecdsa::EcdsaSig;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use rand::Rng;
use reqwest::header::HeaderMap;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::future::Future;
use uuid::Uuid;

// TODO: proper error handling / messaging (see others)
// TODO: make sure refreshing timelines are set correctly

const AUTH_JSON: &str = "minecraft_auth.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct SaveDeviceToken {
    pub id: String,
    pub private_key: String,
    pub x: String,
    pub y: String,
    pub token: DeviceToken,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MinecraftLoginFlow {
    pub challenge: String,
    pub session_id: String,
    pub redirect_uri: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MinecraftAuthStore {
    pub users: HashMap<Uuid, Credentials>,
    pub token: Option<SaveDeviceToken>,
    pub default_user: Option<Uuid>,
}

impl MinecraftAuthStore {
    pub async fn init(
        dirs: &DirectoryInfo,
        io_semaphore: &IoSemaphore,
    ) -> crate::Result<Self> {
        let auth_path = dirs.caches_meta_dir().await.join(AUTH_JSON);
        let store = read_json(&auth_path, io_semaphore).await.ok();

        if let Some(store) = store {
            Ok(store)
        } else {
            Ok(Self {
                users: HashMap::new(),
                token: None,
                default_user: None,
            })
        }
    }

    pub async fn save(&self) -> crate::Result<()> {
        let state = State::get().await?;
        let auth_path =
            state.directories.caches_meta_dir().await.join(AUTH_JSON);

        write(&auth_path, &serde_json::to_vec(&self)?, &state.io_semaphore)
            .await?;

        Ok(())
    }

    async fn refresh_and_get_device_token(
        &mut self,
    ) -> crate::Result<(DeviceTokenKey, DeviceToken)> {
        macro_rules! generate_key {
            ($self:ident, $generate_key:expr, $device_token:expr, $SaveDeviceToken:path) => {{
                let key = generate_key()?;
                let token = device_token(&key).await?;

                self.token = Some(SaveDeviceToken {
                    id: key.id.clone(),
                    private_key: String::from_utf8(
                        key.key.private_key_to_pem_pkcs8().map_err(|err| {
                            crate::ErrorKind::MinecraftAuthenticationError(
                                format!("Could not save private key: {}", err),
                            )
                        })?,
                    )
                    .map_err(|err| {
                        crate::ErrorKind::MinecraftAuthenticationError(format!(
                            "Could not save private key: {}",
                            err
                        ))
                    })?,
                    x: key.x.clone(),
                    y: key.y.clone(),
                    token: token.clone(),
                });
                self.save().await?;

                (key, token)
            }};
        }

        let (key, token) = if let Some(ref token) = self.token {
            if token.token.not_after > Utc::now() {
                if let Ok(private_key) =
                    PKey::private_key_from_pem(token.private_key.as_bytes())
                {
                    (
                        DeviceTokenKey {
                            id: token.id.clone(),
                            key: private_key,
                            x: token.x.clone(),
                            y: token.y.clone(),
                        },
                        token.token.clone(),
                    )
                } else {
                    generate_key!(
                        self,
                        generate_key,
                        device_token,
                        SaveDeviceToken
                    )
                }
            } else {
                generate_key!(self, generate_key, device_token, SaveDeviceToken)
            }
        } else {
            generate_key!(self, generate_key, device_token, SaveDeviceToken)
        };

        Ok((key, token))
    }

    pub async fn login_begin(&mut self) -> crate::Result<MinecraftLoginFlow> {
        let (key, token) = self.refresh_and_get_device_token().await?;

        let challenge = generate_oauth_challenge();
        let (session_id, redirect_uri) =
            sisu_authenticate(&token.token, &challenge, &key).await?;

        Ok(MinecraftLoginFlow {
            challenge,
            session_id,
            redirect_uri: redirect_uri.msa_oauth_redirect,
        })
    }

    pub async fn login_finish(
        &mut self,
        code: &str,
        flow: MinecraftLoginFlow,
    ) -> crate::Result<Credentials> {
        let (key, token) = self.refresh_and_get_device_token().await?;

        let oauth_token = oauth_token(code, &flow.challenge).await?;
        let sisu_authorize = sisu_authorize(
            Some(&flow.session_id),
            &oauth_token.access_token,
            &token.token,
            &key,
        )
        .await?;

        let xbox_token =
            xsts_authorize(sisu_authorize, &token.token, &key).await?;
        let minecraft_token = minecraft_token(xbox_token).await?;
        let profile = minecraft_profile(&minecraft_token.access_token).await?;

        minecraft_entitlements(&minecraft_token.access_token).await?;

        let profile_id = profile.id.unwrap_or_default();

        let credentials = Credentials {
            id: profile_id,
            username: profile.name,
            access_token: minecraft_token.access_token,
            refresh_token: oauth_token.refresh_token,
            expires: Utc::now()
                + Duration::seconds(oauth_token.expires_in as i64),
        };

        self.users.insert(profile_id, credentials.clone());

        if self.default_user.is_none() {
            self.default_user = Some(profile_id);
        }

        self.save().await?;

        Ok(credentials)
    }

    pub async fn get_default_credential(
        &mut self,
    ) -> crate::Result<Option<Credentials>> {
        let credentials = if let Some(default_user) = self.default_user {
            if let Some(creds) = self.users.get(&default_user) {
                Some(creds)
            } else if let Some(creds) = self.users.values().next() {
                Some(creds)
            } else {
                None
            }
        } else if let Some(creds) = self.users.values().next() {
            Some(creds)
        } else {
            None
        };

        if let Some(creds) = credentials {
            if self.default_user != Some(creds.id) {
                self.default_user = Some(creds.id);
                self.save().await?;
            }

            if creds.expires < Utc::now() {
                let cred_id = creds.id;
                let profile_name = creds.username.clone();

                println!("{}", creds.refresh_token);
                let oauth_token = oauth_refresh(&creds.refresh_token).await?;
                let (key, token) = self.refresh_and_get_device_token().await?;

                println!("test past");

                let sisu_authorize = sisu_authorize(
                    None,
                    &oauth_token.access_token,
                    &token.token,
                    &key,
                )
                .await?;

                println!("sisu authorize past");

                let xbox_token =
                    xsts_authorize(sisu_authorize, &token.token, &key).await?;

                println!("xbox authorize past");

                let minecraft_token = minecraft_token(xbox_token).await?;

                let val = Credentials {
                    id: cred_id,
                    username: profile_name,
                    access_token: minecraft_token.access_token,
                    refresh_token: oauth_token.refresh_token,
                    expires: Utc::now()
                        + Duration::seconds(oauth_token.expires_in as i64),
                };

                self.users.insert(val.id, val.clone());
                self.save().await?;

                Ok(Some(val))
            } else {
                Ok(Some(creds.clone()))
            }
        } else {
            Ok(None)
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn remove(
        &mut self,
        id: Uuid,
    ) -> crate::Result<Option<Credentials>> {
        let val = self.users.remove(&id);
        self.save().await?;
        Ok(val)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Credentials {
    pub id: Uuid,
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires: DateTime<Utc>,
}

const MICROSOFT_CLIENT_ID: &str = "00000000402b5328";
const REDIRECT_URL: &str = "https://login.live.com/oauth20_desktop.srf";
const REQUESTED_SCOPES: &str = "service::user.auth.xboxlive.com::MBI_SSL";

// flow steps
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DeviceToken {
    pub issue_instant: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub token: String,
    pub display_claims: HashMap<String, serde_json::Value>,
}

pub async fn device_token(key: &DeviceTokenKey) -> crate::Result<DeviceToken> {
    Ok(send_signed_request(
        None,
        "https://device.auth.xboxlive.com/device/authenticate",
        "/device/authenticate",
        json!({
            "Properties": {
                "AuthMethod": "ProofOfPossession",
                "Id": format!("{{{}}}", key.id),
                "DeviceType": "Win32",
                "Version": "10.16.0",
                "ProofKey": {
                    "kty": "EC",
                    "x": key.x,
                    "y": key.y,
                    "crv": "P-256",
                    "alg": "ES256",
                    "use": "sig"
                }
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"

        }),
        key,
    )
    .await?
    .1)
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RedirectUri {
    pub msa_oauth_redirect: String,
}

async fn sisu_authenticate(
    token: &str,
    challenge: &str,
    key: &DeviceTokenKey,
) -> crate::Result<(String, RedirectUri)> {
    let (headers, res) = send_signed_request(
        None,
        "https://sisu.xboxlive.com/authenticate",
        "/authenticate",
        json!({
          "AppId": MICROSOFT_CLIENT_ID,
          "DeviceToken": token,
          "Offers": [
            REQUESTED_SCOPES
          ],
          "Query": {
            "code_challenge": challenge,
            "code_challenge_method": "plain",
            "state": "",
            "prompt": "select_account"
          },
          "RedirectUri": REDIRECT_URL,
          "Sandbox": "RETAIL",
          "TokenType": "code",
        }),
        key,
    )
    .await?;

    let session_id = headers
        .get("X-SessionId")
        .and_then(|x| x.to_str().ok())
        .unwrap()
        .to_string();

    Ok((session_id, res))
}

#[derive(Deserialize)]
struct OAuthToken {
    // pub token_type: String,
    pub expires_in: u64,
    // pub scope: String,
    pub access_token: String,
    pub refresh_token: String,
    // pub user_id: String,
    // pub foci: String,
}

async fn oauth_token(code: &str, challenge: &str) -> crate::Result<OAuthToken> {
    let mut query = HashMap::new();
    query.insert("client_id", "00000000402b5328");
    query.insert("code", code);
    query.insert("code_verifier", challenge);
    query.insert("grant_type", "authorization_code");
    query.insert("redirect_uri", "https://login.live.com/oauth20_desktop.srf");
    query.insert("scope", "service::user.auth.xboxlive.com::MBI_SSL");

    let res = auth_retry(|| {
        REQWEST_CLIENT
            .post("https://login.live.com/oauth20_token.srf")
            .header("Accept", "application/json")
            .form(&query)
            .send()
    })
    .await?;

    let text = res.text().await?;

    Ok(serde_json::from_str(&text)?)
}

async fn oauth_refresh(refresh_token: &str) -> crate::Result<OAuthToken> {
    let mut query = HashMap::new();
    query.insert("client_id", "00000000402b5328");
    query.insert("refresh_token", refresh_token);
    query.insert("grant_type", "refresh_token");
    query.insert("redirect_uri", "https://login.live.com/oauth20_desktop.srf");
    query.insert("scope", "service::user.auth.xboxlive.com::MBI_SSL");

    let res = auth_retry(|| {
        REQWEST_CLIENT
            .post("https://login.live.com/oauth20_token.srf")
            .header("Accept", "application/json")
            .form(&query)
            .send()
    })
    .await?;

    let text = res.text().await?;

    Ok(serde_json::from_str(&text)?)
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SisuAuthorize {
    // pub authorization_token: DeviceToken,
    // pub device_token: String,
    // pub sandbox: String,
    pub title_token: DeviceToken,
    pub user_token: DeviceToken,
    // pub web_page: String,
}

async fn sisu_authorize(
    session_id: Option<&str>,
    access_token: &str,
    device_token: &str,
    key: &DeviceTokenKey,
) -> crate::Result<SisuAuthorize> {
    Ok(send_signed_request(
        None,
        "https://sisu.xboxlive.com/authorize",
        "/authorize",
        json!({
            "AccessToken": format!("t={access_token}"),
            "AppId": "00000000402b5328",
            "DeviceToken": device_token,
            "ProofKey": {
                "kty": "EC",
                "x": key.x,
                "y": key.y,
                "crv": "P-256",
                "alg": "ES256",
                "use": "sig"
            },
            "Sandbox": "RETAIL",
            "SessionId": session_id,
            "SiteName": "user.auth.xboxlive.com",
        }),
        key,
    )
    .await?
    .1)
}

async fn xsts_authorize(
    authorize: SisuAuthorize,
    device_token: &str,
    key: &DeviceTokenKey,
) -> crate::Result<DeviceToken> {
    Ok(send_signed_request(
        None,
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        "/xsts/authorize",
        json!({
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [authorize.user_token.token],
                "DeviceToken": device_token,
                "TitleToken": authorize.title_token.token,
            },
        }),
        key,
    )
    .await?
    .1)
}

#[derive(Deserialize)]
struct MinecraftToken {
    // pub username: String,
    pub access_token: String,
    // pub token_type: String,
    // pub expires_in: u64,
}

async fn minecraft_token(token: DeviceToken) -> crate::Result<MinecraftToken> {
    let uhs = token
        .display_claims
        .get("xui")
        .and_then(|x| x.get(0))
        .and_then(|x| x.get("uhs"))
        .and_then(|x| x.as_str().map(String::from))
        .ok_or_else(|| {
            crate::ErrorKind::MinecraftAuthenticationError(
                "Error reading user hash".to_string(),
            )
        })?;

    let token = token.token;

    let res = auth_retry(|| {
        REQWEST_CLIENT
            .post("https://api.minecraftservices.com/launcher/login")
            .header("Accept", "application/json")
            .json(&json!({
                "platform": "PC_LAUNCHER",
                "xtoken": format!("XBL3.0 x={uhs};{token}"),
            }))
            .send()
    })
    .await?;

    let text = res.text().await?;

    Ok(serde_json::from_str(&text)?)
}

#[derive(Deserialize)]
struct MinecraftProfile {
    pub id: Option<Uuid>,
    pub name: String,
}

async fn minecraft_profile(token: &str) -> crate::Result<MinecraftProfile> {
    let res = auth_retry(|| {
        REQWEST_CLIENT
            .get("https://api.minecraftservices.com/minecraft/profile")
            .header("Accept", "application/json")
            .bearer_auth(token)
            .send()
    })
    .await?;

    let text = res.text().await?;

    println!("{token}");
    println!("{text}");

    Ok(serde_json::from_str(&text)?)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinecraftEntitlements {}

async fn minecraft_entitlements(
    token: &str,
) -> crate::Result<MinecraftEntitlements> {
    let res = auth_retry(|| {
        REQWEST_CLIENT
            .get("https://api.minecraftservices.com/entitlements/mcstore")
            .header("Accept", "application/json")
            .bearer_auth(token)
            .send()
    })
    .await?;

    let text = res.text().await?;

    Ok(serde_json::from_str(&text)?)
}

// auth utils
#[tracing::instrument(skip(reqwest_request))]
async fn auth_retry<F>(
    reqwest_request: impl Fn() -> F,
) -> crate::Result<reqwest::Response>
where
    F: Future<Output = Result<Response, reqwest::Error>>,
{
    const RETRY_COUNT: usize = 9; // Does command 9 times
    const RETRY_WAIT: std::time::Duration =
        std::time::Duration::from_millis(250);

    let mut resp = reqwest_request().await?;
    for i in 0..RETRY_COUNT {
        if resp.status().is_success() {
            break;
        }
        tracing::debug!(
            "Request failed with status code {}, retrying...",
            resp.status()
        );
        if i < RETRY_COUNT - 1 {
            tokio::time::sleep(RETRY_WAIT).await;
        }
        resp = reqwest_request().await?;
    }
    Ok(resp)
}

#[derive(Debug)]
pub struct DeviceTokenKey {
    pub id: String,
    pub key: PKey<Private>,
    pub x: String,
    pub y: String,
}

fn generate_key() -> crate::Result<DeviceTokenKey> {
    let id = Uuid::new_v4().to_string();

    let group =
        EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).map_err(|err| {
            crate::ErrorKind::MinecraftAuthenticationError(format!(
                "Error initializing key pair: {}",
                err
            ))
        })?;

    let ec_key = EcKey::generate(&group).map_err(|err| {
        crate::ErrorKind::MinecraftAuthenticationError(format!(
            "Error generating private key: {}",
            err
        ))
    })?;

    // Create PKey structures for the private and public keys
    let private_key = PKey::from_ec_key(ec_key.clone()).map_err(|err| {
        crate::ErrorKind::MinecraftAuthenticationError(format!(
            "Error parsing generated key: {}",
            err
        ))
    })?;
    let public_key = PKey::from_ec_key(ec_key).map_err(|err| {
        crate::ErrorKind::MinecraftAuthenticationError(format!(
            "Error parsing generated key: {}",
            err
        ))
    })?;

    // Get the binary form of the public key
    let mut ctx = BigNumContext::new().map_err(|err| {
        crate::ErrorKind::MinecraftAuthenticationError(format!(
            "Error initializing big num ctx: {}",
            err
        ))
    })?;
    let buf = public_key
        .ec_key()
        .map_err(|err| {
            crate::ErrorKind::MinecraftAuthenticationError(format!(
                "Error reading pub elliptical curve: {}",
                err
            ))
        })?
        .public_key()
        .to_bytes(&group, PointConversionForm::UNCOMPRESSED, &mut ctx)
        .map_err(|err| {
            crate::ErrorKind::MinecraftAuthenticationError(format!(
                "Error reading pub key: {}",
                err
            ))
        })?;

    // Extract X and Y from the buffer
    // Skip the first byte which is a prefix (0x04 for uncompressed)
    let x = BigNum::from_slice(&buf[1..33]).map_err(|err| {
        crate::ErrorKind::MinecraftAuthenticationError(format!(
            "Error retrieving X encoded point: {}",
            err
        ))
    })?;
    let y = BigNum::from_slice(&buf[33..65]).map_err(|err| {
        crate::ErrorKind::MinecraftAuthenticationError(format!(
            "Error retrieving Y encoded point: {}",
            err
        ))
    })?;

    Ok(DeviceTokenKey {
        id,
        key: private_key,
        x: BASE64_URL_SAFE_NO_PAD.encode(x.to_vec()),
        y: BASE64_URL_SAFE_NO_PAD.encode(y.to_vec()),
    })
}

async fn send_signed_request<T: DeserializeOwned>(
    authorization: Option<&str>,
    url: &str,
    url_path: &str,
    raw_body: serde_json::Value,
    key: &DeviceTokenKey,
) -> crate::Result<(HeaderMap, T)> {
    let auth = authorization
        .clone()
        .map_or(Vec::new(), |v| v.as_bytes().to_vec());

    let body = serde_json::to_vec(&raw_body)?;
    let time: u128 =
        { ((Utc::now().timestamp() as u128) + 11644473600) * 10000000 };

    use byteorder::WriteBytesExt;
    let mut buffer = Vec::new();
    buffer.write_u32::<BigEndian>(1)?;
    buffer.write_u8(0)?;
    buffer.write_u64::<BigEndian>(time as u64)?;
    buffer.write_u8(0)?;
    buffer.extend_from_slice("POST".as_bytes());
    buffer.write_u8(0)?;
    buffer.extend_from_slice(url_path.as_bytes());
    buffer.write_u8(0)?;
    buffer.extend_from_slice(&auth);
    buffer.write_u8(0)?;
    buffer.extend_from_slice(&body);
    buffer.write_u8(0)?;

    let digest = Sha256::digest(&buffer);

    let ecdsa_sig = EcdsaSig::sign(
        &digest.to_vec(),
        key.key
            .ec_key()
            .map_err(|err| {
                crate::ErrorKind::MinecraftAuthenticationError(format!(
                    "Error reading private key for signing: {}",
                    err
                ))
            })?
            .as_ref(),
    )
    .map_err(|err| {
        crate::ErrorKind::MinecraftAuthenticationError(format!(
            "Error signing request body: {}",
            err
        ))
    })?;

    let mut sig_buffer = Vec::new();
    sig_buffer.write_i32::<BigEndian>(1)?;
    sig_buffer.write_u64::<BigEndian>(time as u64)?;
    sig_buffer.extend_from_slice(&ecdsa_sig.r().to_vec());
    sig_buffer.extend_from_slice(&ecdsa_sig.s().to_vec());

    let signature = BASE64_STANDARD.encode(&sig_buffer);

    let res = auth_retry(|| {
        let mut request = REQWEST_CLIENT
            .post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("x-xbl-contract-version", "1")
            .header("signature", &*signature);

        if let Some(auth) = authorization {
            request = request.header("Authorization", auth);
        }

        request.body(body.clone()).send()
    })
    .await?;

    let headers = res.headers().clone();
    let res = res.text().await?;

    let body = serde_json::from_str(&res)?;
    Ok((headers, body))
}

fn generate_oauth_challenge() -> String {
    let mut rng = rand::thread_rng();

    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}
