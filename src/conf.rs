use std::{
    collections::HashMap,
    fs::{self, DirEntry},
    path::PathBuf,
};

use crate::{
    log::{self, elog},
    run::Command,
};

pub fn load() -> Configuration {
    log::elog("Loading configuration");

    let mut conf = Configuration {
        su_command: SuCommand {
            command: Command::new("su", &["-c", "{% command %}"]),
            wraps: true,
        },
        su_command_wraps: None,
    };

    let root = get_root();
    let Ok(contents) = fs::read_to_string(root.join("tori.conf")) else {
        eprintln!("Failed reading configuration file at {root:?}");
        return conf;
    };

    let lines: Vec<(&str, &str)> = contents
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();

    let mut map: HashMap<String, String> = HashMap::new();

    for line in &lines {
        let (key, value) = line;
        map.insert(key.to_string(), value.to_string());
    }

    elog(&format!("{lines:#?}"));

    if let Some(su_command) = map.get("su_command") {
        let split: Vec<String> = su_command
            .split(' ')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        if let Some((base, args)) = split.split_first()
            && let Ok(resolved_path) = resolve_from_path(base)
        {
            elog(&format!(
                "Succesfully resolved 'su_command' configuration value \
                {su_command} through PATH to {resolved_path:?}, with base \
                {base} and args {args:?}"
            ));
            conf.su_command = SuCommand {
                command: Command::new_from_strings(base, args),
                wraps: map.get("su_command_wraps").is_some_and(|v| v == "true"),
            }
        } else {
            eprintln!("Failed validation of 'su_command' configuration value");
        }
    }

    if let Some(su_command_wraps) = map.get("su_command_wraps") {
        conf.su_command_wraps = Some(su_command_wraps == "true");
    }

    conf
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
                PathBuf::from("/home").join(user).join(".config").join("tori")
            } else {
                eprintln!("Failed to determine home directory");
                PathBuf::from("/etc/tori")
            }
        }
    }
}

fn resolve_from_path(command: &str) -> Result<PathBuf, String> {
    elog(&format!("Solving from PATH for {command}"));

    let paths: Vec<PathBuf> = if let Ok(path) = std::env::var("PATH") {
        path.split(':')
            .filter(|p| !p.is_empty() && PathBuf::from(p).is_dir())
            .map(PathBuf::from)
            .collect()
    } else {
        elog("Error: PATH is not set");
        return Err(format!(
            "{command} not found: PATH is not set in the environment"
        ));
    };

    elog(&format!("Gathered paths {paths:?}"));
    for path in paths {
        elog(&format!("On path {path:?}"));
        let Ok(mut entries) = fs::read_dir(path) else {
            elog("Skipping: Could not read directory contents");
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
    Err(format!(
        "{command} not found in any of the directories in PATH"
    ))
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    pub su_command: SuCommand,
    pub su_command_wraps: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SuCommand {
    command: Command,
    wraps: bool,
}

impl SuCommand {
    pub fn command(&self) -> Command {
        self.command.clone()
    }

    pub const fn wraps(&self) -> bool {
        self.wraps
    }
}
