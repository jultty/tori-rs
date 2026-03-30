use std::process;

use crate::{os::pkg::Package, run::Command};

pub mod meta;

pub fn print(message: &str) -> Result<(), Error> {
    println!("{message}");
    Ok(())
}

pub fn print_packages(packages: Vec<Package>) -> Result<(), Error> {
    for package in packages {
        print(&format!("{package}"))?;
    }
    Ok(())
}

pub fn spawn(command: &Command) -> Result<(), Error> {
    if let Ok(mut child) = process::Command::new(&command.base)
        .args(&command.args)
        .spawn()
    {
        let Ok(exit_status) = child.wait() else {
            return Err(Error {
                message: format!("Error while waiting for child to exit given {command:?}"),
                kind: ErrorKind::ChildExit,
            });
        };
        if exit_status.success() {
            Ok(())
        } else {
            Err(Error {
                message: format!("Command {command:?} did not exit with success"),
                kind: ErrorKind::DirtyExit,
            })
        }
    } else {
        Err(Error {
            message: format!("Failed to spawn child for command {command:?}"),
            kind: ErrorKind::FailedSpawn,
        })
    }
}

pub fn read(command: &Command) -> Result<String, Error> {
    if let Ok(output) = process::Command::new(&command.base)
        .args(&command.args)
        .output()
    {
        if let Ok(utf8) = String::from_utf8(output.stdout) {
            Ok(utf8)
        } else {
            let message = format!("Failed UTF8 coversion of {command:?} output");
            eprintln!("{message}");
            Err(Error {
                message,
                kind: ErrorKind::UTF8,
            })
        }
    } else {
        let message = format!("Failed executing or reading output of {command:?}");
        eprintln!("{message}");
        Err(Error {
            message,
            kind: ErrorKind::IO,
        })
    }
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    CommandNotFound,
    Unknown,
    FailedSpawn,
    ChildExit,
    DirtyExit,
    UTF8,
    IO,
}
