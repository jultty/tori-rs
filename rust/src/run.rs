// B2.1. DONE version | -v | --version -> MUST print the version as in v0.8.0
// B2.2. DONE help | -h | --help -> MUST print '<long help>'
// B2.3. DONE os -> MUST print the OS name and MUST log contents of /etc/os-release
// B2.4. DONE user -> MUST print the output of the 'whoami' command
// B2.6. DONE echo x y z -> MUST print x y z
// B2.7. DONE echo -> MUST NOT print any output and exit with status code 0
// B2.8. DONE [no input] -> MUST NOT print any output and exit with status code 0
// B2.9. DONE [any other input] -> MUST print 'Unrecognized command: [command]',
//       a newline, '<short help>' and exit with status code 1

use crate::conf::Configuration;

#[derive(Default, Debug)]
pub struct Order<'o> {
    tasks: Vec<Task<'o>>,
}

impl Order<'_> {

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
pub struct Task<'t> {
    kind: TaskKind,
    done: bool,
    argument: String,
    parameters: Vec<String>,
    configuration: &'t Configuration,
}

impl Task<'_> {
    pub fn complete(&mut self) {
        use crate::{run::exec::{meta, os, pkg}};
        use TaskKind::*;

        self.done = match self.kind {
            Version => { meta::print_version() },
            Help => { meta::print_help() },
            OsInfo => { os::print_info() },
            User => { os::print_user() },
            Echo => { meta::echo(self) },
            Unrecognized => { meta::unrecognized(self) },
            Package => { pkg::install_uninstall(self) },
        }
    }

    fn new<'t>(
        kind: TaskKind,
        argument: &str,
        parameters: Vec<String>,
        configuration: &'t Configuration,
    ) -> Task<'t> {
        Task {
            kind,
            done: false,
            argument: String::from(argument),
            parameters,
            configuration,
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

#[derive(Debug, Clone)]
pub struct Command {
    pub base: String,
    pub args: Vec<String>,
}

pub mod teller {
    use crate::{conf::Configuration, log::elog, run::{Order, Task, TaskKind}};
    use std::{env, path::PathBuf};

    pub fn parse(mut raw_args: env::Args, configuration: &Configuration) -> Order<'_> {
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

        let make_order = |kind: TaskKind| -> Order {
            Order { tasks: vec![Task::new(kind, &argument, parameters, configuration)] }
        };

        if argument == "version" || argument == "-v" || argument == "--version" {
            elog("Command is 'version'");
            make_order(TaskKind::Version)
        } else if argument == "help" || argument == "-h" || argument == "--help" {
            elog("Command is 'help'");
            make_order(TaskKind::Help)
        } else if argument == "os" {
            elog("Command is 'os'");
            make_order(TaskKind::OsInfo)
        } else if argument == "pkg" {
            elog("Command is 'pkg'");
            make_order(TaskKind::Package)
        } else if argument == "user" {
            elog("Command is 'user'");
            make_order(TaskKind::User)
        } else if argument == "echo" {
            elog("Command is 'echo'");
            make_order(TaskKind::Echo)
        } else {
            make_order(TaskKind::Unrecognized)
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
    use crate::{ log::elog,run::exec::shell::run};

        pub fn print_info() -> bool {

            let uname_result = run("uname", &["--operating-system"]);

            let os_release_result = if let Ok(os_release) =
            std::fs::read_to_string("/etc/os-release") {
                elog(&os_release);
                true
            } else {
                elog("Failed reading os-release");
                false
            };

            uname_result.is_ok() && os_release_result
        }

        pub fn print_user() -> bool {
            run("whoami", &[]).is_ok()
        }
    }

    pub mod pkg {
        use crate::run::Task;

        // B2.5. DONE pkg p -> MUST call the system package manager using the su_command
        //       to install and then uninstall package p. The user MUST be able to
        //       freely input to these commands' interactive inputs before control
        //       is returned. When done, it MUST log 'Done:', a newline, and the
        //       system commands executed, one per line. If no p is provided, it
        //       MUST NOT run any system commands and print a message
        pub fn install_uninstall(task: &Task) -> bool {
            let su_base: String = task.configuration.su_command.base.clone();
            let su_args: Vec<String> = task.configuration.su_command.args.clone();
            let command_base: Vec<String> = vec!["apt".into(), "install".into()];
            let command_args: Vec<String> = task.parameters.clone();

            let su_args_str: Vec<&str> = su_args.iter().map(|s| s.as_str()).collect();
            let command_base_str: Vec<&str> = command_base.iter().map(|s| s.as_str()).collect();
            let command_args_str: Vec<&str> = command_args.iter().map(|s| s.as_str()).collect();

            if command_args.is_empty() {
                println!("Parameters are empty: Nothing to install or uninstall");
                return false
            }

            let args: Vec<&str> = [
                su_args_str,
                command_base_str,
                command_args_str,
            ].iter().flatten().copied().collect();

            crate::run::exec::shell::spawn(&su_base, &args);

            crate::run::exec::shell::spawn("sudo", vec!["apt", "remove"]
                .into_iter()
                .chain(task.parameters.iter().map(|s| s.as_str()))
                .collect::<Vec<&str>>().as_slice());

            println!(
                "Done:\n{su_base} {} {} {}",
                su_args.join(" "),
                command_base.join(" "),
                command_args.join(" "),
            );
            true
        }
    }

    pub mod shell {
        use std::process::Command;

        pub fn spawn(command: &str, args: &[&str]) -> bool {
            if let Ok(mut child) = Command::new(command)
                .args(args)
                .spawn() {
                let Ok(exit_status) = child.wait() else { return false };
                exit_status.success()
            } else {
                false
            }
        }

        pub fn run(command: &str, args: &[&str]) -> Result<String, String> {
            use std::process::Command;

            if let Ok(output) = Command::new(command)
                .args(args)
                .output()

            {
                if let Ok(utf8) = String::from_utf8(output.stdout) {
                    print!("{utf8}");
                    Ok(utf8)
                } else {
                    let message = format!("Failed UTF8 coversion of {command} output");
                    eprintln!("{message}");
                    Err(message)
                }
            } else {
                let message = format!("Failed executing or reading output of {command}");
                eprintln!("{message}");
                Err(message)

            }
        }

    }


}
