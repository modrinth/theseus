//! Minecraft CLI argument logic
use crate::launcher::parse_rules;
use crate::state::Credentials;
use crate::{
    state::{MemorySettings, WindowSize},
    util::{io::IOError, platform::classpath_separator},
};
use daedalus::{
    get_path_from_artifact,
    minecraft::{Argument, ArgumentValue, Library, VersionType},
    modded::SidedDataEntry,
};
use dunce::canonicalize;
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::{collections::HashMap, path::Path};
use uuid::Uuid;

// Replaces the space separator with a newline character, as to not split the arguments
const TEMPORARY_REPLACE_CHAR: &str = "\n";

pub fn get_class_paths(
    libraries_path: &Path,
    libraries: &[Library],
    client_path: &Path,
    java_arch: &str,
    minecraft_updated: bool,
) -> crate::Result<String> {
    let mut cps = libraries
        .iter()
        .filter_map(|library| {
            if let Some(rules) = &library.rules {
                if !parse_rules(rules, java_arch, minecraft_updated) {
                    return None;
                }
            }

            if !library.include_in_classpath {
                return None;
            }

            Some(get_lib_path(libraries_path, &library.name, false))
        })
        .collect::<Result<HashSet<_>, _>>()?;

    cps.insert(
        canonicalize(client_path)
            .map_err(|_| {
                crate::ErrorKind::LauncherError(format!(
                    "Specified class path {} does not exist",
                    client_path.to_string_lossy()
                ))
                .as_error()
            })?
            .to_string_lossy()
            .to_string(),
    );

    Ok(cps
        .into_iter()
        .collect::<Vec<_>>()
        .join(classpath_separator(java_arch)))
}

pub fn get_class_paths_jar<T: AsRef<str>>(
    libraries_path: &Path,
    libraries: &[T],
    java_arch: &str,
) -> crate::Result<String> {
    let cps = libraries
        .iter()
        .map(|library| get_lib_path(libraries_path, library.as_ref(), false))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(cps.join(classpath_separator(java_arch)))
}

pub fn get_lib_path(
    libraries_path: &Path,
    lib: &str,
    allow_not_exist: bool,
) -> crate::Result<String> {
    let mut path = libraries_path.to_path_buf();

    path.push(get_path_from_artifact(lib)?);

    if !path.exists() && allow_not_exist {
        return Ok(path.to_string_lossy().to_string());
    }

    let path = &canonicalize(&path).map_err(|_| {
        crate::ErrorKind::LauncherError(format!(
            "Library file at path {} does not exist",
            path.to_string_lossy()
        ))
        .as_error()
    })?;

    Ok(path.to_string_lossy().to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn get_jvm_arguments(
    arguments: Option<&[Argument]>,
    natives_path: &Path,
    libraries_path: &Path,
    class_paths: &str,
    version_name: &str,
    memory: MemorySettings,
    custom_args: Vec<String>,
    java_arch: &str,
) -> crate::Result<Vec<String>> {
    let mut parsed_arguments = Vec::new();

    if let Some(args) = arguments {
        parse_arguments(
            args,
            &mut parsed_arguments,
            |arg| {
                parse_jvm_argument(
                    arg.to_string(),
                    natives_path,
                    libraries_path,
                    class_paths,
                    version_name,
                    java_arch,
                )
            },
            java_arch,
        )?;
    } else {
        parsed_arguments.push(format!(
            "-Djava.library.path={}",
            canonicalize(natives_path)
                .map_err(|_| crate::ErrorKind::LauncherError(format!(
                    "Specified natives path {} does not exist",
                    natives_path.to_string_lossy()
                ))
                .as_error())?
                .to_string_lossy()
        ));
        parsed_arguments.push("-cp".to_string());
        parsed_arguments.push(class_paths.to_string());
    }
    parsed_arguments.push(format!("-Xmx{}M", memory.maximum));
    for arg in custom_args {
        if !arg.is_empty() {
            parsed_arguments.push(arg);
        }
    }

    Ok(parsed_arguments)
}

fn parse_jvm_argument(
    mut argument: String,
    natives_path: &Path,
    libraries_path: &Path,
    class_paths: &str,
    version_name: &str,
    java_arch: &str,
) -> crate::Result<String> {
    argument.retain(|c| !c.is_whitespace());
    Ok(argument
        .replace(
            "${natives_directory}",
            &canonicalize(natives_path)
                .map_err(|_| {
                    crate::ErrorKind::LauncherError(format!(
                        "Specified natives path {} does not exist",
                        natives_path.to_string_lossy()
                    ))
                    .as_error()
                })?
                .to_string_lossy(),
        )
        .replace(
            "${library_directory}",
            &canonicalize(libraries_path)
                .map_err(|_| {
                    crate::ErrorKind::LauncherError(format!(
                        "Specified libraries path {} does not exist",
                        libraries_path.to_string_lossy()
                    ))
                    .as_error()
                })?
                .to_string_lossy(),
        )
        .replace("${classpath_separator}", classpath_separator(java_arch))
        .replace("${launcher_name}", "theseus")
        .replace("${launcher_version}", env!("CARGO_PKG_VERSION"))
        .replace("${version_name}", version_name)
        .replace("${classpath}", class_paths))
}

#[allow(clippy::too_many_arguments)]
pub fn get_minecraft_arguments(
    arguments: Option<&[Argument]>,
    legacy_arguments: Option<&str>,
    credentials: &Credentials,
    version: &str,
    asset_index_name: &str,
    game_directory: &Path,
    assets_directory: &Path,
    version_type: &VersionType,
    resolution: WindowSize,
    java_arch: &str,
) -> crate::Result<Vec<String>> {
    if let Some(arguments) = arguments {
        let mut parsed_arguments = Vec::new();

        parse_arguments(
            arguments,
            &mut parsed_arguments,
            |arg| {
                parse_minecraft_argument(
                    arg,
                    &credentials.access_token,
                    &credentials.username,
                    &credentials.id,
                    version,
                    asset_index_name,
                    game_directory,
                    assets_directory,
                    version_type,
                    resolution,
                )
            },
            java_arch,
        )?;

        Ok(parsed_arguments)
    } else if let Some(legacy_arguments) = legacy_arguments {
        let mut parsed_arguments = Vec::new();
        for x in legacy_arguments.split(' ') {
            parsed_arguments.push(parse_minecraft_argument(
                &x.replace(' ', TEMPORARY_REPLACE_CHAR),
                &credentials.access_token,
                &credentials.username,
                &credentials.id,
                version,
                asset_index_name,
                game_directory,
                assets_directory,
                version_type,
                resolution,
            )?);
        }
        Ok(parsed_arguments)
    } else {
        Ok(Vec::new())
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_minecraft_argument(
    argument: &str,
    access_token: &str,
    username: &str,
    uuid: &Uuid,
    version: &str,
    asset_index_name: &str,
    game_directory: &Path,
    assets_directory: &Path,
    version_type: &VersionType,
    resolution: WindowSize,
) -> crate::Result<String> {
    Ok(argument
        .replace("${accessToken}", access_token)
        .replace("${auth_access_token}", access_token)
        .replace("${auth_session}", access_token)
        .replace("${auth_player_name}", username)
        // TODO: add auth xuid eventually
        .replace("${auth_xuid}", "0")
        .replace("${auth_uuid}", &uuid.simple().to_string())
        .replace("${uuid}", &uuid.simple().to_string())
        .replace("${clientid}", "c4502edb-c6-40cb-b595-64a280cf8906")
        .replace("${user_properties}", "{}")
        .replace("${user_type}", "msa")
        .replace("${version_name}", version)
        .replace("${assets_index_name}", asset_index_name)
        .replace(
            "${game_directory}",
            &canonicalize(game_directory)
                .map_err(|_| {
                    crate::ErrorKind::LauncherError(format!(
                        "Specified game directory {} does not exist",
                        game_directory.to_string_lossy()
                    ))
                    .as_error()
                })?
                .to_string_lossy(),
        )
        .replace(
            "${assets_root}",
            &canonicalize(assets_directory)
                .map_err(|_| {
                    crate::ErrorKind::LauncherError(format!(
                        "Specified assets directory {} does not exist",
                        assets_directory.to_string_lossy()
                    ))
                    .as_error()
                })?
                .to_string_lossy(),
        )
        .replace(
            "${game_assets}",
            &canonicalize(assets_directory)
                .map_err(|_| {
                    crate::ErrorKind::LauncherError(format!(
                        "Specified assets directory {} does not exist",
                        assets_directory.to_string_lossy()
                    ))
                    .as_error()
                })?
                .to_string_lossy(),
        )
        .replace("${version_type}", version_type.as_str())
        .replace("${resolution_width}", &resolution.0.to_string())
        .replace("${resolution_height}", &resolution.1.to_string()))
}

fn parse_arguments<F>(
    arguments: &[Argument],
    parsed_arguments: &mut Vec<String>,
    parse_function: F,
    java_arch: &str,
) -> crate::Result<()>
where
    F: Fn(&str) -> crate::Result<String>,
{
    for argument in arguments {
        match argument {
            Argument::Normal(arg) => {
                let parsed =
                    parse_function(&arg.replace(' ', TEMPORARY_REPLACE_CHAR))?;
                for arg in parsed.split(TEMPORARY_REPLACE_CHAR) {
                    parsed_arguments.push(arg.to_string());
                }
            }
            Argument::Ruled { rules, value } => {
                if parse_rules(rules, java_arch, true) {
                    match value {
                        ArgumentValue::Single(arg) => {
                            parsed_arguments.push(parse_function(
                                &arg.replace(' ', TEMPORARY_REPLACE_CHAR),
                            )?);
                        }
                        ArgumentValue::Many(args) => {
                            for arg in args {
                                parsed_arguments.push(parse_function(
                                    &arg.replace(' ', TEMPORARY_REPLACE_CHAR),
                                )?);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn get_processor_arguments<T: AsRef<str>>(
    libraries_path: &Path,
    arguments: &[T],
    data: &HashMap<String, SidedDataEntry>,
) -> crate::Result<Vec<String>> {
    let mut new_arguments = Vec::new();

    for argument in arguments {
        let trimmed_arg = &argument.as_ref()[1..argument.as_ref().len() - 1];
        if argument.as_ref().starts_with('{') {
            if let Some(entry) = data.get(trimmed_arg) {
                new_arguments.push(if entry.client.starts_with('[') {
                    get_lib_path(
                        libraries_path,
                        &entry.client[1..entry.client.len() - 1],
                        true,
                    )?
                } else {
                    entry.client.clone()
                })
            }
        } else if argument.as_ref().starts_with('[') {
            new_arguments.push(get_lib_path(libraries_path, trimmed_arg, true)?)
        } else {
            new_arguments.push(argument.as_ref().to_string())
        }
    }

    Ok(new_arguments)
}

pub async fn get_processor_main_class(
    path: String,
) -> crate::Result<Option<String>> {
    let main_class = tokio::task::spawn_blocking(move || {
        let zipfile = std::fs::File::open(&path)
            .map_err(|e| IOError::with_path(e, &path))?;
        let mut archive = zip::ZipArchive::new(zipfile).map_err(|_| {
            crate::ErrorKind::LauncherError(format!(
                "Cannot read processor at {}",
                path
            ))
            .as_error()
        })?;

        let file = archive.by_name("META-INF/MANIFEST.MF").map_err(|_| {
            crate::ErrorKind::LauncherError(format!(
                "Cannot read processor manifest at {}",
                path
            ))
            .as_error()
        })?;

        let reader = BufReader::new(file);

        for line in reader.lines() {
            let mut line = line.map_err(IOError::from)?;
            line.retain(|c| !c.is_whitespace());

            if line.starts_with("Main-Class:") {
                if let Some(class) = line.split(':').nth(1) {
                    return Ok(Some(class.to_string()));
                }
            }
        }

        Ok::<Option<String>, crate::Error>(None)
    })
    .await??;

    Ok(main_class)
}
