//! Typed config / cache file persistence with versioning + migrations.
//! See `docs/LUDUSAVI_PATTERNS.md` section 3.

use crate::foundations::{app::app_dir, error::AnyError, path::StrictPath};

pub trait ResourceFile
where
  Self: Default + serde::de::DeserializeOwned,
{
  const FILE_NAME: &'static str;

  fn path() -> StrictPath {
    app_dir().joined(Self::FILE_NAME)
  }

  /// Apply defaults when the resource file does not yet exist.
  fn initialize(self) -> Self {
    self
  }

  /// Apply legacy migrations after loading.
  fn migrate(self) -> Self {
    self
  }

  fn load() -> Result<Self, AnyError> {
    Self::load_from(&Self::path())
  }

  fn load_from(path: &StrictPath) -> Result<Self, AnyError> {
    if !path.exists() {
      return Ok(Self::default().initialize());
    }
    let content = Self::load_raw(path)?;
    Self::load_from_string(&content)
  }

  fn load_from_existing(path: &StrictPath) -> Result<Self, AnyError> {
    let content = Self::load_raw(path)?;
    Self::load_from_string(&content)
  }

  fn load_raw(path: &StrictPath) -> Result<String, AnyError> {
    path.try_read()
  }

  fn load_from_string(content: &str) -> Result<Self, AnyError> {
    let format = detect_format(Self::FILE_NAME);
    let value: Self = match format {
      Format::Json => serde_json::from_str(content).map_err(|e| Box::new(e) as AnyError)?,
      Format::Yaml => serde_yaml::from_str(content).map_err(|e| Box::new(e) as AnyError)?,
    };
    Ok(ResourceFile::migrate(value))
  }
}

pub trait SaveableResourceFile
where
  Self: ResourceFile + serde::Serialize,
{
  /// Persist to disk if the serialized content differs from what's already there.
  fn save(&self) {
    let format = detect_format(Self::FILE_NAME);
    let new_content = match format {
      Format::Json => serde_json::to_string_pretty(&self).unwrap_or_default(),
      Format::Yaml => serde_yaml::to_string(&self).unwrap_or_default(),
    };

    if let Ok(old_content) = Self::load_raw(&Self::path())
      && old_content == new_content
    {
      return;
    }

    if Self::path().create_parent_dir().is_ok() {
      let _ = Self::path().write_with_content(&new_content);
    }
  }
}

#[derive(Clone, Copy)]
enum Format {
  Json,
  Yaml,
}

fn detect_format(file_name: &str) -> Format {
  if file_name.ends_with(".json") {
    Format::Json
  } else {
    Format::Yaml
  }
}
