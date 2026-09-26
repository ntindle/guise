//! Filesystem + resource paths for Claude Desktop and the guise account store.
//!
//! guise's model: each saved account is a **permanent, independent userData
//! directory** that Claude Desktop is launched against via `--user-data-dir`.
//! Accounts never share a directory, so logging into one never logs out
//! another — no snapshotting, no token rotation races, no revocation.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

/// Default macOS application bundle location.
#[cfg(unix)]
pub const DEFAULT_APP_PATH: &str = "/Applications/Claude.app";

/// Effective default app location on this platform: the bundle const on
/// macOS, the resolved install dir on Windows.
pub fn default_app_path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from(DEFAULT_APP_PATH)
    }
    #[cfg(windows)]
    {
        resolve_app_dir()
    }
}

/// Human label for the app install used in messages.
#[cfg(unix)]
pub const APP_LABEL: &str = "Claude.app";
/// Human label for the app install used in messages.
#[cfg(windows)]
pub const APP_LABEL: &str = "Claude";

/// Resolved, machine-specific locations guise operates on.
#[derive(Debug, Clone)]
pub struct Paths {
    /// `~` — the user's home directory. Retained for path construction/tests.
    #[allow(dead_code)]
    pub home: PathBuf,
    /// `Claude.app` bundle.
    pub app: PathBuf,
    /// Root of the account store (`~/.config/guise` or legacy `~/.guise`).
    pub guise_root: PathBuf,
}

impl Paths {
    /// Resolve all paths from the environment.
    ///
    /// If data exists at the legacy `~/.guise` location, it is automatically
    /// migrated to `~/.config/guise` (XDG-compliant). The lookup order is:
    ///   1. `$XDG_CONFIG_HOME/guise` (if `$XDG_CONFIG_HOME` is set)
    ///   2. `~/.config/guise`
    ///   3. `~/.guise` (legacy, triggers migration)
    pub fn resolve() -> Result<Self> {
        let home = home_dir()?;
        let guise_root = resolve_guise_root(&home)?;
        #[cfg(unix)]
        let app = PathBuf::from(DEFAULT_APP_PATH);
        #[cfg(windows)]
        let app = resolve_app_dir();
        Ok(Paths {
            home: home.clone(),
            app,
            guise_root,
        })
    }

    /// The Claude Desktop executable inside the install location.
    pub fn app_binary(&self) -> PathBuf {
        #[cfg(unix)]
        {
            self.app.join("Contents").join("MacOS").join("Claude")
        }
        #[cfg(windows)]
        {
            app_binary_for(&self.app)
        }
    }

    /// Directory holding one subdirectory per saved account.
    pub fn accounts_dir(&self) -> PathBuf {
        self.guise_root.join("accounts")
    }

    /// guise's own settings file.
    pub fn tool_config_json(&self) -> PathBuf {
        self.guise_root.join("config.json")
    }

    /// Claude's standard `claude-code-sessions` location (also meld's default
    /// `sessions_root`). Used as the shared target so `meld` can merge chats.
    pub fn default_code_sessions_root(&self) -> PathBuf {
        #[cfg(unix)]
        {
            self.home
                .join("Library")
                .join("Application Support")
                .join("Claude")
                .join("claude-code-sessions")
        }
        #[cfg(windows)]
        {
            // Two Windows flavors exist: the direct install under Roaming
            // (meld's Windows default — preferred so both tools agree with
            // zero configuration) and the Store build under Local `Claude-3p`.
            let roaming = app_data().join("Claude").join("claude-code-sessions");
            let local = local_app_data()
                .join("Claude-3p")
                .join("claude-code-sessions");
            if roaming.exists() {
                roaming
            } else if local.exists() {
                local
            } else {
                roaming
            }
        }
    }

    /// meld's config file, if the user has meld installed.
    pub fn meld_config(&self) -> PathBuf {
        self.home.join(".meld").join("config.toml")
    }
}

/// Resolve the home directory from `$HOME` (`%USERPROFILE%` on Windows).
pub fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| {
            anyhow!("neither $HOME nor %USERPROFILE% is set; cannot locate the home directory")
        })
}

/// `%APPDATA%`, with a home-relative fallback.
#[cfg(windows)]
pub fn app_data() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| {
            home_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("AppData")
                .join("Roaming")
        })
}

/// `%LOCALAPPDATA%`, with a home-relative fallback.
#[cfg(windows)]
pub fn local_app_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| {
            home_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("AppData")
                .join("Local")
        })
}

/// Default Claude install dir on Windows: the direct install when its exe is
/// present, else the folder holding the Store `claude-desktop` execution
/// alias, else the direct dir (so `doctor` reports a useful missing path).
#[cfg(windows)]
pub fn resolve_app_dir() -> PathBuf {
    let direct = local_app_data().join("Programs").join("Claude");
    if direct.join("Claude.exe").exists() {
        return direct;
    }
    if let Some(alias) = find_on_path("claude-desktop.exe") {
        if let Some(parent) = alias.parent() {
            return parent.to_path_buf();
        }
    }
    direct
}

/// First `file` found on `%PATH%`, if any.
#[cfg(windows)]
fn find_on_path(file: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(file);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// The Claude executable inside an install dir: direct `Claude.exe`, the
/// Store `claude-desktop` alias, or the nested Store package layout.
#[cfg(windows)]
pub fn app_binary_for(app_dir: &Path) -> PathBuf {
    for name in ["Claude.exe", "claude-desktop.exe"] {
        let candidate = app_dir.join(name);
        if candidate.exists() {
            return candidate;
        }
    }
    let nested = app_dir.join("app").join("claude.exe");
    if nested.exists() {
        return nested;
    }
    app_dir.join("Claude.exe")
}

/// Whether a path exists (file, dir, or symlink).
pub fn exists(p: &Path) -> bool {
    p.symlink_metadata().is_ok()
}

/// Validate that a directory exists, returning a helpful error otherwise.
pub fn require_dir(p: &Path, what: &str) -> Result<()> {
    let md =
        std::fs::metadata(p).with_context(|| format!("{what} not found at {}", p.display()))?;
    if !md.is_dir() {
        return Err(anyhow!("{what} at {} is not a directory", p.display()));
    }
    Ok(())
}

/// Determine the guise root directory, migrating from `~/.guise` to
/// `~/.config/guise` if the legacy location exists and the new one doesn't.
fn resolve_guise_root(home: &Path) -> Result<PathBuf> {
    let xdg_root = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(val) if !val.is_empty() => PathBuf::from(val).join("guise"),
        _ => home.join(".config").join("guise"),
    };
    let legacy_root = home.join(".guise");

    if xdg_root.exists() {
        return Ok(xdg_root);
    }

    if legacy_root.exists() {
        migrate_legacy_to_xdg(&legacy_root, &xdg_root)?;
        return Ok(xdg_root);
    }

    // Neither exists yet — use the XDG path (created on first `guise add`).
    Ok(xdg_root)
}

/// Move the legacy `~/.guise` tree to the XDG location atomically:
/// rename the directory, then leave a symlink at the old path so any
/// external scripts that still reference `~/.guise` keep working.
fn migrate_legacy_to_xdg(legacy: &Path, xdg: &Path) -> Result<()> {
    if let Some(parent) = xdg.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::rename(legacy, xdg)
        .with_context(|| format!("migrating {} -> {}", legacy.display(), xdg.display()))?;

    // Best-effort compatibility symlink: ~/.guise -> ~/.config/guise
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(xdg, legacy);
    }

    eprintln!(
        "guise: migrated data from {} -> {}",
        legacy.display(),
        xdg.display()
    );
    Ok(())
}
