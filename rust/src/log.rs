pub fn elog(message: &str) {
    // DONE MUST be printed only if DEBUG is set in the environment
    if let Ok(debug) = std::env::var("DEBUG") && !debug.is_empty() {
        eprintln!(" [log] {message}");
    }
}
