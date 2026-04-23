use std::{
    collections::HashMap,
    fs::{self, DirEntry},
    os::unix::fs::PermissionsExt as _,
    path::PathBuf,
};

use proptest_derive::Arbitrary;

use crate::{dev::log::elog, run::Command};

#[derive(Debug, Arbitrary)]
struct Raw {
    su_command: Option<String>,
    su_command_wraps: Option<String>,
    merge_strategy: Option<String>,
}

pub fn load() -> Result<Configuration, Error> {
    elog("Loading configuration");

    let root = get_root();
    elog(&format!("Reading 'tori.conf' from: {root:?}"));

    let contents = fs::read_to_string(root.join("tori.conf"))?;
    elog(&format!("Read configuration file: {contents:?}"));

    let map: HashMap<String, String> = contents
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect();

    elog(&format!("Assembled configuration map: {map:#?}"));

    let raw = Raw {
        su_command: map.get("su_command").cloned(),
        su_command_wraps: map.get("su_command_wraps").cloned(),
        merge_strategy: map.get("merge_strategy").cloned(),
    };

    elog(&format!("Read raw configuration: {raw:?}"));
    Ok(parse(&raw))
}

fn parse(raw: &Raw) -> Configuration {
    let default = Configuration::default();
    let mut candidate = default;

    if let Some(su_command_value) = &raw.su_command {
        let wraps_value = match &raw.su_command_wraps {
            Some(wraps) => wraps == "true",
            None => false,
        };

        match parse_su_command(su_command_value, wraps_value) {
            Ok(s) => candidate.su_command = s,
            Err(error) => println!("Failed parsing su_comand configuration value: {error}"),
        }
    }

    if let Some(merge_strategy) = &raw.merge_strategy {
        candidate.merge_strategy = match merge_strategy.as_str() {
            "prefer configuration" => MergeStrategy::PreferConfig,
            "prefer system" => MergeStrategy::PreferSystem,
            any => {
                println!("Unrecognized merge strategy: {any}");
                MergeStrategy::default()
            }
        }
    }

    elog(&format!("Parsed configuration candidate: {candidate:?}"));
    candidate
}

fn parse_su_command(config_value: &str, wraps: bool) -> Result<SuCommand, Error> {
    // TODO this is a horrible way to split because it will unquote everything
    let split: Vec<&str> = config_value.split(' ').filter(|s| !s.is_empty()).collect();

    let Some((base, args)) = split.split_first() else {
        return Err(Error::new(
            "Configuration line is empty",
            ErrorKind::MalformedConfigLine,
        ));
    };

    let resolved_base = if PathBuf::from(base).is_absolute() {
        PathBuf::from(base)
    } else {
        resolve_command(base)?
    };

    if resolved_base.is_file()
        && let Ok(metadata) = resolved_base.metadata()
    {
        let mode = metadata.permissions().mode();
        if mode & 0o111 == 0 {
            return Err(Error::new(
                "su_command path does not point to an executable file",
                ErrorKind::WrongPermissions,
            ));
        }
    } else {
        return Err(Error::new(
            "su_command path does not point to a file or its metadata is unreadable",
            ErrorKind::MetadataUnreadable,
        ));
    }

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

#[cfg(test)]
impl From<Error> for proptest::test_runner::TestCaseError {
    fn from(error: Error) -> proptest::test_runner::TestCaseError {
        proptest::test_runner::TestCaseError::fail(format!("{}: {}", error.kind, error.message))
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    CommandNotInPath,
    VarError,
    MalformedConfigLine,
    MetadataUnreadable,
    WrongPermissions,
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
            MetadataUnreadable => "Metadata unreadable",
            WrongPermissions => "Wrong permissions",
            UTF8 => "Invalid characters could not be decoded (expected UTF-8)",
            IO => "Input/Output error",
        };
        write!(f, "{s}")
    }
}

// TODO review this test
#[cfg(test)]
#[expect(clippy::panic_in_result_fn)]
mod serial_tests {
    use proptest::property_test;
    use std::{env, fs, io::Write as _, os::unix::fs::PermissionsExt as _};

    use super::*;
    use crate::dev::test::{Directories, Error};

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

        assert!(matches!(
            configuration.merge_strategy,
            MergeStrategy::PreferSystem
        ));

        Ok(())
    }

    #[property_test]
    fn su_command_wrap_is_read_from_config(value: String) -> Result<(), Error> {
        let dirs = Directories::setup("su_command_wraps_is_read_from_config")?;

        let mut conf = fs::File::create_new(&dirs.conf)?;
        conf.write_all(format!("su_command_wraps = {value}").as_bytes())?;
        conf.sync_all()?;

        let configuration = load()?;
        let default = Configuration::default();

        if value == "false" {
            assert!(!configuration.su_command.wraps);
        } else if configuration.su_command == default.su_command {
            assert!(configuration.su_command.wraps);
        } else {
            assert!(value == "true");
        }

        Ok(())
    }

    #[property_test]
    fn configuration_parses(raw: Raw) {
        let parsed = parse(&raw);
        let default = Configuration::default();

        if let Some(su_command_value) = raw.su_command {
            // these duplicated extractions are also in the tested
            // code, this shpuld be in Command::from(&str)
            let (base, args_opt) = match su_command_value.split_once(' ') {
                Some((b, a)) => (b, Some(a)),
                None => (su_command_value.as_str(), None),
            };

            let args = match args_opt {
                Some(a) => vec![a],
                None => vec![],
            };

            // this could also be a method of Command
            if let Ok(resolved_su_command) = resolve_command(base) {
                assert_eq!(parsed.su_command.command.base, resolved_su_command);
            } else {
                assert_eq!(parsed.su_command, default.su_command);
            }
        } else {
            assert_eq!(parsed.su_command, default.su_command);
        }

        if let Some(merge_strategy) = &raw.merge_strategy {
            use MergeStrategy::*;

            // i guess this is fine (could be a match?) but it makes
            // you think about how tests duplicate tautologies
            if merge_strategy == "prefer system" {
                assert!(matches!(parsed.merge_strategy, PreferSystem));
            } else if merge_strategy == "prefer configuration" {
                assert!(matches!(parsed.merge_strategy, PreferConfig));
            } else {
                assert!(matches!(parsed.merge_strategy, Interactive));
            }
        }

        // TODO match raw.su_command_wraps {}
    }
}
