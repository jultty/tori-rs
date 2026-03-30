use core::{
    clone::Clone,
    cmp::{Eq, PartialEq},
    default::Default,
    fmt::Debug,
};

use crate::{conf::Configuration, os::debian, run};

pub trait Packages: Clone + Default + Debug + PartialEq + Eq {
    fn install(&self, packages: &[Package], config: &Configuration) -> Result<(), Error>;
    fn uninstall(&self, packages: &[Package], config: &Configuration) -> Result<(), Error>;
    fn manual(&self) -> Result<Vec<Package>, Error>;
    fn automatic(&self) -> Result<Vec<Package>, Error>;
    fn variant(&self) -> Result<PackagerVariant, Error>;
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum Packager {
    Apt(debian::Apt),
    #[default]
    Unknown,
}

impl Packages for Packager {
    fn install(&self, packages: &[Package], config: &Configuration) -> Result<(), Error> {
        match self {
            Packager::Apt(p) => p.install(packages, config),
            Packager::Unknown => Error::unknown_packager(&format!("install {packages:?}")),
        }
    }

    fn uninstall(&self, packages: &[Package], config: &Configuration) -> Result<(), Error> {
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

    fn variant(&self) -> Result<PackagerVariant, Error> {
        match self {
            Packager::Apt(p) => p.variant(),
            Packager::Unknown => Error::unknown_packager(
                "Can't determine the package manager's variant because it is unknown",
            ),
        }
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub enum PackagerVariant {
    Apt,
    #[default]
    Unknown,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Package {
    name: String,
    version: Option<Version>,
    manual: Option<bool>,
}

impl Package {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn new_with_manual(name: &str, manual: bool) -> Package {
        Package {
            name: name.to_string(),
            version: None,
            manual: Some(manual),
        }
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
        if let Some(version) = &self.version {
            write!(f, "{} {}", &self.name, version)
        } else {
            write!(f, "{}", &self.name)
        }
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
struct Version {
    major: u32,
    minor: Option<u32>,
    patch: Option<u32>,
    qualifier: Option<String>,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(minor) = &self.minor
            && let Some(patch) = &self.patch
            && let Some(qualifier) = &self.qualifier
        {
            write!(f, "{}.{minor}.{patch}-{qualifier}", &self.major)
        } else if let Some(minor) = &self.minor
            && let Some(patch) = &self.patch
        {
            write!(f, "{}.{minor}.{patch}", &self.major)
        } else if let Some(minor) = &self.minor {
            write!(f, "{}.{minor}", &self.major)
        } else {
            write!(f, "{}", &self.major)
        }
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

    pub fn send<T>(message: &str, kind: ErrorKind) -> Result<T, Error> {
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

#[derive(Debug, Clone)]
pub enum ErrorKind {
    NotFound,
    UnknownPackager,
    MetadataFileRead,
    RunError,
    ExecutorError,
}
