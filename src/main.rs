#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(clippy::manual_string_new)]
#![warn(clippy::map_unwrap_or)]
#![warn(clippy::implicit_clone)]

pub mod app;
pub mod clipboard;
pub mod error;
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
