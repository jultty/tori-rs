use tori::{conf, log, run};

fn main() {
    log::elog(&format!("tori {}", env!("CARGO_PKG_VERSION")));
    let configuration = conf::load();
    log::elog(&format!("Configuration: {configuration:#?}"));
    let mut order = run::teller::parse(std::env::args());
    log::elog(&format!("Order: {order:#?}"));
    order.fill();
    log::elog(&format!("Filled Order: {order:#?}"));
}
