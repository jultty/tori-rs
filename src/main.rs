use tori::{conf, dev::log::elog, run, state};

fn main() -> std::process::ExitCode {
    elog(&format!("tori {}", env!("CARGO_PKG_VERSION")));
    let configuration = match conf::load() {
        Ok(c) => c,
        Err(error) => {
            eprintln!("Configuration error: {error}");
            return 1.into();
        }
    };
    elog(&format!("Configuration: {configuration:#?}"));
    let order = run::teller::parse(std::env::args());
    elog(&format!("Order: {order:#?}"));
    let state = state::setup(configuration, &[order]);
    elog(&format!("State: {state:#?}"));
    let result = run::expeditor::expedite(&state);
    elog(&format!("Filled Order: {result:#?}"));

    if result.is_ok() { 0.into() } else { 1.into() }
}
