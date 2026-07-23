//! Cross-platform foundation modules ported from Ludusavi.
//!
//! Full design notes in `docs/LUDUSAVI_PATTERNS.md`. Each submodule references
//! its corresponding section.

pub mod app;
pub mod command;
pub mod console_win;
pub mod error;
pub mod http;
pub mod logging;
pub mod os;
pub mod parallel;
pub mod path;
pub mod resource;
pub mod signal;

pub use app::{APP_DIR_NAME, USER_AGENT, VERSION, app_dir, is_portable, set_config_dir};
pub use command::{CommandError, CommandOutput, Privacy, run_command};
pub use error::AnyError;
pub use http::{Security, reqwest_blocking_client, reqwest_client};
pub use os::{APPIMAGE, FLATPAK, Os, SNAP, STEAM_DECK};
pub use path::{CommonPath, StrictPath, StrictPathError};
pub use resource::{ResourceFile, SaveableResourceFile};
