pub mod app;
pub mod clipboard;
pub mod error;
pub mod events;
pub mod ui;

use cursive::logger::reserve_logs;
use cursive::logger::CursiveLogger;
use cursive::reexports::log;

fn main() {
    logging();

    app::start();
}

/// Initiate Logging
fn logging() {
    reserve_logs(1_000);
    log::set_logger(&CursiveLogger).unwrap();
    log::set_max_level(log::LevelFilter::Warn);
}
