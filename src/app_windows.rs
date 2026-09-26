//! Windows backend for [`crate::app::AppControl`].
//!
//! Instances launch via the resolved `Claude.exe` (direct install) or the
//! `claude-desktop` Store execution alias, detached from the caller's console.
//! Detection matches the `--user-data-dir=<dir>` token in each `claude.exe`
//! command line (see [`crate::proc`]); quitting is `taskkill` without `/F`
//! (graceful close request), escalating to `/F` after the timeout.

use crate::app::AppControl;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

const DETACHED_PROCESS: u32 = 0x0800_0000;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// Production implementation talking to the real OS.
pub struct WinApp;

impl AppControl for WinApp {
    fn launch_instance(&self, app_bundle: &Path, data_dir: &Path) -> Result<()> {
        use std::os::windows::process::CommandExt;
        let exe = crate::paths::app_binary_for(app_bundle);
        Command::new(&exe)
            .arg(format!("--user-data-dir={}", data_dir.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .with_context(|| format!("launching {}", exe.display()))?;
        Ok(())
    }

    fn is_instance_running(&self, data_dir: &Path) -> Result<bool> {
        Ok(!crate::proc::pids_for_data_dir(data_dir)?.is_empty())
    }

    fn activate(&self, _app_bundle: &Path) -> Result<()> {
        // Bring *a* Claude window forward (they share one title, so a
        // specific account can't be targeted). Best-effort.
        let _ = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(New-Object -ComObject WScript.Shell).AppActivate('Claude')",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        Ok(())
    }

    fn quit_instance(&self, data_dir: &Path, timeout: Duration) -> Result<()> {
        for pid in crate::proc::pids_for_data_dir(data_dir)? {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string()])
                .output();
        }
        let deadline = Instant::now() + timeout;
        loop {
            let live = crate::proc::pids_for_data_dir(data_dir)?;
            if live.is_empty() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                for pid in live {
                    let _ = Command::new("taskkill")
                        .args(["/F", "/PID", &pid.to_string()])
                        .output();
                }
                return Ok(());
            }
            sleep(Duration::from_millis(200));
        }
    }
}
