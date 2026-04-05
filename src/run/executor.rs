use std::process;

use crate::{
    dev::log::elog,
    os::pkg::Package,
    run::{Command, Transaction, TransactionCommandStatus},
};

pub mod meta;

// TODO Should this be a method of Transaction instead?
pub fn commit(transaction: &mut Transaction) -> Result<(), Error> {
    elog(&format!("Committing transaction: {transaction:#?}"));
    for command in &mut transaction.commands {
        if let Err(error) = spawn(&command.run) {
            command.status = TransactionCommandStatus::PendingRollback;
            command.push_error(&error);
            if let Err(rollback_error) = spawn(&command.rollback) {
                command.status = TransactionCommandStatus::FailedRollback;
                command.push_error(&rollback_error);
                elog(&format!("Failed rollback of command {:#?}", &command));
                return Err(rollback_error);
            } else {
                command.status = TransactionCommandStatus::Rolledback;
                elog(&format!("Successfully rolled back command {:#?}", &command));
                return Err(error);
            }
        } else {
            command.status = TransactionCommandStatus::Success;
            elog(&format!("Successfully ran command {:#?}", &command));
        }
    }

    elog("Transaction committed");
    Ok(())
}

pub(super) fn spawn(command: &Command) -> Result<(), Error> {
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
    if command.escalated() {
        return Err(Error {
            message: "Read function is strictly rootless".to_string(),
            kind: ErrorKind::RootlessReadOnly,
        });
    }

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

pub fn print(message: &str) -> Result<(), Error> {
    println!("{message}");
    Ok(())
}

pub(super) fn print_packages(packages: Vec<Package>) -> Result<(), Error> {
    for package in packages {
        print(&format!("{package}"))?;
    }
    Ok(())
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub kind: ErrorKind,
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    CommandNotFound,
    RootlessReadOnly,
    #[default]
    Unknown,
    FailedSpawn,
    ChildExit,
    DirtyExit,
    UTF8,
    IO,
}
