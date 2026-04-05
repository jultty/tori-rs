use crate::{conf::Configuration, dev::log::elog};

pub mod executor;
pub mod expeditor;
pub mod teller;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Order {
    tasks: Vec<Task>,
}

impl Order {
    pub fn finished(&self) -> bool {
        if self.tasks.is_empty() {
            true
        } else {
            self.tasks.iter().all(|e| e.done)
        }
    }

    pub const fn tasks(&self) -> &Vec<Task> {
        &self.tasks
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Task {
    kind: TaskKind,
    done: bool,
    argument: String,
    parameters: Vec<String>,
}

impl Task {
    fn new(kind: TaskKind, argument: &str, parameters: Vec<String>) -> Task {
        Task {
            kind,
            done: false,
            argument: String::from(argument),
            parameters,
        }
    }

    pub fn argument(&self) -> &str {
        &self.argument
    }

    pub const fn parameters(&self) -> &Vec<String> {
        &self.parameters
    }

    pub const fn done(&self) -> bool {
        self.done
    }

    pub const fn kind(&self) -> &TaskKind {
        &self.kind
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum TaskKind {
    Version,
    Help,
    PackageInstall,
    PackageUninstall,
    PackageListAuto,
    PackageListManual,
    #[default]
    Unrecognized,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub base: String,
    pub args: Vec<String>,
    escalated: bool,
}

impl Command {
    pub fn escalate(&self, config: &Configuration) -> Result<Command, Error> {
        let mut args = config.su_command.command().clone().args;

        if config.su_command.wraps() {
            let flattened_command = format!("{} {}", self.base, self.args.join(" "));
            let marker_index = args
                .iter()
                .position(|s| s.replace(" ", "") == "{%command%}");
            if let Some(index) = marker_index
                && let Some(marker) = args.get_mut(index)
            {
                *marker = flattened_command;
            } else {
                let message = "Could not replace command marker in su command from configuration";
                elog(message);
                return Err(Error {
                    message: message.to_string(),
                    kind: ErrorKind::BadSuCommandConfig,
                });
            }
        } else {
            args.push(self.base.clone());
            args.extend_from_slice(&self.args);
        }

        Ok(Command {
            base: config.su_command.command().clone().base,
            escalated: true,
            args,
        })
    }

    pub fn new(base: &str, args: &[&str]) -> Command {
        Command {
            base: base.to_string(),
            args: args.iter().map(|e| e.to_string()).collect(),
            escalated: false,
        }
    }

    pub const fn escalated(&self) -> bool {
        self.escalated
    }
}

#[must_use]
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Transaction {
    commands: Vec<TransactionCommand>,
}

impl Transaction {
    pub fn single(command: &TransactionCommand) -> Transaction {
        Transaction {
            commands: vec![command.clone()],
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct TransactionCommand {
    run: Command,
    rollback: Command,
    status: TransactionCommandStatus,
    errors: Option<Vec<executor::Error>>,
}

impl TransactionCommand {
    pub const fn new(run: Command, rollback: Command) -> TransactionCommand {
        TransactionCommand {
            run,
            rollback,
            status: TransactionCommandStatus::Pending,
            errors: None,
        }
    }

    pub fn push_error(&mut self, error: &executor::Error) {
        self.errors.get_or_insert_with(Vec::new).push(error.clone());
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum TransactionCommandStatus {
    #[default]
    Pending,
    Success,
    PendingRollback,
    Rolledback,
    FailedRollback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    BadSuCommandConfig,
}
