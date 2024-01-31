//! API for interacting with Theseus
pub mod auth;
pub mod handler;
pub mod hydra;
pub mod jre;
pub mod logs;
pub mod metadata;
pub mod mr_auth;
pub mod pack;
pub mod process;
pub mod profile;
pub mod safety;
pub mod settings;
pub mod shared_profile;
pub mod tags;

pub mod data {
    pub use crate::state::{
        DirectoryInfo, Hooks, JavaSettings, LinkedData, MemorySettings,
        ModLoader, ModrinthCredentials, ModrinthCredentialsResult,
        ModrinthProject, ModrinthTeamMember, ModrinthUser, ModrinthVersion,
        ProfileMetadata, ProjectMetadata, Settings, Theme, WindowSize,
    };
}

pub mod prelude {
    pub use crate::{
        auth::{self, Credentials},
        data::*,
        event::CommandPayload,
        jre, metadata, pack, process,
        profile::{self, create, Profile},
        settings, shared_profile,
        state::JavaGlobals,
        state::{Dependency, ProfilePathId, ProjectPathId},
        util::{
            io::{canonicalize, IOError},
            jre::JavaVersion,
        },
        State,
    };
}
