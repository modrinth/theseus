//! Profile management subcommand
use crate::util::{
    confirm_async, prompt_async, select_async, table, table_path_display,
};
use daedalus::modded::LoaderVersion;
use dunce::canonicalize;
use eyre::{ensure, Result};
use futures::prelude::*;
use paris::*;
use std::path::{Path, PathBuf};
use tabled::Tabled;
use theseus::prelude::*;
use theseus::profile::create::profile_create;
use tokio::fs;
use tokio_stream::wrappers::ReadDirStream;

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand, name = "profile")]
/// manage Minecraft instances
pub struct ProfileCommand {
    #[argh(subcommand)]
    action: ProfileSubcommand,
}

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand)]
pub enum ProfileSubcommand {
    Init(ProfileInit),
    List(ProfileList),
    Remove(ProfileRemove),
    Run(ProfileRun),
}

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand, name = "init")]
/// create a new profile and manage it with Theseus
pub struct ProfileInit {
    #[argh(positional, default = "std::env::current_dir().unwrap()")]
    /// the path of the newly created profile
    path: PathBuf,

    #[argh(option)]
    /// the name of the profile
    name: Option<String>,

    #[argh(option)]
    /// the game version of the profile
    game_version: Option<String>,

    #[argh(option, from_str_fn(modloader_from_str))]
    /// the modloader to use
    modloader: Option<ModLoader>,

    #[argh(option)]
    /// the modloader version to use, set to "latest", "stable", or the ID of your chosen loader
    loader_version: Option<String>,
}

impl ProfileInit {
    pub async fn run(
        &self,
        _args: &crate::Args,
        _largs: &ProfileCommand,
    ) -> Result<()> {
        // TODO: validate inputs from args early
        let state = State::get().await?;
        let metadata = state.metadata.read().await;

        if self.path.exists() {
            ensure!(
                self.path.is_dir(),
                "Attempted to create profile in something other than a folder!"
            );
            ensure!(
                !self.path.join("profile.json").exists(),
                "Profile already exists! Perhaps you want `profile add` instead?"
            );
            if ReadDirStream::new(fs::read_dir(&self.path).await?)
                .next()
                .await
                .is_some()
            {
                warn!("You are trying to create a profile in a non-empty directory. If this is an instance from another launcher, please be sure to properly fill the profile.json fields!");
                if !confirm_async(
                    String::from("Do you wish to continue"),
                    false,
                )
                .await?
                {
                    eyre::bail!("Aborted!");
                }
            }
        } else {
            fs::create_dir_all(&self.path).await?;
        }
        info!(
            "Creating profile at path {}",
            &canonicalize(&self.path)?.display()
        );

        // TODO: abstract default prompting
        let name = match &self.name {
            Some(name) => name.clone(),
            None => {
                let default = self.path.file_name().unwrap().to_string_lossy();

                prompt_async(
                    String::from("Instance name"),
                    Some(default.into_owned()),
                )
                .await?
            }
        };

        let game_version = match &self.game_version {
            Some(version) => version.clone(),
            None => {
                let default = &metadata.minecraft.latest.release;

                prompt_async(
                    String::from("Game version"),
                    Some(default.clone()),
                )
                .await?
            }
        };

        let loader = match &self.modloader {
            Some(loader) => *loader,
            None => {
                let choice = select_async(
                    "Modloader".to_owned(),
                    &["vanilla", "fabric", "forge"],
                )
                .await?;

                match choice {
                    0 => ModLoader::Vanilla,
                    1 => ModLoader::Fabric,
                    2 => ModLoader::Forge,
                    _ => eyre::bail!(
                        "Invalid modloader ID: {choice}. This is a bug in the launcher!"
                    ),
                }
            }
        };

        let loader = if loader != ModLoader::Vanilla {
            let version = match &self.loader_version {
                Some(version) => String::from(version),
                None => prompt_async(
                    String::from(
                        "Modloader version (latest, stable, or a version ID)",
                    ),
                    Some(String::from("latest")),
                )
                .await?,
            };

            let filter = |it: &LoaderVersion| match version.as_str() {
                "latest" => true,
                "stable" => it.stable,
                id => it.id == *id,
            };

            let loader_data = match loader {
                ModLoader::Forge => &metadata.forge,
                ModLoader::Fabric => &metadata.fabric,
                _ => eyre::bail!("Could not get manifest for loader {loader}. This is a bug in the CLI!"),
            };

            let loaders = &loader_data.game_versions
                .iter()
                .find(|it| it.id == game_version)
                .ok_or_else(|| eyre::eyre!("Modloader {loader} unsupported for Minecraft version {game_version}"))?
                .loaders;

            let loader_version = loaders
                .iter()
                .find(|&x| filter(x))
                .cloned()
                .ok_or_else(|| {
                eyre::eyre!("Invalid version {version} for modloader {loader}")
            })?;

            Some((loader_version, loader))
        } else {
            None
        };

        profile_create(
            name,
            game_version,
            loader.clone().map(|x| x.1).unwrap_or(ModLoader::Vanilla),
            loader.map(|x| x.0.id),
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        success!(
            "Successfully created instance, it is now available to use with Theseus!"
        );
        Ok(())
    }
}

#[derive(argh::FromArgs, Debug)]
/// list all managed profiles
#[argh(subcommand, name = "list")]
pub struct ProfileList {}

#[derive(Tabled)]
struct ProfileRow<'a> {
    name: &'a str,
    #[field(display_with = "table_path_display")]
    path: &'a Path,
    #[header("game version")]
    game_version: &'a str,
    loader: &'a ModLoader,
    #[header("loader version")]
    loader_version: &'a str,
}

impl<'a> From<&'a Profile> for ProfileRow<'a> {
    fn from(it: &'a Profile) -> Self {
        Self {
            name: &it.metadata.name,
            path: Path::new(&it.metadata.name),
            game_version: &it.metadata.game_version,
            loader: &it.metadata.loader,
            loader_version: it
                .metadata
                .loader_version
                .as_ref()
                .map_or("", |it| &it.id),
        }
    }
}

impl<'a> From<&'a Path> for ProfileRow<'a> {
    fn from(it: &'a Path) -> Self {
        Self {
            name: "?",
            path: it,
            game_version: "?",
            loader: &ModLoader::Vanilla,
            loader_version: "?",
        }
    }
}

impl ProfileList {
    pub async fn run(
        &self,
        _args: &crate::Args,
        _largs: &ProfileCommand,
    ) -> Result<()> {
        let profiles = profile::list(None).await?;
        let rows = profiles.values().map(ProfileRow::from);

        let table = table(rows).with(
            tabled::Modify::new(tabled::Column(1..=1))
                .with(tabled::MaxWidth::wrapping(40)),
        );
        println!("{table}");

        Ok(())
    }
}

#[derive(argh::FromArgs, Debug)]
/// unmanage a profile
#[argh(subcommand, name = "remove")]
pub struct ProfileRemove {
    #[argh(positional, default = "std::env::current_dir().unwrap()")]
    /// the profile to get rid of
    profile: PathBuf,
}

impl ProfileRemove {
    pub async fn run(
        &self,
        _args: &crate::Args,
        _largs: &ProfileCommand,
    ) -> Result<()> {
        let profile =
            ProfilePathId::from_fs_path(canonicalize(&self.profile)?).await?;
        info!("Removing profile {} from Theseus", self.profile.display());

        if confirm_async(String::from("Do you wish to continue"), true).await? {
            profile::remove(&profile).await?;
            State::sync().await?;
            success!("Profile removed!");
        } else {
            error!("Aborted!");
        }

        Ok(())
    }
}

#[derive(argh::FromArgs, Debug)]
/// run a profile
#[argh(subcommand, name = "run")]
pub struct ProfileRun {
    #[argh(positional, default = "std::env::current_dir().unwrap()")]
    /// the profile to run
    profile: PathBuf,

    #[argh(option)]
    /// the user to authenticate with
    user: Option<uuid::Uuid>,
}

impl ProfileRun {
    pub async fn run(
        &self,
        _args: &crate::Args,
        _largs: &ProfileCommand,
    ) -> Result<()> {
        info!("Starting profile at path {}...", self.profile.display());
        let path = canonicalize(&self.profile)?;

        let id = future::ready(self.user.ok_or(()))
            .or_else(|_| async move {
                let state = State::get().await?;
                let settings = state.settings.read().await;

                settings.default_user
                    .ok_or(eyre::eyre!(
                        "Could not find any users, please add one using the `user add` command."
                    ))
            })
            .await?;
        let credentials = minecraft_auth::refresh(id).await?;

        let profile_path_id = ProfilePathId::from_fs_path(path).await?;
        let proc_lock =
            profile::run_credentials(&profile_path_id, &credentials).await?;
        let mut proc = proc_lock.write().await;
        process::wait_for(&mut proc).await?;

        success!("Process exited successfully!");
        Ok(())
    }
}

impl ProfileCommand {
    pub async fn run(&self, args: &crate::Args) -> Result<()> {
        dispatch!(&self.action, (args, self) => {
            ProfileSubcommand::Init,
            ProfileSubcommand::List,
            ProfileSubcommand::Remove,
            ProfileSubcommand::Run
        })
    }
}

fn modloader_from_str(it: &str) -> core::result::Result<ModLoader, String> {
    match it {
        "vanilla" => Ok(ModLoader::Vanilla),
        "forge" => Ok(ModLoader::Forge),
        "fabric" => Ok(ModLoader::Fabric),
        "quilt" => Ok(ModLoader::Quilt),
        "neoforge" => Ok(ModLoader::NeoForge),
        _ => Err(String::from("Invalid modloader: {it}")),
    }
}
