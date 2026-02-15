use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
  pub name: String,
  pub version: String,
}

#[tauri::command]
pub fn greet(name: String) -> String {
  format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub fn get_app_info() -> Result<AppInfo, String> {
  Ok(AppInfo {
    name: env!("CARGO_PKG_NAME").to_string(),
    version: env!("CARGO_PKG_VERSION").to_string(),
  })
}
