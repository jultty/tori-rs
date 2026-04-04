use std::{
    collections::HashMap,
    fs::{self, DirEntry},
    path::PathBuf,
};

use crate::{
    log::{self, elog},
    run::Command,
};

pub fn load() -> Result<Configuration, Error> {
    log::elog("Loading configuration");

    let mut candidate = Configuration::default();

    let root = get_root();
    let contents = fs::read_to_string(root.join("tori.conf"))?;

    let map: HashMap<String, String> = contents
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();

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

pub struct Error {
    message: String,
    kind: ErrorKind,
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
