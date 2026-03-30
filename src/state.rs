use std::collections::HashMap;

use crate::{
    conf::Configuration,
    log::elog,
    os::OperatingSystem,
    run::{Command, Order, executor::read},
};

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct State {
    configuration: Configuration,
    os: OperatingSystem,
    orders: Vec<Order>,
}

impl State {
    fn new(config: &Configuration, os: &OperatingSystem, orders: &[Order]) -> State {
        State {
            configuration: config.clone(),
            os: os.clone(),
            orders: orders.to_vec(),
        }
    }

    pub fn configuration(&self) -> Configuration {
        self.configuration.clone()
    }

    pub fn os(&self) -> OperatingSystem {
        self.os.clone()
    }

    pub fn orders(&self) -> Vec<Order> {
        self.orders.clone()
    }
}

pub fn setup(config: &Configuration, orders: &[Order]) -> State {
    State::new(config, &detect_os(), orders)
}

fn detect_os() -> OperatingSystem {
    use crate::os;

    if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
        elog(&os_release);
        let mut map: HashMap<String, String> = HashMap::new();
        let lines: Vec<Option<(&str, &str)>> = os_release
            .lines()
            .map(|line| line.split_once('='))
            .collect();
        for line in lines.into_iter().flatten() {
            let (key, value) = line;
            map.insert(key.to_string(), strip_quotes(value));
        }

        elog(&format!("os-release map: {map:#?}"));

        // TODO This should ideally exhaustively check against the possible OSs
        if let Some(os_name) = map.get("NAME") {
            if os_name == "Debian GNU/Linux" {
                return os::debian::DEBIAN;
            }
        }
    } else {
        elog("Failed reading os-release");
        if let Ok(uname_stdout) = read(&Command::new("uname", &["--operating-system"])) {
            if uname_stdout == "Debian GNU/Linux" {
                return os::debian::DEBIAN;
            }
        } else {
            elog("Failed reading uname output");
        }
    }

    elog("OS detection failed");
    OperatingSystem::unknown()
}

fn strip_quotes(original: &str) -> String {
    let no_prefix = match original.strip_prefix('"') {
        Some(stripped) => stripped,
        None => original,
    };
    let no_suffix = match no_prefix.strip_suffix('"') {
        Some(stripped) => stripped,
        None => original,
    };
    no_suffix.to_string()
}
