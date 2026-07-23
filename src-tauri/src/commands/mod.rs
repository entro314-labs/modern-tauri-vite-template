use serde::Serialize;

use crate::foundations::{app, os::Os, path::StrictPath};

#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
  pub name: String,
  pub version: String,
  pub portable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OsInfo {
  pub host: Os,
  pub flatpak: bool,
  pub snap: bool,
  pub appimage: bool,
  pub steam_deck: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedPath {
  /// The original input string.
  pub raw: String,
  /// Display-friendly form (placeholders resolved, forward slashes).
  pub render: String,
  /// Glob-escaped form, safe for `globset` patterns.
  pub globbable: String,
  /// OS-native canonicalized form, if the path is accessible.
  pub interpret: Option<String>,
  pub exists: bool,
}

#[tauri::command]
pub fn greet(name: String) -> String {
  format!("Hello, {name}! You've been greeted from Rust!")
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
  AppInfo {
    name: env!("CARGO_PKG_NAME").to_string(),
    version: env!("CARGO_PKG_VERSION").to_string(),
    portable: app::is_portable(),
  }
}

/// Returns the resolved persistent storage directory for the app, taking into
/// account portable-mode detection and any CLI override.
#[tauri::command]
pub fn get_app_dir() -> String {
  app::app_dir().render()
}

#[tauri::command]
pub fn get_os() -> OsInfo {
  use crate::foundations::os::{APPIMAGE, FLATPAK, SNAP, STEAM_DECK};
  OsInfo {
    host: Os::HOST,
    flatpak: *FLATPAK,
    snap: *SNAP,
    appimage: *APPIMAGE,
    steam_deck: *STEAM_DECK,
  }
}

/// Resolve a path expression with placeholders (`<home>`, `<xdgConfig>`,
/// `<winAppData>`, `<osUserName>`, etc.) and return all useful forms in one call.
#[tauri::command]
pub fn resolve_path(input: String) -> ResolvedPath {
  let path = StrictPath::new(input.clone());
  ResolvedPath {
    raw: input,
    render: path.render(),
    globbable: path.globbable(),
    interpret: path.interpret().ok(),
    exists: path.exists(),
  }
}
