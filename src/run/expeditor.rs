use crate::{
    os::pkg::{self, Package, Packages as _},
    run::{TaskKind, executor},
    state::State,
};

pub fn fulfill(state: &State) -> Result<(), Error> {
    let orders = state.orders();

    for order in orders {
        if order.finished() {
            continue;
        }
        for task in order.tasks().iter().filter(|t| !t.done()) {
            match task.kind() {
                TaskKind::Version => executor::print(env!("CARGO_PKG_VERSION"))?,
                TaskKind::Help => executor::print("<long help>")?,
                TaskKind::PackageInstall => {
                    let packages: Vec<Package> = task.parameters.iter().map(|s| s.into()).collect();
                    state
                        .os()
                        .packager()
                        .install(&packages, state.configuration())?;
                }
                TaskKind::PackageUninstall => {
                    let packages: Vec<Package> = task.parameters.iter().map(|s| s.into()).collect();
                    state
                        .os()
                        .packager()
                        .uninstall(&packages, state.configuration())?;
                }
                TaskKind::PackageListAuto => {
                    match state.os().packager().automatic() {
                        Ok(packages) => Ok::<(), Error>(executor::print_packages(packages)?),
                        Err(error) => {
                            executor::print(&format!(
                                "Error gathering automatically-installed packages: {error:?}",
                            ))?;
                            Err(error.into())
                        }
                    }?;
                }
                TaskKind::PackageListManual => {
                    match state.os().packager().manual() {
                        Ok(packages) => Ok::<(), Error>(executor::print_packages(packages)?),
                        Err(error) => {
                            executor::print(&format!(
                                "Error gathering manually-installed packages: {error:?}",
                            ))?;
                            Err(error.into())
                        }
                    }?;
                }
                TaskKind::Unrecognized => executor::print(&format!(
                    "Unrecognized command: {}\n<short help>",
                    task.argument()
                ))?,
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub kind: ErrorKind,
}

impl From<executor::Error> for Error {
    fn from(executor_error: executor::Error) -> Error {
        Error {
            message: format!("{:?}: {}", executor_error.kind, executor_error.message),
            kind: ErrorKind::ExecutorError,
        }
    }
}

impl From<pkg::Error> for Error {
    fn from(pkg_error: pkg::Error) -> Error {
        Error {
            message: format!("{:?}: {}", pkg_error.kind, pkg_error.message),
            kind: ErrorKind::PackagingError,
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    ExecutorError,
    PackagingError,
    PackagerUnknown,
    OsUnknown,
}
