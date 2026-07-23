use std::sync::LazyLock;

#[derive(
  Clone,
  Copy,
  Debug,
  Default,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
  Hash,
  serde::Serialize,
  serde::Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum Os {
  Windows,
  Linux,
  Mac,
  #[default]
  #[serde(other)]
  Other,
}

impl Os {
  const WINDOWS: bool = cfg!(target_os = "windows");
  const MAC: bool = cfg!(target_os = "macos");
  const LINUX: bool = cfg!(target_os = "linux");

  /// Compile-time host OS — usable in `match` arms.
  pub const HOST: Os = if Self::WINDOWS {
    Self::Windows
  } else if Self::MAC {
    Self::Mac
  } else if Self::LINUX {
    Self::Linux
  } else {
    Self::Other
  };

  pub fn is_case_sensitive(&self) -> bool {
    match self {
      Self::Windows | Self::Mac => false,
      Self::Linux | Self::Other => true,
    }
  }
}

impl From<&str> for Os {
  fn from(value: &str) -> Self {
    match value.to_lowercase().as_str() {
      "windows" => Self::Windows,
      "linux" => Self::Linux,
      "mac" | "macos" => Self::Mac,
      _ => Self::Other,
    }
  }
}

/// Detects whether the app is running on a Steam Deck (Linux + /home/deck exists).
pub static STEAM_DECK: LazyLock<bool> =
  LazyLock::new(|| Os::HOST == Os::Linux && std::path::Path::new("/home/deck").exists());

/// True if running inside a Flatpak sandbox.
pub static FLATPAK: LazyLock<bool> = LazyLock::new(|| {
  std::env::var("FLATPAK_ID").is_ok() || std::path::Path::new("/.flatpak-info").exists()
});

/// True if running inside a Snap package.
pub static SNAP: LazyLock<bool> = LazyLock::new(|| std::env::var("SNAP").is_ok());

/// True if running as an AppImage (set by AppImage runtime).
pub static APPIMAGE: LazyLock<bool> = LazyLock::new(|| std::env::var("APPIMAGE").is_ok());
