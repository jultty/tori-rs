use crate::run::Task;

pub fn print_version() -> bool {
    println!("v{}", env!("CARGO_PKG_VERSION"));
    true
}

pub fn print_help() -> bool {
    println!("<long help>");
    true
}

pub fn unrecognized(task: &Task) -> bool {
    println!("Unrecognized command: {}\n<short help>", task.argument);
    false
}
