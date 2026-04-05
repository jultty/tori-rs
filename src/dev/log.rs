pub fn elog(message: &str) {
    if let Ok(debug) = std::env::var("DEBUG")
        && !debug.is_empty()
    {
        eprintln!(" [log] {message}");
    }
}
