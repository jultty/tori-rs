use std::fs::read_to_string;

use crate::{
    conf::Configuration,
    log::elog,
    os::{
        OperatingSystem, Kind,
        pkg::{self, Package, PackagerVariant, Packages},
    },
    run::{Command, executor::read},
};

pub const DEBIAN: OperatingSystem = OperatingSystem {
    kind: Kind::Debian,
    packager: pkg::Packager::Apt(APT),
};

const APT: Apt = Apt {
    variant: PackagerVariant::Apt,
};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Apt {
    variant: PackagerVariant,
}

impl Packages for Apt {
    fn install(&self, packages: &[Package], config: &Configuration) -> Result<(), pkg::Error> {
        super::debian::Apt::haul("install", packages, config)
    }

    fn uninstall(&self, packages: &[Package], config: &Configuration) -> Result<(), pkg::Error> {
        super::debian::Apt::haul("remove", packages, config)
    }

    fn manual(&self) -> Result<Vec<Package>, pkg::Error> {
        let raw_all = read(&Command::new(
            "dpkg-query",
            &["--show", "--showformat", "${Package} ${Status}\\n"],
        ))?;
        let all: Vec<String> = raw_all
            .lines()
            .filter_map(|line| {
                let pair = line.split_once(' ');
                match pair {
                    Some((pkg, "install ok installed")) => Some(pkg.to_string()),
                    Some(_) => None,
                    None => {
                        elog("Warning: Dropped a None pair when cleaning up package list");
                        None
                    }
                }
            })
            .collect();

        let auto_packages: Vec<Package> = self.automatic()?;
        let mut manual_packages: Vec<Package> = vec![];

        for package in all {
            let auto = Package::new_with_manual(&package, false);
            if !auto_packages.contains(&auto) {
                manual_packages.push(Package::new_with_manual(&package, true));
            }
        }

        Ok(manual_packages)
    }

    fn automatic(&self) -> Result<Vec<Package>, pkg::Error> {
        let path = "/var/lib/apt/extended_states";
        let Ok(extended_states) = read_to_string(path) else {
            return pkg::Error::send(
                &format!("Failed reading {path}"),
                pkg::ErrorKind::MetadataFileRead,
            );
        };
        let lines: Vec<String> = extended_states
            .lines()
            .map(|s| s.to_string())
            .filter(|line| !line.is_empty())
            .collect();

        let iterator = lines.chunks_exact(3);
        let remainder = iterator.remainder();
        if !remainder.is_empty() {
            elog(&format!(
                "Warning: Reading package extended states left a remainder: {remainder:?}"
            ));
        }

        let mut packages: Vec<Package> = vec![];

        for chunk in iterator {
            if let Some(name_line) = chunk.first()
                && let Some(auto_line) = chunk.get(2)
            {
                let Some(name_key) = name_line.split(' ').nth(0) else {
                    elog(&format!(
                        "Warning: Unexpected structure for package line when \
                        reading extended states chunk {chunk:?}"
                    ));
                    continue;
                };
                if name_key != "Package:" {
                    elog(&format!(
                        "Warning: Expected package line key to be 'Package:' \
                        but found {name_key} instead in chunk {chunk:?}"
                    ));
                    continue;
                }
                let Some(name_value) = name_line.split(' ').nth(1) else {
                    elog(&format!(
                        "Warning: No package name when reading extended states chunk {chunk:?}"
                    ));
                    continue;
                };

                let Some(auto_key) = auto_line.split(' ').nth(0) else {
                    elog(&format!(
                        "Warning: Unexpected structure for auto-installed line \
                        when reading extended states chunk {chunk:?}"
                    ));
                    continue;
                };
                if auto_key != "Auto-Installed:" {
                    elog(&format!(
                        "Warning: Expected auto-installed line key to be 'Auto-Installed:' \
                        but found {auto_key} instead in chunk {chunk:?}"
                    ));
                    continue;
                }
                let Some(auto_value) = auto_line.split(' ').nth(1) else {
                    elog(&format!(
                        "Warning: No auto-installed value when reading extended states chunk {chunk:?}"
                    ));
                    continue;
                };

                packages.push(Package::new_with_manual(name_value, auto_value == "0"));
            }
        }

        Ok(packages)
    }

    fn variant(&self) -> Result<PackagerVariant, pkg::Error> {
        Ok(self.variant.clone())
    }
}

impl Apt {
    fn haul(
        subcommand: &str,
        packages: &[Package],
        config: &Configuration,
    ) -> Result<(), pkg::Error> {
        if packages.is_empty() {
            println!("Package selection is empty: Nothing to {subcommand}");
            return Ok(());
        }

        let mut args = vec![subcommand];
        args.extend_from_slice(&packages.iter().map(|p| p.into()).collect::<Vec<&str>>());

        let command = Command::new("apt", &args).escalate(config)?;

        Ok(crate::run::executor::spawn(&command)?)
    }
}
