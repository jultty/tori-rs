use crate::{
    dev::log::elog,
    run::{Order, Task, TaskKind},
};
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

    let make_order = |kind: TaskKind| -> Order {
        Order {
            tasks: vec![Task::new(kind, &argument, parameters)],
        }
    };

    elog(&format!("Command is {argument}"));

    match argument.as_str() {
        "version" | "-v" | "--version" => make_order(TaskKind::Version),
        "help" | "-h" | "--help" => make_order(TaskKind::Help),
        "install" => make_order(TaskKind::PackageInstall),
        "uninstall" => make_order(TaskKind::PackageUninstall),
        "auto" => make_order(TaskKind::PackageListAuto),
        "manual" => make_order(TaskKind::PackageListManual),
        _ => make_order(TaskKind::Unrecognized),
    }
}

fn is_executable_path(candidate: &str) -> bool {
    fn assume(message: &str) -> bool {
        elog(&format!("Assuming args[0] is the executable {message}"));
        true
    }

    let Ok(executable_path) = env::current_exe() else {
        return assume("Failed to get executable path");
    };
    let Some(executable_file) = executable_path.file_name() else {
        return assume("Executable path lacks a file component");
    };

    let argument_path = PathBuf::from(candidate);
    let Some(argument_file) = argument_path.file_name() else {
        return assume("Argument path lacks a file component");
    };

    elog(&format!(
        "Executable path: {executable_path:?}, file {executable_file:?} \
        Argument path: {argument_path:?}, file {argument_file:?} "
    ));

    if argument_path.exists() {
        if let Ok(argument_canonical) = argument_path.canonicalize()
            && let Ok(executable_canonical) = executable_path.canonicalize()
        {
            let judgment = argument_canonical == executable_canonical;
            elog(&format!(
                "args[0] canonically is executable path: {judgment}"
            ));
            judgment
        } else {
            assume("Could not canonicalize executable and argument paths")
        }
    } else {
        let judgment = argument_file == executable_file;
        elog(&format!(
            "args[0] matches executable path by name only: {judgment}"
        ));
        judgment
    }
}
