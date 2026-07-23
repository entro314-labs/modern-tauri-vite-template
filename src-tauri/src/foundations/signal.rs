//! SIGINT registration with idempotent register/unregister.
//! See `docs/LUDUSAVI_PATTERNS.md` section 11.
//!
//! Only meaningful on Unix; the Windows variant is a no-op that returns an
//! `Arc<AtomicBool>` that will never flip — callers can treat both identically.

use std::sync::{Arc, atomic::AtomicBool};

#[cfg(unix)]
use std::sync::Mutex;

#[cfg(unix)]
static HANDLER_SIGINT: Mutex<Option<signal_hook::SigId>> = Mutex::new(None);

/// Register a flag that will be set when SIGINT (Ctrl-C) arrives.
/// Subsequent calls replace the previous registration — the latest caller wins.
#[cfg(unix)]
pub fn register_sigint() -> Arc<AtomicBool> {
  let flag = Arc::new(AtomicBool::new(false));

  if let Ok(mut guard) = HANDLER_SIGINT.lock() {
    if let Some(id) = guard.as_ref() {
      signal_hook::low_level::unregister(*id);
      *guard = None;
    }

    if let Ok(id) = signal_hook::flag::register(signal_hook::consts::SIGINT, flag.clone()) {
      *guard = Some(id);
    }
  }

  flag
}

#[cfg(unix)]
pub fn unregister_sigint() {
  if let Ok(mut guard) = HANDLER_SIGINT.lock()
    && let Some(id) = guard.take()
  {
    signal_hook::low_level::unregister(id);
  }
}

#[cfg(not(unix))]
pub fn register_sigint() -> Arc<AtomicBool> {
  Arc::new(AtomicBool::new(false))
}

#[cfg(not(unix))]
pub fn unregister_sigint() {}
