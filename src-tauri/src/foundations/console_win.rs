//! Detach the Windows console after CLI parsing, when the app is being launched
//! as a GUI. See `docs/LUDUSAVI_PATTERNS.md` section 6 for full context on why
//! `#![windows_subsystem = "windows"]` plus `AllocConsole` is the wrong solution.
//!
//! Only relevant when shipping a single binary that handles both CLI and GUI
//! invocations. A pure-GUI Tauri app does not need this.

#[cfg(target_os = "windows")]
pub unsafe fn detach_console() -> Result<(), String> {
  use windows::Win32::{
    Foundation::HANDLE,
    System::Console::{
      FreeConsole, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
    },
  };

  unsafe {
    FreeConsole().map_err(|e| format!("FreeConsole failed: {e}"))?;
    SetStdHandle(STD_INPUT_HANDLE, HANDLE::default()).map_err(|e| format!("reset stdin: {e}"))?;
    SetStdHandle(STD_OUTPUT_HANDLE, HANDLE::default()).map_err(|e| format!("reset stdout: {e}"))?;
    SetStdHandle(STD_ERROR_HANDLE, HANDLE::default()).map_err(|e| format!("reset stderr: {e}"))?;
  }

  Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn detach_console() -> Result<(), String> {
  Ok(())
}
