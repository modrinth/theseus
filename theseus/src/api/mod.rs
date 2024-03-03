//! API for interacting with Theseus
pub mod handler;
pub mod jre;
pub mod logs;
pub mod metadata;
pub mod minecraft_auth;
pub mod mr_auth;
pub mod pack;
pub mod process;
pub mod profile;
pub mod safety;
pub mod settings;
pub mod tags;

pub mod data {
    pub use crate::state::{
        Credentials, DirectoryInfo, Hooks, JavaSettings, LinkedData,
        MemorySettings, MinecraftLoginFlow, ModLoader, ModrinthCredentials,
        ModrinthCredentialsResult, ModrinthProject, ModrinthTeamMember,
        ModrinthUser, ModrinthVersion, ProfileMetadata, ProjectMetadata,
        Settings, Theme, WindowSize,
    };
}

pub mod prelude {
    pub use crate::{
        data::*,
        event::CommandPayload,
        jre, metadata, minecraft_auth, pack, process,
        profile::{self, create, Profile},
        settings,
        state::JavaGlobals,
        state::{Dependency, ProfilePathId, ProjectPathId},
        util::{
            io::{canonicalize, IOError},
            jre::JavaVersion,
        },
        State,
    };
}
