//! Authentication flow interface
use crate::state::{Credentials, MinecraftLoginFlow};
use crate::State;

pub async fn begin_login() -> crate::Result<MinecraftLoginFlow> {
    let state = State::get().await?;
    let mut users = state.users.write().await;

    Ok(users.login_begin().await?)
}

pub async fn finish_login(
    code: &str,
    flow: MinecraftLoginFlow,
) -> crate::Result<Credentials> {
    let state = State::get().await?;
    let mut users = state.users.write().await;

    Ok(users.login_finish(code, flow).await?)
}

pub async fn get_default_user() -> crate::Result<Option<uuid::Uuid>> {
    let state = State::get().await?;
    let users = state.users.read().await;
    Ok(users.default_user)
}

pub async fn set_default_user(user: uuid::Uuid) -> crate::Result<()> {
    let user = get_user(user).await?;
    let state = State::get().await?;
    let mut users = state.users.write().await;
    users.default_user = Some(user.id);
    users.save().await?;
    Ok(())
}

/// Remove a user account from the database
#[tracing::instrument]
pub async fn remove_user(user: uuid::Uuid) -> crate::Result<()> {
    let state = State::get().await?;
    let mut users = state.users.write().await;
    users.remove(user).await?;

    Ok(())
}

/// Get a copy of the list of all user credentials
#[tracing::instrument]
pub async fn users() -> crate::Result<Vec<Credentials>> {
    let state = State::get().await?;
    let users = state.users.read().await;
    Ok(users.users.values().cloned().collect())
}

/// Get a specific user by user ID
/// Prefer to use 'refresh' instead of this function
#[tracing::instrument]
pub async fn get_user(user: uuid::Uuid) -> crate::Result<Credentials> {
    let state = State::get().await?;
    let users = state.users.read().await;
    let user = users
        .users
        .get(&user)
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Tried to get nonexistent user with ID {user}"
            ))
            .as_error()
        })?
        .clone();
    Ok(user)
}
