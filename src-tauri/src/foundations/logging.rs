//! Rotating file logger + panic hook that flushes the log on crash.
//! See `docs/LUDUSAVI_PATTERNS.md` section 5.

use crate::foundations::app::app_dir;

/// Initialize logging. Returns a `LoggerHandle` that **must** be retained for the
/// lifetime of the app — `flexi_logger` flushes on drop. Stash it in Tauri state
/// via `Builder::manage(_)`.
pub fn prepare_logging(
  debug: bool,
) -> Result<flexi_logger::LoggerHandle, flexi_logger::FlexiLoggerError> {
  let app_dir_path = app_dir().as_std_path_buf().ok();

  let directory_spec = match app_dir_path {
    Some(p) => flexi_logger::FileSpec::default().directory(p),
    None => flexi_logger::FileSpec::default(),
  };

  if debug {
    flexi_logger::Logger::try_with_str("debug")?
      .log_to_file(directory_spec.basename("debug").suppress_timestamp())
      .write_mode(flexi_logger::WriteMode::BufferAndFlush)
      .use_utc()
      .format_for_files(file_format)
      .start()
  } else {
    flexi_logger::Logger::try_with_env_or_str("warn")?
      .log_to_file(directory_spec)
      .write_mode(flexi_logger::WriteMode::BufferAndFlush)
      .rotate(
        flexi_logger::Criterion::Size(1024 * 1024 * 10),
        flexi_logger::Naming::Timestamps,
        flexi_logger::Cleanup::KeepLogFiles(4),
      )
      .use_utc()
      .format_for_files(file_format)
      .start()
  }
}

fn file_format(
  w: &mut dyn std::io::Write,
  now: &mut flexi_logger::DeferredNow,
  record: &log::Record,
) -> std::io::Result<()> {
  write!(
    w,
    "[{}] {} [{}] {}",
    now.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
    record.level(),
    record.module_path().unwrap_or("<unnamed>"),
    record.args(),
  )
}

/// Set a panic hook that captures the backtrace, logs it, and flushes the
/// logger before delegating to the previous hook.
pub fn install_panic_hook(handle: Option<flexi_logger::LoggerHandle>) {
  let original_hook = std::panic::take_hook();
  std::panic::set_hook(Box::new(move |info| {
    let thread_name = std::thread::current()
      .name()
      .unwrap_or("<unnamed thread>")
      .to_owned();

    let location = if let Some(loc) = info.location() {
      format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
    } else {
      "<unknown location>".to_owned()
    };

    let message = info
      .payload()
      .downcast_ref::<&str>()
      .map(|s| s.to_string())
      .or_else(|| info.payload().downcast_ref::<String>().cloned())
      .unwrap_or_default();

    let backtrace = std::backtrace::Backtrace::force_capture();

    log::error!(
      "thread '{thread_name}' panicked at {location}:\n{message}\nstack backtrace:\n{backtrace}"
    );

    if let Some(handle) = handle.clone() {
      handle.flush();
    }

    original_hook(info);
  }));
}
