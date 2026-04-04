use crate::{conf::Configuration, log::elog};

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
            args,
        })
    }

    pub fn new(base: &str, args: &[&str]) -> Command {
        Command {
            base: base.to_string(),
            args: args.iter().map(|e| e.to_string()).collect(),
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    BadSuCommandConfig,
}
