use pkg::Packager;

pub mod pkg;

pub mod debian;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct OperatingSystem {
    kind: Kind,
    packager: Packager,
}

impl OperatingSystem {
    pub fn kind(&self) -> Kind {
        self.kind.clone()
    }

    pub fn packager(&self) -> Packager {
        self.packager.clone()
    }

    pub const fn new(kind: Kind, packager: Packager) -> OperatingSystem {
        OperatingSystem { kind, packager }
    }

    pub const fn unknown() -> OperatingSystem {
        OperatingSystem {
            kind: Kind::Unknown,
            packager: Packager::Unknown,
        }
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub enum Kind {
    #[default]
    Unknown,
    Debian,
}
