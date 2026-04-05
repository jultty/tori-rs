use std::{
    collections::HashMap,
    fs::{self, DirEntry},
    path::PathBuf,
};

use crate::{
    dev::{log::elog},
    run::Command,
};

pub fn load() -> Result<Configuration, Error> {
    elog("Loading configuration");

    let mut candidate = Configuration::default();

    let root = get_root();
    elog(&format!("Reading 'tori.conf' from: {root:?}"));
    let contents = fs::read_to_string(root.join("tori.conf"))?;
    elog(&format!("Read configuration: {contents:?}"));

    let map: HashMap<String, String> = contents
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect();

    elog(&format!("Assembled configuration map: {map:#?}"));

    if let Some(su_command) = map.get("su_command") {
        let wraps = map.get("su_command_wraps").is_some_and(|v| v == "true");
        candidate.su_command = parse_su_command(su_command, wraps)?;
    }

    if let Some(merge_strategy) = map.get("merge_strategy") {
        candidate.merge_strategy = match merge_strategy.as_str() {
            "prefer configuration" => MergeStrategy::PreferConfig,
            "prefer system" => MergeStrategy::PreferSystem,
            _ => MergeStrategy::default(),
        }
    }

    elog(&format!("Assembled configuration candidate: {candidate:?}"));
    Ok(candidate)
}

fn parse_su_command(config_value: &str, wraps: bool) -> Result<SuCommand, Error> {
    let split: Vec<&str> = config_value.split(' ').filter(|s| !s.is_empty()).collect();

    let Some((base, args)) = split.split_first() else {
        return Err(Error::new(
            "Configuration line is empty",
            ErrorKind::MalformedConfigLine,
        ));
    };

    let Ok(resolved_base) = resolve_command(base) else {
        return Err(Error::new(
            "su_command does not resolve to a command in PATH",
            ErrorKind::CommandNotInPath,
        ));
    };

    let Some(resolved_base_str) = resolved_base.to_str() else {
        return Err(Error::new(
            "su_command path contains invalid characters (expected UTF-8)",
            ErrorKind::UTF8,
        ));
    };

    elog(&format!(
        "Successfully resolved 'su_command' configuration value \
                {config_value} through PATH from base {base} and args {args:?} \
                to {resolved_base:?}"
    ));

    Ok(SuCommand {
        command: Command::new(resolved_base_str, args),
        wraps,
    })
}

fn get_root() -> PathBuf {
    if let Ok(xdg_config_dir) = std::env::var("XDG_CONFIG_DIR") {
        PathBuf::from(xdg_config_dir).join("tori")
    } else {
        if let Some(mut root) = std::env::home_dir() {
            root.push(".config");
            root.push("tori");
            root
        } else {
            if let Ok(user) = std::env::var("USER") {
                PathBuf::from("/home")
                    .join(user)
                    .join(".config")
                    .join("tori")
            } else {
                eprintln!("Failed to determine home directory");
                PathBuf::from("/etc/tori")
            }
        }
    }
}

fn resolve_command(command: &str) -> Result<PathBuf, Error> {
    elog(&format!("Solving from PATH for {command}"));

    let path_var = std::env::var("PATH")?;

    let paths = path_var
        .split(':')
        .filter(|p| !p.is_empty() && PathBuf::from(p).is_dir())
        .map(PathBuf::from);

    elog(&format!("Gathered paths {paths:?}"));

    for path in paths {
        let Ok(mut entries) = fs::read_dir(&path) else {
            elog(&format!(
                "Skipping: Could not read directory contents for path {:?}",
                &path
            ));
            continue;
        };

        let filter = |candidate: &Result<DirEntry, std::io::Error>| -> bool {
            if let Ok(entry) = candidate {
                entry.path().is_file() && entry.file_name() == command
            } else {
                false
            }
        };

        let Some(filtered) = entries.find(filter) else {
            elog("Skipping: No entries passed filter");
            continue;
        };

        if let Ok(found) = filtered {
            return Ok(found.path());
        } else {
            elog("Skipping: Filtered match is Err");
            continue;
        };
    }
    Err(Error {
        message: format!("{command} not found in any of the directories in PATH"),
        kind: ErrorKind::CommandNotInPath,
    })
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    pub su_command: SuCommand,
    pub merge_strategy: MergeStrategy,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum MergeStrategy {
    PreferSystem,
    PreferConfig,
    #[default]
    Interactive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuCommand {
    command: Command,
    wraps: bool,
}

impl SuCommand {
    pub const fn command(&self) -> &Command {
        &self.command
    }

    pub const fn wraps(&self) -> bool {
        self.wraps
    }
}

impl Default for SuCommand {
    fn default() -> SuCommand {
        SuCommand {
            command: Command::new("su", &["-c", "{% command %}"]),
            wraps: true,
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub kind: ErrorKind,
}

impl Error {
    pub fn new(message: &str, kind: ErrorKind) -> Error {
        Error {
            message: message.to_owned(),
            kind,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl From<std::env::VarError> for Error {
    fn from(var_error: std::env::VarError) -> Error {
        Error {
            message: format!("Environment variable error: {var_error}"),
            kind: ErrorKind::VarError,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(io_error: std::io::Error) -> Error {
        Error {
            message: format!("{}: {io_error}", io_error.kind()),
            kind: ErrorKind::IO,
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    CommandNotInPath,
    VarError,
    MalformedConfigLine,
    UTF8,
    IO,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ErrorKind::*;
        let s = match self {
            VarError => "Environment variable error",
            CommandNotInPath => "Command not in PATH",
            MalformedConfigLine => "Malformed configuration line",
            UTF8 => "Invalid characters could not be decoded (expected UTF-8)",
            IO => "Input/Output error",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
#[expect(clippy::panic_in_result_fn, //clippy::unwrap_in_result
)]
mod serial_tests {
    use std::{env, fs, os::unix::fs::PermissionsExt as _, io::{Write as _}};
    use super::*;
    use crate::{dev::test::{Directories, Error}};

    #[test]
    fn failed_config_read() -> Result<(), Error> {
        let dirs = Directories::setup("failed_config_read")?;

        fs::write(&dirs.conf, [1, 0, 1])?;
        let mut permissions = fs::metadata(&dirs.conf)?.permissions();
        permissions.set_mode(0o200);
        fs::set_permissions(&dirs.conf, permissions)?;

        let new_permissions = fs::metadata(&dirs.conf)?.permissions();
        assert_eq!(new_permissions.mode() & 0o777, 0o200);

        let error = load().unwrap_err();

        assert!(matches!(&error.kind, ErrorKind::IO));
        Ok(())
    }

    #[test]
    fn prefer_system() -> Result<(), Error> {
        let dirs = Directories::setup("prefer_system")?;

        let mut conf = fs::File::create_new(&dirs.conf)?;
        println!("conf: {conf:#?}");
        println!("XDG_CONFIG_DIR: {:#?}", env::var("XDG_CONFIG_DIR"));

        let conf_root_contents = dirs.conf_root.read_dir();
        println!("conf_root_contents: {conf_root_contents:#?}");

        let write_result = conf.write_all(b"merge_strategy = prefer system\n");
        println!("write_result: {write_result:#?}");
        conf.sync_all()?;

        let mut perms = fs::metadata(&dirs.conf)?.permissions();
        println!("perms: {perms:#?}");
        perms.set_mode(0o664);
        conf.set_permissions(perms)?;

        let configuration = load()?;
        println!("configuration: {configuration:#?}");

        assert!(matches!(configuration.merge_strategy, MergeStrategy::PreferSystem));

        Ok(())
    }
}
