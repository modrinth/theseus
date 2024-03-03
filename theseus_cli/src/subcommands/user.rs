//! User management subcommand
use crate::util::{confirm_async, table};
use eyre::Result;
use paris::*;
use tabled::Tabled;
use theseus::prelude::*;

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand, name = "user")]
/// manage Minecraft accounts
pub struct UserCommand {
    #[argh(subcommand)]
    action: UserSubcommand,
}

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand)]
pub enum UserSubcommand {
    Add(UserAdd),
    List(UserList),
    Remove(UserRemove),
    SetDefault(UserDefault),
}

#[derive(argh::FromArgs, Debug)]
/// add a new user to Theseus
#[argh(subcommand, name = "add")]
pub struct UserAdd {
    #[argh(option)]
    /// the browser to authenticate using
    browser: Option<webbrowser::Browser>,
}

impl UserAdd {
    pub async fn run(
        &self,
        _args: &crate::Args,
        _largs: &UserCommand,
    ) -> Result<()> {
        info!("Adding new user account to Theseus");
        info!("A browser window will now open, follow the login flow there.");

        let login = minecraft_auth::authenticate_begin_flow().await?;
        let flow =
            tokio::spawn(minecraft_auth::authenticate_await_complete_flow());

        info!("Opening browser window at {}", login.verification_uri);
        info!("Your code is {}", login.user_code);

        match self.browser {
            Some(browser) => webbrowser::open_browser(
                browser,
                login.verification_uri.as_str(),
            ),
            None => webbrowser::open(login.verification_uri.as_str()),
        }?;

        let credentials = flow.await??;
        State::sync().await?;
        success!("Logged in user {}.", credentials.username);
        Ok(())
    }
}

#[derive(argh::FromArgs, Debug)]
/// list all known users
#[argh(subcommand, name = "list")]
pub struct UserList {}

#[derive(Tabled)]
struct UserRow<'a> {
    username: &'a str,
    id: uuid::Uuid,
    default: bool,
}

impl<'a> UserRow<'a> {
    pub fn from(
        credentials: &'a Credentials,
        default: Option<uuid::Uuid>,
    ) -> Self {
        Self {
            username: &credentials.username,
            id: credentials.id,
            default: Some(credentials.id) == default,
        }
    }
}

impl UserList {
    pub async fn run(
        &self,
        _args: &crate::Args,
        _largs: &UserCommand,
    ) -> Result<()> {
        let state = State::get().await?;
        let default = state.settings.read().await.default_user;

        let users = minecraft_auth::users().await?;
        let rows = users.iter().map(|user| UserRow::from(user, default));

        let table = table(rows);
        println!("{table}");

        Ok(())
    }
}

#[derive(argh::FromArgs, Debug)]
/// remove a user
#[argh(subcommand, name = "remove")]
pub struct UserRemove {
    /// the user to remove
    #[argh(positional)]
    user: uuid::Uuid,
}

impl UserRemove {
    pub async fn run(
        &self,
        _args: &crate::Args,
        _largs: &UserCommand,
    ) -> Result<()> {
        info!("Removing user {}", self.user.as_hyphenated());

        if confirm_async(String::from("Do you wish to continue"), true).await? {
            if !minecraft_auth::has_user(self.user).await? {
                warn!("Profile was not managed by Theseus!");
            } else {
                minecraft_auth::remove_user(self.user).await?;
                State::sync().await?;
                success!("User removed!");
            }
        } else {
            error!("Aborted!");
        }

        Ok(())
    }
}

#[derive(argh::FromArgs, Debug)]
/// set the default user
#[argh(subcommand, name = "set-default")]
pub struct UserDefault {
    /// the user to set as default
    #[argh(positional)]
    user: uuid::Uuid,
}

impl UserDefault {
    pub async fn run(
        &self,
        _args: &crate::Args,
        _largs: &UserCommand,
    ) -> Result<()> {
        info!("Setting user {} as default", self.user.as_hyphenated());

        let state = State::get().await?;
        let mut settings = state.settings.write().await;

        if settings.default_user == Some(self.user) {
            warn!("User is already the default!");
        } else {
            settings.default_user = Some(self.user);
            success!("User set as default!");
        }

        Ok(())
    }
}

impl UserCommand {
    pub async fn run(&self, args: &crate::Args) -> Result<()> {
        dispatch!(&self.action, (args, self) => {
            UserSubcommand::Add,
            UserSubcommand::List,
            UserSubcommand::Remove,
            UserSubcommand::SetDefault
        })
    }
}
