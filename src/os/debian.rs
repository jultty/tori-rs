use std::{collections::HashSet, fs::read_to_string, iter};

use crate::{
    conf::Configuration,
    log::elog,
    os::{
        Kind, OperatingSystem,
        pkg::{self, Package, PackagerVariant, Packages},
    },
    run::{Command, Transaction, TransactionCommand, executor::read},
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
    fn install(
        &self,
        packages: &[Package],
        config: &Configuration,
    ) -> Result<Transaction, pkg::Error> {
        super::debian::Apt::haul(&Operation::Install, packages, config)
    }

    fn uninstall(
        &self,
        packages: &[Package],
        config: &Configuration,
    ) -> Result<Transaction, pkg::Error> {
        super::debian::Apt::haul(&Operation::Uninstall, packages, config)
    }

    fn manual(&self) -> Result<Vec<Package>, pkg::Error> {
        let raw_all = read(&Command::new(
            "dpkg-query",
            &["--show", "--showformat", "${Package} ${Status}\\n"],
        ))?;
        let all = raw_all.lines().filter_map(|line| {
            let pair = line.split_once(' ');
            match pair {
                Some((pkg, "install ok installed")) => Some(pkg.to_string()),
                Some(_) => None,
                None => {
                    elog("Warning: Dropped a None pair when cleaning up package list");
                    None
                }
            }
        });

        let auto_set: HashSet<String> = self
            .automatic()?
            .into_iter()
            .map(|package| package.name().to_owned())
            .collect();
        let mut manual_packages: Vec<Package> = all
            .into_iter()
            .filter(|name| !auto_set.contains(name))
            .map(|name| Package::new_with_manual(&name, true))
            .collect();

        manual_packages.sort();
        Ok(manual_packages)
    }

    fn automatic(&self) -> Result<Vec<Package>, pkg::Error> {
        let path = "/var/lib/apt/extended_states";
        let extended_states = read_to_string(path)?;

        let lines: Vec<&str> = extended_states
            .lines()
            .filter(|line| !line.is_empty())
            .collect();

        let chunks = lines.chunks_exact(3);
        if !chunks.remainder().is_empty() {
            elog(&format!(
                "Warning: Package extended states read left a remainder: {:?}",
                chunks.remainder()
            ));
        }

        let mut packages: Vec<Package> = vec![];

        for chunk in chunks {
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

        packages.sort();
        Ok(packages)
    }

    fn variant(&self) -> &PackagerVariant {
        &self.variant
    }
}

impl Apt {
    fn haul(
        operation: &Operation,
        packages: &[Package],
        config: &Configuration,
    ) -> Result<Transaction, pkg::Error> {
        if packages.is_empty() {
            println!("Package selection is empty: Nothing to {operation}");
            return Ok(Transaction::default());
        }

        // TODO This works as it is stated and is interesting as part of the
        // PoC, but doesn't really make sense to install something that wasn't
        // installed in the first place as the "rollback" of a failed uninstall
        let rollback_operation = match operation {
            Operation::Install => Operation::Uninstall,
            Operation::Uninstall => Operation::Install,
        };

        let run_args: Vec<&str> = iter::once(operation.into())
            .chain(packages.iter().map(|p| p.into()))
            .collect();

        let rollback_args: Vec<&str> = iter::once(rollback_operation.into())
            .chain(packages.iter().map(|p| p.into()))
            .collect();

        let run = Command::new("apt", &run_args).escalate(config)?;
        let rollback = Command::new("apt", &rollback_args).escalate(config)?;
        let transaction_command = TransactionCommand::new(run, rollback);
        Ok(Transaction::single(&transaction_command))
    }
}

enum Operation {
    Install,
    Uninstall,
}

impl<'s> From<Operation> for &'s str {
    fn from(operation: Operation) -> &'s str {
        match operation {
            Operation::Install => "install",
            Operation::Uninstall => "remove",
        }
    }
}

impl<'s> From<&'s Operation> for &'s str {
    fn from(operation: &Operation) -> &str {
        match *operation {
            Operation::Install => "install",
            Operation::Uninstall => "remove",
        }
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s: &str = self.into();
        write!(f, "{s}")
    }
}
