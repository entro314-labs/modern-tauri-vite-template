//! Cross-platform path abstraction lifted from Ludusavi (`src/path.rs`).
//! See `docs/LUDUSAVI_PATTERNS.md` section 1 + 2.

use std::sync::{Arc, LazyLock, Mutex};

use itertools::Itertools;

use crate::foundations::os::Os;

pub mod placeholder {
  pub const HOME: &str = "<home>";
  pub const OS_USER_NAME: &str = "<osUserName>";
  pub const XDG_CONFIG: &str = "<xdgConfig>";
  pub const XDG_DATA: &str = "<xdgData>";
  pub const WIN_APP_DATA: &str = "<winAppData>";
  pub const WIN_LOCAL_APP_DATA: &str = "<winLocalAppData>";
  pub const WIN_LOCAL_APP_DATA_LOW: &str = "<winLocalAppDataLow>";
  pub const WIN_DOCUMENTS: &str = "<winDocuments>";
  pub const WIN_PUBLIC: &str = "<winPublic>";
  pub const WIN_PROGRAM_DATA: &str = "<winProgramData>";
  pub const WIN_DIR: &str = "<winDir>";
  pub const SAVED_GAMES: &str = "<savedGames>";
}

pub static OS_USERNAME: LazyLock<String> = LazyLock::new(whoami::username);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StrictPathError {
  Empty,
  Relative,
  Unmappable,
  Unsupported,
}

impl std::fmt::Display for StrictPathError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Empty => write!(f, "path is empty"),
      Self::Relative => write!(f, "path is relative with no basis"),
      Self::Unmappable => write!(f, "path could not be mapped"),
      Self::Unsupported => write!(f, "path form is unsupported on this OS"),
    }
  }
}

impl std::error::Error for StrictPathError {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Drive {
  Root,
  Windows(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Canonical {
  Valid(String),
  Unsupported,
  Inaccessible,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Analysis {
  drive: Option<Drive>,
  parts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnalysisMode {
  Normal,
  Glob,
}

/// Standard cross-platform directories.
///
/// All variants are `LazyLock`-cached. Returns `None` if the OS does not
/// expose the requested directory.
pub enum CommonPath {
  Config,
  Data,
  DataLocal,
  DataLocalLow,
  Document,
  Home,
  Public,
  SavedGames,
}

impl CommonPath {
  pub fn get(&self) -> Option<&str> {
    fn check_dir(path: Option<std::path::PathBuf>) -> Option<String> {
      Some(path?.to_string_lossy().to_string())
    }

    static CONFIG: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::config_dir()));
    static DATA: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::data_dir()));
    static DATA_LOCAL: LazyLock<Option<String>> =
      LazyLock::new(|| check_dir(dirs::data_local_dir()));
    static DOCUMENT: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::document_dir()));
    static HOME: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::home_dir()));
    static PUBLIC: LazyLock<Option<String>> = LazyLock::new(|| check_dir(dirs::public_dir()));

    #[cfg(windows)]
    static DATA_LOCAL_LOW: LazyLock<Option<String>> = LazyLock::new(|| {
      known_folders::get_known_folder_path(known_folders::KnownFolder::LocalAppDataLow).map(|x| {
        x.to_string_lossy()
          .trim_end_matches(['/', '\\'])
          .to_string()
      })
    });
    #[cfg(not(windows))]
    static DATA_LOCAL_LOW: Option<String> = None;

    #[cfg(windows)]
    static SAVED_GAMES: LazyLock<Option<String>> = LazyLock::new(|| {
      known_folders::get_known_folder_path(known_folders::KnownFolder::SavedGames).map(|x| {
        x.to_string_lossy()
          .trim_end_matches(['/', '\\'])
          .to_string()
      })
    });
    #[cfg(not(windows))]
    static SAVED_GAMES: Option<String> = None;

    match self {
      Self::Config => CONFIG.as_ref(),
      Self::Data => DATA.as_ref(),
      Self::DataLocal => DATA_LOCAL.as_ref(),
      Self::DataLocalLow => DATA_LOCAL_LOW.as_ref(),
      Self::Document => DOCUMENT.as_ref(),
      Self::Home => HOME.as_ref(),
      Self::Public => PUBLIC.as_ref(),
      Self::SavedGames => SAVED_GAMES.as_ref(),
    }
    .map(|x| x.as_str())
  }

  pub fn get_globbable(&self) -> Option<String> {
    self.get().map(|x| StrictPath::new(x).globbable())
  }
}

/// A path string with explicit semantics for `raw` vs `interpret` vs `render` vs `globbable`.
///
/// See `docs/LUDUSAVI_PATTERNS.md` section 1.
#[derive(Clone, Default)]
pub struct StrictPath {
  raw: String,
  basis: Option<String>,
  canonical: Arc<Mutex<Option<Canonical>>>,
}

impl Eq for StrictPath {}

impl PartialEq for StrictPath {
  fn eq(&self, other: &Self) -> bool {
    self.raw == other.raw && self.basis == other.basis
  }
}

impl Ord for StrictPath {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    let raw = self.raw.cmp(&other.raw);
    if raw != std::cmp::Ordering::Equal {
      raw
    } else {
      self.basis.cmp(&other.basis)
    }
  }
}

impl PartialOrd for StrictPath {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl std::hash::Hash for StrictPath {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.raw.hash(state);
    self.basis.hash(state);
  }
}

impl std::fmt::Debug for StrictPath {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "StrictPath {{ raw: {:?}, basis: {:?} }}",
      self.raw, self.basis
    )
  }
}

impl From<std::path::PathBuf> for StrictPath {
  fn from(value: std::path::PathBuf) -> Self {
    Self::new(value.display().to_string())
  }
}

impl From<&std::path::Path> for StrictPath {
  fn from(value: &std::path::Path) -> Self {
    Self::new(value.display().to_string())
  }
}

impl From<Analysis> for StrictPath {
  fn from(analysis: Analysis) -> Self {
    let raw = match analysis {
      Analysis {
        drive: Some(Drive::Root),
        parts,
      } => format!("/{}", parts.join("/")),
      Analysis {
        drive: Some(Drive::Windows(id)),
        parts,
      } => format!("{}/{}", id, parts.join("/")),
      Analysis { drive: None, parts } => parts.join("/"),
    };
    Self::new(raw)
  }
}

impl StrictPath {
  pub fn new(raw: impl Into<String>) -> Self {
    Self {
      raw: raw.into(),
      basis: None,
      canonical: Arc::new(Mutex::new(None)),
    }
  }

  pub fn relative(raw: impl Into<String>, basis: Option<impl Into<String>>) -> Self {
    Self {
      raw: raw.into(),
      basis: basis.map(|x| x.into()),
      canonical: Arc::new(Mutex::new(None)),
    }
  }

  pub fn rebase(&mut self, basis: &StrictPath) {
    self.basis = Some(basis.render());
    self.invalidate_cache();
  }

  pub fn cwd() -> Self {
    Self::from(std::env::current_dir().unwrap_or_default())
  }

  pub fn raw(&self) -> &str {
    &self.raw
  }

  /// Clear the cached canonical form. Use when underlying file system may have changed.
  pub fn invalidate_cache(&self) {
    let mut cached = self.canonical.lock().unwrap();
    *cached = None;
  }

  pub fn as_std_path_buf(&self) -> Result<std::path::PathBuf, std::io::Error> {
    Ok(std::path::PathBuf::from(self.interpret().map_err(
      |_| std::io::Error::other(format!("Cannot interpret path: {self:?}")),
    )?))
  }

  fn analyze(&self) -> Analysis {
    self.analyze_with_mode(AnalysisMode::Normal)
  }

  fn analyze_with_mode(&self, mode: AnalysisMode) -> Analysis {
    use typed_path::{
      Utf8TypedComponent as Component, Utf8TypedPath as TypedPath, Utf8UnixComponent as UComponent,
      Utf8WindowsComponent as WComponent, Utf8WindowsPrefix as WindowsPrefix,
    };

    let mut analysis = Analysis::default();

    macro_rules! correct_windows_slashes {
      ($start:expr, $server:expr, $share:expr) => {{
        let mut share_parts: Vec<_> = $share.split('/').collect();
        while share_parts.len() > 1 {
          analysis.parts.push(share_parts.remove(1).to_string());
        }

        let share = share_parts.pop().unwrap_or($share);
        match $server {
          Some(server) => format!(r"{}{}\{}", $start, server, share),
          None => format!(r"{}{}", $start, share),
        }
      }};
    }

    for (i, component) in TypedPath::derive(self.raw.trim()).components().enumerate() {
      match component {
        Component::Windows(WComponent::Prefix(prefix)) => {
          let mapped = match prefix.kind() {
            WindowsPrefix::Verbatim(id) => match mode {
              AnalysisMode::Normal => correct_windows_slashes!(r"\\?\", None::<&str>, id),
              AnalysisMode::Glob => correct_windows_slashes!(r"\\", None::<&str>, id),
            },
            WindowsPrefix::VerbatimUNC(server, share) => match mode {
              AnalysisMode::Normal => correct_windows_slashes!(r"\\?\UNC\", Some(server), share),
              AnalysisMode::Glob => correct_windows_slashes!(r"\\", Some(server), share),
            },
            WindowsPrefix::VerbatimDisk(id) => format!("{}:", id.to_ascii_uppercase()),
            WindowsPrefix::DeviceNS(id) => format!(r"\\.\{id}"),
            WindowsPrefix::UNC(server, share) => {
              correct_windows_slashes!(r"\\", Some(server), share)
            }
            WindowsPrefix::Disk(id) => format!("{}:", id.to_ascii_uppercase()),
          };
          analysis.drive = Some(Drive::Windows(mapped));
        }
        Component::Unix(UComponent::RootDir) | Component::Windows(WComponent::RootDir) => {
          if i == 0 {
            analysis.drive = Some(Drive::Root);
          }
        }
        Component::Unix(UComponent::CurDir) | Component::Windows(WComponent::CurDir) => {
          if i == 0
            && let Some(basis) = &self.basis
          {
            analysis = Self::new(basis.clone()).analyze_with_mode(mode);
          }
        }
        Component::Unix(UComponent::ParentDir) | Component::Windows(WComponent::ParentDir) => {
          if i == 0
            && let Some(basis) = &self.basis
          {
            analysis = Self::new(basis.clone()).analyze_with_mode(mode);
          }
          analysis.parts.pop();
        }
        Component::Unix(UComponent::Normal(part))
        | Component::Windows(WComponent::Normal(part)) => {
          if i == 0 {
            let mapped = match part {
              "~" | placeholder::HOME => CommonPath::Home.get(),
              placeholder::XDG_CONFIG => CommonPath::Config.get(),
              placeholder::XDG_DATA | placeholder::WIN_APP_DATA => CommonPath::Data.get(),
              placeholder::WIN_LOCAL_APP_DATA => CommonPath::DataLocal.get(),
              placeholder::WIN_LOCAL_APP_DATA_LOW => CommonPath::DataLocalLow.get(),
              placeholder::WIN_DOCUMENTS => CommonPath::Document.get(),
              placeholder::WIN_PUBLIC => CommonPath::Public.get(),
              placeholder::SAVED_GAMES => CommonPath::SavedGames.get(),
              placeholder::WIN_PROGRAM_DATA => Some("C:/ProgramData"),
              placeholder::WIN_DIR => Some("C:/Windows"),
              _ => None,
            };

            if let Some(mapped) = mapped {
              analysis = Self::new(mapped.to_string()).analyze_with_mode(mode);
              continue;
            } else if let Some(basis) = &self.basis {
              analysis = Self::new(basis.clone()).analyze_with_mode(mode);
            }
          }

          if part == placeholder::OS_USER_NAME {
            analysis.parts.push(OS_USERNAME.to_string());
            continue;
          }

          if part.contains(['/', '\\']) {
            for part in part.split(['/', '\\']) {
              if !part.trim().is_empty() {
                analysis.parts.push(part.to_string());
              }
            }
          } else {
            analysis.parts.push(part.to_string());
          }
        }
      }
    }

    analysis
  }

  fn display(&self) -> String {
    if self.raw.is_empty() {
      return String::new();
    }

    match self.analyze() {
      Analysis {
        drive: Some(Drive::Root),
        parts,
      } => format!("/{}", parts.join("/")),
      Analysis {
        drive: Some(Drive::Windows(id)),
        parts,
      } => format!("{}/{}", id, parts.join("/")),
      Analysis { drive: None, parts } => parts.join("/"),
    }
  }

  fn access(&self) -> Result<String, StrictPathError> {
    self.access_with_mode(AnalysisMode::Normal)
  }

  fn access_with_mode(&self, mode: AnalysisMode) -> Result<String, StrictPathError> {
    if Os::HOST == Os::Windows {
      self.access_windows(mode)
    } else {
      self.access_nonwindows(mode)
    }
  }

  fn access_windows(&self, mode: AnalysisMode) -> Result<String, StrictPathError> {
    if self.raw.is_empty() {
      return Err(StrictPathError::Empty);
    }

    let analysis = self.analyze_with_mode(mode);
    if analysis.parts.iter().any(|x| x.contains(':')) {
      return Err(StrictPathError::Unsupported);
    }

    match analysis {
      Analysis {
        drive: Some(Drive::Root),
        ..
      } => Err(StrictPathError::Unsupported),
      Analysis {
        drive: Some(Drive::Windows(id)),
        parts,
      } => Ok(format!("{}\\{}", id, parts.join("\\"))),
      Analysis { drive: None, parts } => match &self.basis {
        Some(basis) => Ok(format!("{}\\{}", basis, parts.join("\\"))),
        None => Err(StrictPathError::Relative),
      },
    }
  }

  fn access_nonwindows(&self, mode: AnalysisMode) -> Result<String, StrictPathError> {
    if self.raw.is_empty() {
      return Err(StrictPathError::Empty);
    }

    match self.analyze_with_mode(mode) {
      Analysis {
        drive: Some(Drive::Root),
        parts,
      } => Ok(format!("/{}", parts.join("/"))),
      Analysis {
        drive: Some(Drive::Windows(_)),
        ..
      } => Err(StrictPathError::Unsupported),
      Analysis { drive: None, parts } => match &self.basis {
        Some(basis) => Ok(format!("{}/{}", basis, parts.join("/"))),
        None => Err(StrictPathError::Relative),
      },
    }
  }

  /// Glob-escaped form. Use with `globset`/`globetter`.
  pub fn globbable(&self) -> String {
    if self.raw.is_empty() {
      return String::new();
    }

    match self.analyze_with_mode(AnalysisMode::Glob) {
      Analysis {
        drive: Some(Drive::Root),
        parts,
      } => {
        format!("/{}", globset::escape(&parts.join("/")))
      }
      Analysis {
        drive: Some(Drive::Windows(id)),
        parts,
      } => {
        format!("{}/{}", id, globset::escape(&parts.join("/")))
      }
      Analysis { drive: None, parts } => globset::escape(&parts.join("/")),
    }
  }

  fn canonical(&self) -> Canonical {
    let mut cached = self.canonical.lock().unwrap();

    match cached.as_ref() {
      Some(canonical) => canonical.clone(),
      None => match self.access() {
        Err(_) => Canonical::Unsupported,
        Ok(path) => match std::fs::canonicalize(&path) {
          Err(_) => Canonical::Inaccessible,
          Ok(path) => {
            let path = path.to_string_lossy().to_string();
            *cached = Some(Canonical::Valid(path.clone()));
            Canonical::Valid(path)
          }
        },
      },
    }
  }

  /// OS-native form, canonicalized if the path exists. This is the form to
  /// hand to `std::fs::*` operations.
  pub fn interpret(&self) -> Result<String, StrictPathError> {
    match self.canonical() {
      Canonical::Valid(path) => match StrictPath::new(path).access() {
        Ok(path) => Ok(path),
        Err(_) => Ok(self.display()),
      },
      Canonical::Unsupported => Err(StrictPathError::Unsupported),
      Canonical::Inaccessible => self.access(),
    }
  }

  /// Display-friendly form (forward slashes, placeholders resolved). Stable for
  /// UI display — does not touch the file system.
  pub fn render(&self) -> String {
    self.display()
  }

  pub fn resolve(&self) -> String {
    self.access().unwrap_or_else(|_| self.raw().to_string())
  }

  pub fn equivalent(&self, other: &Self) -> bool {
    self.interpret() == other.interpret()
  }

  pub fn is_file(&self) -> bool {
    self
      .as_std_path_buf()
      .map(|x| x.is_file())
      .unwrap_or_default()
  }

  pub fn is_dir(&self) -> bool {
    self
      .as_std_path_buf()
      .map(|x| x.is_dir())
      .unwrap_or_default()
  }

  pub fn exists(&self) -> bool {
    self.is_file() || self.is_dir()
  }

  pub fn metadata(&self) -> std::io::Result<std::fs::Metadata> {
    self.as_std_path_buf()?.metadata()
  }

  pub fn read(&self) -> Option<String> {
    std::fs::read_to_string(self.as_std_path_buf().ok()?).ok()
  }

  pub fn try_read(&self) -> Result<String, crate::foundations::error::AnyError> {
    Ok(std::fs::read_to_string(self.as_std_path_buf()?)?)
  }

  pub fn write_with_content(&self, content: &str) -> std::io::Result<()> {
    std::fs::write(self.as_std_path_buf()?, content.as_bytes())
  }

  pub fn create_dirs(&self) -> std::io::Result<()> {
    std::fs::create_dir_all(self.as_std_path_buf()?)
  }

  pub fn create_parent_dir(&self) -> std::io::Result<()> {
    let mut pb = self.as_std_path_buf()?;
    pb.pop();
    std::fs::create_dir_all(&pb)
  }

  pub fn remove(&self) -> Result<(), crate::foundations::error::AnyError> {
    if self.is_file() {
      std::fs::remove_file(self.as_std_path_buf()?)?;
    } else if self.is_dir() {
      std::fs::remove_dir_all(self.as_std_path_buf()?)?;
    }
    Ok(())
  }

  pub fn joined(&self, other: impl AsRef<str>) -> Self {
    Self {
      raw: format!("{}/{}", self.raw, other.as_ref()).replace('\\', "/"),
      basis: self.basis.clone(),
      canonical: Arc::new(Mutex::new(None)),
    }
  }

  pub fn popped(&self) -> Self {
    let raw = match self.analyze() {
      Analysis {
        drive: Some(Drive::Root),
        mut parts,
      } => {
        parts.pop();
        format!("/{}", parts.join("/"))
      }
      Analysis {
        drive: Some(Drive::Windows(id)),
        mut parts,
      } => {
        parts.pop();
        format!("{}/{}", id, parts.join("/"))
      }
      Analysis {
        drive: None,
        mut parts,
      } => {
        parts.pop();
        match &self.basis {
          Some(basis) => format!("{}/{}", basis, parts.join("/")),
          None => parts.join("/"),
        }
      }
    };

    Self::new(raw)
  }

  pub fn replace(&self, find: &Self, new: &Self) -> Self {
    if find.raw.trim().is_empty() || new.raw.trim().is_empty() {
      return self.clone();
    }

    let us = self.analyze();
    let find = find.analyze();

    if us.drive != find.drive {
      return self.clone();
    }

    let mut tail = vec![];
    for pair in us.parts.into_iter().zip_longest(find.parts) {
      match pair {
        itertools::EitherOrBoth::Both(old, find) => {
          if old != find {
            return self.clone();
          }
        }
        itertools::EitherOrBoth::Left(old) => tail.push(old),
        itertools::EitherOrBoth::Right(..) => return self.clone(),
      }
    }

    let mut new = new.analyze();
    new.parts.extend(tail);
    new.into()
  }
}

impl std::fmt::Display for StrictPath {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.render())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn home_placeholder_resolves() {
    let path = StrictPath::new("<home>/foo");
    let rendered = path.render();
    assert!(rendered.ends_with("/foo"));
    assert!(!rendered.contains("<home>"));
  }

  #[test]
  fn os_user_name_placeholder_resolves() {
    let path = StrictPath::new("<home>/<osUserName>");
    let rendered = path.render();
    assert!(rendered.contains(OS_USERNAME.as_str()));
  }

  #[test]
  fn joined_keeps_basis() {
    let mut a = StrictPath::new("foo");
    a.basis = Some("/tmp".to_string());
    let b = a.joined("bar");
    assert_eq!(b.basis.as_deref(), Some("/tmp"));
  }

  #[test]
  fn empty_path_renders_empty() {
    let path = StrictPath::new(String::new());
    assert_eq!(path.render(), "");
  }
}
