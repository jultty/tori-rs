// B2.1. DONE version | -v | --version -> MUST print the version as in v0.8.0
// B2.2. TODO help | -h | --help -> MUST print '<long help>'
// B2.3. TODO os -> MUST print the OS name and MUST log contents of /etc/os-release
// B2.4. TODO user -> MUST print the output of the 'whoami' command
// B2.5. TODO pkg p -> MUST call the system package manager using the su_command
//       to install and then uninstall package p. The user MUST be able to
//       freely input to these commands' interactive inputs before control
//       is returned. When done, it MUST log 'Done:', a newline, and the
//       system commands executed, one per line. If no p is provided, it
//       MUST NOT run any system commands and print a message
// B2.6. TODO echo x y z -> MUST print x y z
// B2.7. TODO echo -> MUST NOT print any output and exit with status code 0
// B2.8. DONE [no input] -> MUST NOT print any output and exit with status code 0
// B2.9. TODO [any other input] -> MUST print 'Unrecognized command: [command]',
//       a newline, '<short help>' and exit with status code 1

#[derive(Default, Debug)]
pub struct Order {
    tasks: Vec<Task>,
}

impl Order {

    pub fn fill(&mut self) {
        for task in self.tasks.iter_mut() {
            if task.done { continue }
            task.complete();
        }
    }

    fn is_complete(&self) -> bool {
        self.tasks.iter().any(|e| !e.done)
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    kind: TaskKind,
    done: bool,
    parameters: Vec<String>,
}

impl Task {
    pub fn complete(&mut self) {
        use crate::{run::exec::{meta, os, shell, pkg}};
        use TaskKind::*;

        self.done = match self.kind {
            Version => { meta::print_version() },
            _ => false, // TODO
        }
    }

    fn new(kind: TaskKind, parameters: Vec<String>) -> Task {
        Task {
            kind,
            done: false,
            parameters,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskKind {
    Version,
    OperatingSystem,
    Package,
    User,
    Echo,
}

pub mod teller {
    use crate::{log::elog, run::{Order, Task, TaskKind}};
    use std::{env, path::PathBuf};

    pub fn parse(mut raw_args: env::Args) -> Order {
        let (argument, parameters): (String, Vec<String>) = if let Some(first) = raw_args.next() {
            if is_executable_path(&first) {
                elog("First argument is the executable path");
                if let Some(second) = raw_args.next() {
                    elog(&format!(
                        "Assembled command {second}, arguments {raw_args:?}"
                    ));
                    (second, raw_args.collect())
                } else {
                    elog("No arguments provided");
                    return Order::default();
                }
            } else {
                elog("First argument is not the executable path");
                elog(&format!(
                    "Assembled command {first}, arguments {raw_args:?}"
                ));
                (first, raw_args.collect())
            }
        } else {
            elog("No arguments provided");
            return Order::default();
        };

        use TaskKind::*;

        if argument == "version" || argument == "-v" || argument == "--version" {
            elog("Command is 'version'");
            Order { tasks: vec![Task::new(Version, parameters)] }
        } else if argument == "os" {
            elog("Command is 'os'");
            Order { tasks: vec![Task::new(OperatingSystem, parameters)] }
        } else if argument == "pkg" {
            elog("Command is 'pkg'");
            Order { tasks: vec![Task::new(Package, parameters)] }
        } else if argument == "user" {
            elog("Command is 'user'");
            Order { tasks: vec![Task::new(User, parameters)] }
        } else if argument == "echo" {
            elog("Command is 'echo'");
            Order { tasks: vec![Task::new(Echo, parameters)] }
        } else {
            Order::default()
        }
    }

    fn is_executable_path(candidate: &str) -> bool {

        fn assume(message: &str) -> bool {
            elog(&format!("Assuming args[0] is the executable {message}"));
            true
        }

        let Ok(executable_path) = env::current_exe() else {
            return assume("Failed to get executable path")
        };
        let Some(executable_file) = executable_path.file_name() else {
            return assume("Executable path lacks a file component")
        };

        let argument_path = PathBuf::from(candidate);
        let Some(argument_file) = argument_path.file_name() else {
            return assume("Argument path lacks a file component")
        };

        elog(&format!(
            "Executable path: {executable_path:?}, file {executable_file:?} \
            Argument path: {argument_path:?}, file {argument_file:?} "
        ));

        if argument_path.exists() {
            if let Ok(argument_canonical) = argument_path.canonicalize()
                && let Ok(executable_canonical) = executable_path.canonicalize() {
                let judgment = argument_canonical == executable_canonical;
                elog(&format!("args[0] canonically is executable path: {judgment}"));
                judgment
            } else {
                assume("Could not canonicalize executable and argument paths")
            }
        } else {
            let judgment = argument_file == executable_file;
            elog(&format!("args[0] matches executable path by name only: {judgment}"));
            judgment
        }
    }

}

pub mod expeditor {}

pub mod exec {
    pub mod meta {
        pub fn print_version() -> bool {
            println!("v{}", env!("CARGO_PKG_VERSION"));
            true
        }
    }
    pub mod os {}
    pub mod shell {}
    pub mod pkg {}
}
