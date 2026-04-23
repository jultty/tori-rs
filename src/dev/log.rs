#[track_caller]
pub fn elog(message: &str) {
    if let Ok(debug) = std::env::var("DEBUG")
        && !debug.is_empty()
    {
        let location = std::panic::Location::caller();
        eprintln!(" !debug [{location}] {message}");
    }
}
