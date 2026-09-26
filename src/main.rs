//! guise — Claude Desktop account switcher.
//!
//! Keeps each Claude Desktop account in its own permanent profile directory
//! (launched via `--user-data-dir`) so multiple accounts stay logged in at once
//! and you switch by opening the one you want. Local-only; guise never reads,
//! copies, or decrypts session data — Claude owns each profile directory.

mod account;
mod app;
#[cfg(unix)]
mod app_macos;
#[cfg(windows)]
mod app_windows;
mod cli;
mod config;
mod doctor;
mod paths;
#[cfg(windows)]
mod proc;

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if let Err(e) = cli::dispatch(argv) {
        eprintln!("guise: {e:#}");
        std::process::exit(1);
    }
}
