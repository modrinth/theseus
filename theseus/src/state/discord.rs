use std::sync::{atomic::AtomicBool, Arc};

use discord_rich_presence::{
    activity::{Activity, Assets},
    DiscordIpc, DiscordIpcClient,
};
use tokio::sync::RwLock;

use crate::State;

pub struct DiscordGuard {
    client: Arc<RwLock<DiscordIpcClient>>,
    connected: Arc<AtomicBool>,
}

impl DiscordGuard {
    /// Initialize discord IPC client, and attempt to connect to it
    /// If it fails, it will still return a DiscordGuard, but the client will be unconnected
    pub async fn init() -> crate::Result<DiscordGuard> {
        let mut dipc =
            DiscordIpcClient::new("1084015525241311292").map_err(|e| {
                crate::ErrorKind::OtherError(format!(
                    "Could not create Discord client {}",
                    e,
                ))
            })?;
        let res = dipc.connect(); // Do not need to connect to Discord to use app
        let connected = if res.is_ok() {
            Arc::new(AtomicBool::new(true))
        } else {
            Arc::new(AtomicBool::new(false))
        };

        let client = Arc::new(RwLock::new(dipc));
        Ok(DiscordGuard { client, connected })
    }

    /// If the client failed connecting during init(), this will check for connection and attempt to reconnect
    /// This MUST be called first in any client method that requires a connection, because those can PANIC if the client is not connected
    /// (No connection is different than a failed connection, the latter will not panic and can be retried)
    pub async fn retry_if_not_ready(&self) -> bool {
        let mut client = self.client.write().await;
        if !self.connected.load(std::sync::atomic::Ordering::Relaxed) {
            if client.connect().is_ok() {
                self.connected
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                return true;
            }
            return false;
        }
        true
    }

    /// Set the activity to the given message
    pub async fn set_activity(
        &self,
        msg: &str,
        details: &str,
        reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        // Attempt to connect if not connected. Do not continue if it fails, as the client.set_activity can panic if it never was connected
        if !self.retry_if_not_ready().await {
            return Ok(());
        }

        let mut activity = Activity::new().state(msg).details(details).assets(
            Assets::new()
                .large_image("modrinth_simple")
                .large_text("Modrinth Logo"),
        );

        if details == "" || details == &format!("") {
            activity = Activity::new().state(msg).assets(
                Assets::new()
                    .large_image("modrinth_simple")
                    .large_text("Modrinth Logo"),
            );
        }

        // Attempt to set the activity
        // If the existing connection fails, attempt to reconnect and try again
        let mut client: tokio::sync::RwLockWriteGuard<'_, DiscordIpcClient> =
            self.client.write().await;
        let res = client.set_activity(activity.clone());
        let could_not_set_err = |e: Box<dyn serde::ser::StdError>| {
            crate::ErrorKind::OtherError(format!(
                "Could not update Discord activity {}",
                e,
            ))
        };

        if reconnect_if_fail {
            if let Err(_e) = res {
                client.reconnect().map_err(|e| {
                    crate::ErrorKind::OtherError(format!(
                        "Could not reconnect to Discord IPC {}",
                        e,
                    ))
                })?;
                return Ok(client
                    .set_activity(activity)
                    .map_err(could_not_set_err)?); // try again, but don't reconnect if it fails again
            }
        } else {
            res.map_err(could_not_set_err)?;
        }

        Ok(())
    }

    /*
    /// Clear the activity
    pub async fn clear_activity(
        &self,
        reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        // Attempt to connect if not connected. Do not continue if it fails, as the client.clear_activity can panic if it never was connected
        if !self.retry_if_not_ready().await {
            return Ok(());
        }

        // Attempt to clear the activity
        // If the existing connection fails, attempt to reconnect and try again
        let mut client = self.client.write().await;
        let res = client.clear_activity();

        let could_not_clear_err = |e: Box<dyn serde::ser::StdError>| {
            crate::ErrorKind::OtherError(format!(
                "Could not clear Discord activity {}",
                e,
            ))
        };

        if reconnect_if_fail {
            if res.is_err() {
                client.reconnect().map_err(|e| {
                    crate::ErrorKind::OtherError(format!(
                        "Could not reconnect to Discord IPC {}",
                        e,
                    ))
                })?;
                return Ok(client
                    .clear_activity()
                    .map_err(could_not_clear_err)?); // try again, but don't reconnect if it fails again
            }
        } else {
            res.map_err(could_not_clear_err)?;
        }
        Ok(())
    }*/

    /// Clear the activity, but if there is a running profile, set the activity to that instead
    pub async fn clear_to_default(
        &self,
        reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        let state: Arc<tokio::sync::RwLockReadGuard<'_, State>> =
            State::get().await?;
        if let Some(existing_child) = state
            .children
            .read()
            .await
            .running_profile_paths()
            .await?
            .first()
        {
            self.set_activity(
                &format!("Playing {}", existing_child),
                &format!("as you :)"),
                reconnect_if_fail,
            )
            .await?;
        } else {
            self.set_activity("Idling...", "", reconnect_if_fail)
                .await?;
        }
        Ok(())
    }
}
