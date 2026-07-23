//! App identity, version, and config directory resolution.
//! See `docs/LUDUSAVI_PATTERNS.md` section 4.

use std::{
  path::PathBuf,
  sync::{LazyLock, Mutex},
};

use crate::foundations::path::{CommonPath, StrictPath};

/// Override at compile time with `TAURI_APP_NAME` env var (used by build.rs).
/// Defaults to the cargo package name.
pub static APP_DIR_NAME: LazyLock<&'static str> =
  LazyLock::new(|| option_env!("TAURI_APP_NAME").unwrap_or(env!("CARGO_PKG_NAME")));

pub static VERSION: LazyLock<&'static str> =
  LazyLock::new(|| option_env!("TAURI_APP_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")));

pub static USER_AGENT: LazyLock<String> =
  LazyLock::new(|| format!("{}/{}", *APP_DIR_NAME, *VERSION));

pub static CANONICAL_VERSION: LazyLock<(u32, u32, u32)> = LazyLock::new(|| {
  let parts: Vec<u32> = env!("CARGO_PKG_VERSION")
    .split('.')
    .map(|x| x.parse().unwrap_or(0))
    .collect();
  if parts.len() != 3 {
    (0, 0, 0)
  } else {
    (parts[0], parts[1], parts[2])
  }
});

/// CLI / runtime override for the config directory.
pub static CONFIG_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Sentinel file name that, when found next to the executable, switches the app
/// into portable mode (config lives alongside the exe, not in the user dir).
const PORTABLE_FLAG_FILE_NAME_SUFFIX: &str = ".portable";

/// Resolve the app's persistent storage directory with three-tier precedence:
///
/// 1. `CONFIG_DIR` override (set via CLI or programmatically).
/// 2. Portable flag file (`<app-name>.portable`) next to the executable.
/// 3. `<config-dir>/<app-name>` (OS standard).
pub fn app_dir() -> StrictPath {
  if let Some(dir) = CONFIG_DIR.lock().unwrap().as_ref() {
    return StrictPath::from(dir.clone());
  }

  let portable_flag = format!("{}{PORTABLE_FLAG_FILE_NAME_SUFFIX}", *APP_DIR_NAME);
  if let Ok(mut flag) = std::env::current_exe() {
    flag.pop();
    flag.push(&portable_flag);
    if flag.exists() {
      flag.pop();
      return StrictPath::from(flag);
    }
  }

  let base = CommonPath::Config
    .get()
    .map(|x| x.to_string())
    .unwrap_or_else(|| ".".to_string());
  StrictPath::new(format!("{}/{}", base, *APP_DIR_NAME))
}

pub fn is_portable() -> bool {
  let portable_flag = format!("{}{PORTABLE_FLAG_FILE_NAME_SUFFIX}", *APP_DIR_NAME);
  if let Ok(mut flag) = std::env::current_exe() {
    flag.pop();
    flag.push(&portable_flag);
    flag.exists()
  } else {
    false
  }
}

pub fn set_config_dir(dir: PathBuf) {
  *CONFIG_DIR.lock().unwrap() = Some(dir);
}
