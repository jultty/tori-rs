// B2.1. DONE version | -v | --version -> MUST print the version as in v0.8.0
// B2.2. DONE help | -h | --help -> MUST print '<long help>'
// B2.3. DONE os -> MUST print the OS name and MUST log contents of /etc/os-release
// B2.4. DONE user -> MUST print the output of the 'whoami' command
// B2.5. TODO pkg p -> MUST call the system package manager using the su_command
//       to install and then uninstall package p. The user MUST be able to
//       freely input to these commands' interactive inputs before control
//       is returned. When done, it MUST log 'Done:', a newline, and the
//       system commands executed, one per line. If no p is provided, it
//       MUST NOT run any system commands and print a message
// B2.6. DONE echo x y z -> MUST print x y z
// B2.7. DONE echo -> MUST NOT print any output and exit with status code 0
// B2.8. DONE [no input] -> MUST NOT print any output and exit with status code 0
// B2.9. DONE [any other input] -> MUST print 'Unrecognized command: [command]',
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

    pub fn finished(&self) -> bool {
        if self.tasks.is_empty() {
            true
        } else {
            self.tasks.iter().all(|e| e.done)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    kind: TaskKind,
    done: bool,
    argument: String,
    parameters: Vec<String>,
}

impl Task {
    pub fn complete(&mut self) {
        use crate::{run::exec::{meta, os, shell, pkg}};
        use TaskKind::*;

        self.done = match self.kind {
            Version => { meta::print_version() },
            Help => { meta::print_help() },
            OsInfo => { os::print_info() },
            User => { os::print_user() },
            Echo => { meta::echo(self) },
            Unrecognized => { meta::unrecognized(self) },
            _ => false, // TODO
        }
    }

    fn new(kind: TaskKind, argument: String, parameters: Vec<String>) -> Task {
        Task {
            kind,
            done: false,
            argument,
            parameters,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskKind {
    Version,
    Help,
    OsInfo,
    Package,
    User,
    Echo,
    Unrecognized,
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
            Order { tasks: vec![Task::new(Version, argument, parameters)] }
        } else if argument == "help" {
            elog("Command is 'help'");
            Order { tasks: vec![Task::new(Help, argument, parameters)] }
        } else if argument == "os" {
            elog("Command is 'os'");
            Order { tasks: vec![Task::new(OsInfo, argument, parameters)] }
        } else if argument == "pkg" {
            elog("Command is 'pkg'");
            Order { tasks: vec![Task::new(Package, argument, parameters)] }
        } else if argument == "user" {
            elog("Command is 'user'");
            Order { tasks: vec![Task::new(User, argument, parameters)] }
        } else if argument == "echo" {
            elog("Command is 'echo'");
            Order { tasks: vec![Task::new(Echo, argument, parameters)] }
        } else {
            Order { tasks: vec![Task::new(Unrecognized, argument, parameters)] }
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
        use crate::run::Task;

        pub fn print_version() -> bool {
            println!("v{}", env!("CARGO_PKG_VERSION"));
            true
        }

        pub fn print_help() -> bool {
            println!("<long help>");
            true
        }

        pub fn echo(task: &Task) -> bool {
            if !task.parameters.is_empty() {
                let text = task.parameters.join(" ");
                println!("{text}");
            }
            true
        }

        pub fn unrecognized(task: &Task) -> bool {
            println!("Unrecognized command: {}\n<short help>", task.argument);
            false
        }
    }

    pub mod os {
    use crate::log::elog;

        pub fn print_info() -> bool {
            use std::process::Command;

            let uname_success = if let Ok(output) = Command::new("uname")
                .arg("--operating-system")
                .output() {
                if let Ok(utf8) = String::from_utf8(output.stdout) {
                    print!("{utf8}");
                    true
                } else {
                    elog("Failed UTF8 coversion of uname output");
                    false
                }
            } else {
                elog("Failed executing or reading output of uname");
                false
            };

            let os_release_success = if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
                elog(&os_release);
                true
            } else {
                elog("Failed reading os-release");
                false
            };

            uname_success && os_release_success
        }

        pub fn print_user() -> bool {
            use std::process::Command;

            if let Ok(output) = Command::new("whoami").output() {
                if let Ok(utf8) = String::from_utf8(output.stdout) {
                    print!("{utf8}");
                    true
                } else {
                    elog("Failed UTF8 coversion of whoami output");
                    false
                }
            } else {
                elog("Failed executing or reading output of whoami");
                false
            }
        }
    }

    pub mod shell {}
    pub mod pkg {}
}
