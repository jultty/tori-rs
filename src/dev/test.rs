use std::{env, fs, io, path::PathBuf};

use crate::{dev::log::elog, conf};

#[derive(Debug)]
pub struct Directories {
    pub original: PathBuf,
    pub tube: PathBuf,
    pub conf: PathBuf,
    pub conf_root: PathBuf,
}

impl Directories {
    /// Sets up self-cleaning original, temporary and 'templates' directories.
    ///
    /// # Errors
    /// May return Error when:
    /// - Current directory does not exist or lacking permissions
    /// - Several I/O possibilities from directory creation failures
    /// - Several I/O possibilities from working directory changing failures
    pub fn setup(dir_name: &str) -> Result<Directories, Error> {
        let original = env::current_dir()?;
        let tube = original.join(format!("target/tubes/{dir_name}"));
        let xdg_conf = tube.join(".config");
        let conf_root = xdg_conf.join("tori");
        let conf = conf_root.join("tori.conf");

        drop(fs::remove_dir_all(&tube));

        if let Err(error) = fs::create_dir_all(&conf_root) {
            return Err(Error::with_io(
                "Failed configuration root directory creation",
                error,
            ))
        }

        if let Err(error) = env::set_current_dir(&tube) {
            return Err(Error::with_io("Failed current directory change", error))
        }

        unsafe { env::set_var("XDG_CONFIG_DIR", &xdg_conf); }

        Ok(Directories {
            original,
            tube,
            conf,
            conf_root,
        })
    }
}

impl Drop for Directories {
    fn drop(&mut self) {
        if let Err(error) = std::env::set_current_dir(&self.original) {
            elog(&format!("Couldn't reset to original directory: {error}"));
        }
        if let Err(error) = std::fs::remove_dir_all(&self.tube) {
            elog(&format!("Couldn't cleanup tube directory: {error}"));
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub inner: Option<InnerErrors>,
}

#[derive(Debug, Default)]
pub struct InnerErrors {
    pub io: Option<io::Error>,
    pub conf: Option<conf::Error>,
}

impl Error {
    fn with_io(message: &str, inner: io::Error) -> Error {
        Error {
            message: String::from(message),
            inner: Some(InnerErrors { io: Some(inner), conf: None }),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut message = self.message.clone();

        if let Some(inner) = &self.inner {
            message = format!("{message}\n{inner:#?}");
        }

        write!(f, "{message}")
    }
}

impl From<String> for Error {
    fn from(string: String) -> Error {
        Error {
            message: string,
            inner: None,
        }
    }
}

impl From<&str> for Error {
    fn from(str: &str) -> Error { Error::from(String::from(str)) }
}

impl From<io::Error> for Error {
    fn from(inner: io::Error) -> Error {
        let mut error = Error::from(inner.to_string());
        error.inner = Some(InnerErrors { io: Some(inner), ..InnerErrors::default() });
        error
    }
}

impl From<conf::Error> for Error {
    fn from(conf_error: conf::Error) -> Error {
        Error {
            message: conf_error.message.clone(),
            inner: Some(InnerErrors { conf: Some(conf_error), io: None }),
        }
    }
}
