//! Claude Desktop process control for the per-account instance model.
//!
//! Each account runs as its own Claude process launched with
//! `--user-data-dir=<account data dir>`. The [`AppControl`] trait is
//! implemented per platform (`app_macos`, `app_windows`) so call sites stay
//! portable. Behind a trait (rather than free functions) so the logic stays
//! testable with a fake.

use anyhow::Result;
use std::path::Path;
use std::time::Duration;

/// Control over Claude Desktop instances. Testable via a fake.
pub trait AppControl {
    /// Launch a Claude instance bound to `data_dir`. `app_bundle` is the
    /// install location (`.app` bundle on macOS, install dir on Windows).
    /// Fully detached from the caller's terminal.
    fn launch_instance(&self, app_bundle: &Path, data_dir: &Path) -> Result<()>;
    /// Is a Claude instance for exactly this `data_dir` currently running?
    fn is_instance_running(&self, data_dir: &Path) -> Result<bool>;
    /// Bring the instance bound to `data_dir` to the foreground. Every
    /// Claude window shares one title, so this must target the account's
    /// own processes — never match by window title.
    fn activate(&self, app_bundle: &Path, data_dir: &Path) -> Result<()>;
    /// Quit the Claude instance bound to `data_dir` (gracefully, then firmly).
    fn quit_instance(&self, data_dir: &Path, timeout: Duration) -> Result<()>;
}

/// Construct the production app controller for this platform.
#[cfg(unix)]
pub fn control() -> crate::app_macos::RealApp {
    crate::app_macos::RealApp
}

/// Construct the production app controller for this platform.
#[cfg(windows)]
pub fn control() -> crate::app_windows::WinApp {
    crate::app_windows::WinApp
}
