use tori::{conf, log, run, state};

fn main() -> std::process::ExitCode {
    log::elog(&format!("tori {}", env!("CARGO_PKG_VERSION")));
    let configuration = match conf::load() {
        Ok(c) => c,
        Err(error) => {
            eprintln!("Configuration error: {error}");
            return 1.into();
        }
    };
    log::elog(&format!("Configuration: {configuration:#?}"));
    let order = run::teller::parse(std::env::args());
    log::elog(&format!("Order: {order:#?}"));
    let state = state::setup(configuration, &[order]);
    log::elog(&format!("State: {state:#?}"));
    let result = run::expeditor::fulfill(&state);
    log::elog(&format!("Filled Order: {result:#?}"));

    if result.is_ok() { 0.into() } else { 1.into() }
}
