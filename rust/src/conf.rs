use std::{collections::HashMap, fs::{self, DirEntry}, path::PathBuf};

use crate::{run::Command, log::{self, elog}};

pub fn load() -> Configuration {
    log::elog("Loading configuration");

    // DONE A3.1. Before parsing the user arguments, a configuration file at
    //       $XDG_CONFIG_DIR/tori/tori.conf MUST be read for a line such as:
    //       'su_command = doas'.
    // DONE A4.2. If this line is not found, the su_command MUST default to 'su -c'.
    // DONE A3.3. If it is found, the su_command used MUST be whatever was specified.
    // DONE A3.4. Whatever su_command MUST be validated once for presence at the path
    //       provided or obtained from $PATH and filesystem permission to execute

    let mut conf = Configuration {
        su_command: Command {
            base: "su".into(),
            args: vec!["-c".into(), "{% command %}".into()],
        }
    };

    let root = get_root();
    let Ok(contents) = fs::read_to_string(root.join("tori.conf")) else {
        eprintln!("Failed reading configuration file at {root:?}");
        return conf
    };

    let lines: Vec<Vec<String>> = contents.lines()
        .map(|line| line.split('=')
        .map(|s| s.trim().to_string()).collect()).collect();

    let mut map: HashMap<String, String> = HashMap::new();

    for line in &lines {
        if let Some(key) = line.first() && let Some(value) = line.last() {
            map.insert(key.clone(), value.clone());
        }
    }

    elog(&format!("{lines:#?}"));

    if let Some(su_command) = map.get("su_command") {
        let split: Vec<String> = su_command.split(' ')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()).collect();

        if let Some((base, args)) = split.split_first() && let Ok(resolved_path) = resolve_from_path(base) {
            elog(&format!("Succesfully resolved 'su_command' configuration value {su_command} through PATH to {resolved_path:?}, with base {base} and args {args:?}"));
            conf.su_command = Command { base: base.clone(), args: args.to_vec() }
        } else {
            eprintln!("Failed validation of 'su_command' configuration value");
        }
    }

    conf
}

fn get_root() -> PathBuf {
    if let Ok(xdg_config_dir) = std::env::var("XDG_CONFIG_DIR") {
        let mut root = PathBuf::from(xdg_config_dir);
        root.push("tori");
        root
    } else {
        if let Some(mut root) = std::env::home_dir() {
            root.push(".config");
            root.push("tori");
            root
        } else {
            if let Ok(user) = std::env::var("USER") {
                let mut root = PathBuf::from("/home");
                root.push(user);
                root.push(".config");
                root.push("tori");
                root
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
            .filter(|p| !p.is_empty() && PathBuf::from(p).is_dir()).map(PathBuf::from).collect()
    } else {
        elog("Error: PATH is not set");
        return Err("{command} not found: PATH is not set in the environment".to_string())
    };

    elog(&format!("Gathered paths {paths:?}"));
    for path in paths {
        elog(&format!("On path {path:?}"));
        let Ok(mut entries) = fs::read_dir(path) else {
            elog("Skipping: Could not read directory contents");
            continue
        };

        let filter = |candidate: &Result<DirEntry, std::io::Error>| -> bool {
            if let Ok(entry) = candidate {
                entry.path().is_file() && entry.file_name() == command
            } else { false }
        };

        let Some(filtered) = entries.find(filter) else {
            elog("Skipping: No entries passed filter");
            continue
        };

        if let Ok(found) = filtered {
            return Ok(found.path())
        } else {
            elog("Skipping: Filtered match is Err");
            continue
        };

    }
    Err("{command} not found in any of the directories in PATH".to_string())
}

#[derive(Debug, Clone)]
pub struct Configuration {
    pub su_command: Command,
}


