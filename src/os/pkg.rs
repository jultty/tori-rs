use core::{
    clone::Clone,
    cmp::{Eq, PartialEq},
    default::Default,
    fmt::Debug,
};

use crate::{
    conf::Configuration,
    os::debian,
    run::{self, Transaction},
};

pub trait Packages: Clone + Default + Debug + PartialEq + Eq {
    fn install(&self, packages: &[Package], config: &Configuration) -> Result<Transaction, Error>;
    fn uninstall(&self, packages: &[Package], config: &Configuration)
    -> Result<Transaction, Error>;
    fn manual(&self) -> Result<Vec<Package>, Error>;
    fn automatic(&self) -> Result<Vec<Package>, Error>;
    fn variant(&self) -> &PackagerVariant;
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum Packager {
    Apt(debian::Apt),
    #[default]
    Unknown,
}

impl Packages for Packager {
    fn install(&self, packages: &[Package], config: &Configuration) -> Result<Transaction, Error> {
        match self {
            Packager::Apt(p) => p.install(packages, config),
            Packager::Unknown => Error::unknown_packager(&format!("install {packages:?}")),
        }
    }

    fn uninstall(
        &self,
        packages: &[Package],
        config: &Configuration,
    ) -> Result<Transaction, Error> {
        match self {
            Packager::Apt(p) => p.uninstall(packages, config),
            Packager::Unknown => Error::unknown_packager(&format!("uninstall {packages:?}")),
        }
    }

    fn manual(&self) -> Result<Vec<Package>, Error> {
        match self {
            Packager::Apt(p) => p.manual(),
            Packager::Unknown => Error::unknown_packager("list manually-installed packages"),
        }
    }

    fn automatic(&self) -> Result<Vec<Package>, Error> {
        match self {
            Packager::Apt(p) => p.automatic(),
            Packager::Unknown => Error::unknown_packager("list automatically-installed packages"),
        }
    }

    fn variant(&self) -> &PackagerVariant {
        match self {
            Packager::Apt(p) => p.variant(),
            Packager::Unknown => &PackagerVariant::Unknown,
        }
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub enum PackagerVariant {
    Apt,
    #[default]
    Unknown,
}

#[derive(Default, Debug, Clone, Eq)]
pub struct Package {
    name: String,
    manual: Option<bool>,
}

impl Package {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn manual(&self) -> &Option<bool> {
        &self.manual
    }

    pub fn new_with_manual(name: &str, manual: bool) -> Package {
        Package {
            name: name.to_string(),
            manual: Some(manual),
        }
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Package) -> bool {
        self.name() == other.name()
    }
}

impl std::hash::Hash for Package {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name().hash(state);
    }
}

impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Package) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Package {
    fn cmp(&self, other: &Package) -> std::cmp::Ordering {
        self.name().cmp(other.name())
    }
}

impl From<&String> for Package {
    fn from(s: &String) -> Package {
        Package {
            name: s.clone(),
            ..Package::default()
        }
    }
}

impl From<String> for Package {
    fn from(s: String) -> Package {
        Package {
            name: s,
            ..Package::default()
        }
    }
}

impl From<&str> for Package {
    fn from(s: &str) -> Package {
        Package {
            name: s.to_string(),
            ..Package::default()
        }
    }
}

impl From<Package> for String {
    fn from(p: Package) -> String {
        p.name
    }
}

impl From<&Package> for String {
    fn from(p: &Package) -> String {
        p.name.clone()
    }
}

impl<'s> From<&'s Package> for &'s str {
    fn from(p: &'s Package) -> &'s str {
        &p.name
    }
}

impl std::fmt::Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}

#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
    pub kind: ErrorKind,
}

impl Error {
    pub fn new(message: &str, kind: ErrorKind) -> Error {
        Error {
            message: message.to_string(),
            kind,
        }
    }

    pub fn wrapped<T>(message: &str, kind: ErrorKind) -> Result<T, Error> {
        Err(Error::new(message, kind))
    }

    fn unknown_packager<T>(action: &str) -> Result<T, Error> {
        Err(Error {
            message: format!("Can't {action} because package manager is unknown"),
            kind: ErrorKind::UnknownPackager,
        })
    }
}

impl From<run::executor::Error> for Error {
    fn from(executor_error: run::executor::Error) -> Error {
        Error {
            message: format!("{:?}: {}", executor_error.kind, executor_error.message),
            kind: ErrorKind::ExecutorError,
        }
    }
}

impl From<run::Error> for Error {
    fn from(run_error: run::Error) -> Error {
        Error {
            message: format!("{:?}: {}", run_error.kind, run_error.message),
            kind: ErrorKind::RunError,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(io_error: std::io::Error) -> Error {
        Error {
            message: format!("{:?}: {}", io_error.kind(), io_error),
            kind: ErrorKind::IO,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    NotFound,
    UnknownPackager,
    MetadataFileRead,
    RunError,
    ExecutorError,
    IO,
}
