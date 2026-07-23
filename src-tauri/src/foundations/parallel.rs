//! Parallelism configuration with three-source precedence.
//! See `docs/LUDUSAVI_PATTERNS.md` section 12.

use std::{num::NonZeroUsize, sync::LazyLock};

pub static AVAILABLE_PARALLELISM: LazyLock<Option<NonZeroUsize>> =
  LazyLock::new(|| std::thread::available_parallelism().ok());

/// Read thread count from an environment variable. Variable name is caller-supplied
/// (e.g. `MY_APP_THREADS`).
pub fn threads_from_env(var: &str) -> Option<NonZeroUsize> {
  match std::env::var(var) {
    Ok(raw) => match raw.parse::<NonZeroUsize>() {
      Ok(n) => {
        log::debug!("Using threads '{raw}' from {var} environment variable");
        Some(n)
      }
      Err(_) => {
        log::warn!("Ignoring invalid threads '{raw}' from {var} environment variable");
        None
      }
    },
    Err(_) => None,
  }
}

/// Initialize the global rayon thread pool. Safe to call once at startup.
#[cfg(feature = "rayon")]
pub fn initialize_rayon(threads: NonZeroUsize) {
  let _ = rayon::ThreadPoolBuilder::new()
    .num_threads(threads.get())
    .build_global();
}
