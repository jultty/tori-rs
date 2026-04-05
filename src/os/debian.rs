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

        let auto_set: HashSet<String> = self
            .automatic()?
            .into_iter()
            .map(|package| package.name().to_owned())
            .collect();

        Ok(Apt::determine_manual(&raw_all, &auto_set))
    }

    fn automatic(&self) -> Result<Vec<Package>, pkg::Error> {
        Ok(Apt::determine_auto(&read_to_string(
            "/var/lib/apt/extended_states",
        )?))
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

    fn determine_manual(raw_all: &str, auto_set: &HashSet<String>) -> Vec<Package> {
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

        let mut manual_packages: Vec<Package> = all
            .into_iter()
            .filter(|name| !auto_set.contains(name))
            .map(|name| Package::new_with_manual(&name, true))
            .collect();

        manual_packages.sort();
        manual_packages
    }

    fn determine_auto(extended_states: &str) -> Vec<Package> {
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

                if auto_value == "1" {
                    packages.push(Package::new_with_manual(name_value, auto_value == "0"));
                } else {
                    elog(&format!(
                        "Skipping: Package {name_value} has an auto-installed value different from 1"
                    ));
                }
            }
        }

        packages.sort();
        packages
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determine_manual_given_empty_auto_set() {
        let raw_all = "avocado install ok installed\n\
            turnip install ok installed\n\
            carrot install ok installed\n\
            sunflower install ok installed\n\
            pumpkin install ok installed\n\
            squash install ok installed";

        let auto_set: HashSet<String> = HashSet::default();

        let manual = Apt::determine_manual(raw_all, &auto_set);
        assert_eq!(
            manual,
            vec![
                "avocado".into(),
                "carrot".into(),
                "pumpkin".into(),
                "squash".into(),
                "sunflower".into(),
                "turnip".into(),
            ]
        );
    }

    #[test]
    fn determine_manual_given_nonempty_auto_set() {
        let raw_all = "avocado install ok installed\n\
            turnip install ok installed\n\
            carrot install ok installed\n\
            sunflower install ok installed\n\
            pumpkin install ok installed\n\
            squash install ok installed";

        let mut auto_set: HashSet<String> = HashSet::default();
        auto_set.insert("sunflower".to_string());
        auto_set.insert("turnip".to_string());

        let manual = Apt::determine_manual(raw_all, &auto_set);
        assert_eq!(
            manual,
            vec![
                "avocado".into(),
                "carrot".into(),
                "pumpkin".into(),
                "squash".into(),
            ]
        );
    }

    #[test]
    fn determine_manual_given_empty_raw_input() {
        let raw_all = "";

        let mut auto_set: HashSet<String> = HashSet::default();
        auto_set.insert("sunflower".to_string());
        auto_set.insert("turnip".to_string());

        let manual = Apt::determine_manual(raw_all, &auto_set);
        assert!(manual.is_empty());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn determine_manual_given_empty_auto_set(
            package_1 in "[a-zA-Z0-9-_]{1,24}",
            package_2 in "[a-zA-Z0-9-_]{1,24}",
            package_3 in "[a-zA-Z0-9-_]{1,24}",
            package_4 in "[a-zA-Z0-9-_]{1,24}",
            package_5 in "[a-zA-Z0-9-_]{1,24}",
            package_6 in "[a-zA-Z0-9-_]{1,24}",
        ) {
            let raw_all = format!("{package_1} install ok installed\n\
                {package_2} install ok installed\n\
                {package_3} install ok installed\n\
                {package_4} install ok installed\n\
                {package_5} install ok installed\n\
                {package_6} install ok installed");

            let auto_set: HashSet<String> = HashSet::default();

            let manual = Apt::determine_manual(&raw_all, &auto_set);

            let mut actual: Vec<String> = manual
                .iter().map(|p| p.name().to_string()).collect();
            actual.sort();
            let mut expected = vec![
                package_1,
                package_2,
                package_3,
                package_4,
                package_5,
                package_6,
            ];
            expected.sort();
            assert_eq!(actual, expected);
        }
    }

    proptest! {
        #[test]
        fn determine_manual_given_nonempty_auto_set(
            package_1 in "[a-zA-Z0-9-_]{1,24}",
            package_2 in "[a-zA-Z0-9-_]{1,24}",
            package_3 in "[a-zA-Z0-9-_]{1,24}",
            package_4 in "[a-z]{1,8}\\p{Cyrillic}{1,8}\\p{Greek}{1,24}",
            package_5 in "[a-z]{1,8}\\p{Cyrillic}{1,8}\\p{Greek}{1,24}",
            package_6 in "[a-z]{1,8}\\p{Cyrillic}{1,8}\\p{Greek}{1,24}",
        ) {
            let mut args = [&package_1,
                &package_2,
                &package_3,
                &package_4,
                &package_5,
                &package_6
            ];
            let (_, dupes) = &args.partition_dedup();
            prop_assume!(dupes.is_empty());

            let raw_all = format!("{package_1} install ok installed\n\
                {package_2} install ok installed\n\
                {package_3} install ok installed\n\
                {package_4} install ok installed\n\
                {package_5} install ok installed\n\
                {package_6} install ok installed");

            println!("raw_all: <{raw_all}>");

            let mut auto_set: HashSet<String> = HashSet::default();
            auto_set.insert(package_1);
            auto_set.insert(package_3);
            auto_set.insert(package_5);
            println!("auto_set: <{auto_set:#?}>");

            let manual = Apt::determine_manual(&raw_all, &auto_set);
            println!("manual: <{manual:#?}>");

            let mut actual: Vec<String> = manual
                .iter().map(|p| p.name().to_string()).collect();
            actual.sort();
            let mut expected = vec![package_2, package_4, package_6];
            expected.sort();
            assert_eq!(actual, expected);
        }
    }

    proptest! {
        #[test]
        fn determine_manual_given_empty_raw_input(
            auto_package_1 in "[a-zA-Z0-9-_]{1,24}",
            auto_package_2 in "[a-zA-Z0-9-_]{1,24}",
            auto_package_3 in "",
            auto_package_4 in "[a-z]{1,4}\\p{Cyrillic}{1,4}\\p{Greek}{1,24}",
        ) {
            let raw_all = "";

            let mut auto_set: HashSet<String> = HashSet::default();
            auto_set.insert(auto_package_1);
            auto_set.insert(auto_package_2);
            auto_set.insert(auto_package_3);
            auto_set.insert(auto_package_4);

            let manual = Apt::determine_manual(raw_all, &auto_set);
            assert!(manual.is_empty());
        }
    }
}
