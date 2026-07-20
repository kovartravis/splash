use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::time::Duration;
use std::sync::mpsc;
fn test() {
    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(100), tx).unwrap();
    debouncer.watcher().watch(std::path::Path::new("."), RecursiveMode::Recursive).unwrap();
    let _ = debouncer;
}
