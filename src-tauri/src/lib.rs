pub mod commands;
pub mod foundations;
pub mod updater;

use foundations::logging;
use tauri::RunEvent;

/// Holds the `flexi_logger` handle for the app's lifetime. Dropping it would
/// stop log flushing — Tauri manages it via `Builder::manage`.
struct LoggerState(#[allow(dead_code)] Option<flexi_logger::LoggerHandle>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
  let debug = std::env::var("TAURI_DEBUG").is_ok() || cfg!(debug_assertions);

  let logger = logging::prepare_logging(debug).ok();
  logging::install_panic_hook(logger.clone());

  log::debug!("Version: {}", *foundations::app::VERSION);
  log::debug!("App dir: {}", foundations::app::app_dir().render());

  let app = tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_process::init())
    .manage(LoggerState(logger))
    // Holds an update downloaded by `download_update` until the app exits, when it is applied
    // (the in-place bundle replace must not happen while the app is running — see updater.rs).
    .manage(updater::PendingUpdate::default())
    .invoke_handler(tauri::generate_handler![
      commands::greet,
      commands::get_app_info,
      commands::get_app_dir,
      commands::get_os,
      commands::resolve_path,
      updater::check_for_updates,
      updater::download_update,
      updater::update_install_support,
      updater::update_staged,
    ])
    .build(tauri::generate_context!())?;

  app.run(|app_handle, event| {
    // `ExitRequested` fires once on the way out, before the process tears down. Apply any update
    // that was downloaded during this session (deferred from `download_update` so the running
    // bundle isn't replaced mid-session — see updater.rs).
    if let RunEvent::ExitRequested { .. } = event {
      updater::install_pending_update_on_exit(app_handle);
    }
  });

  Ok(())
}
