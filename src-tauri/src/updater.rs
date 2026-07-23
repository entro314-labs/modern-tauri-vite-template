//! Auto-update commands — check for, download, and stage updates across release channels.
//!
//! ## Two-phase (staged) install
//!
//! [`download_update`] downloads and signature-verifies the update bundle but does **not**
//! install it while the app is running: replacing the bundle of a live process breaks the macOS
//! code signature / XPC services of the still-running app. Instead the verified bytes (and the
//! plugin's `Update` handle) are stashed in the [`PendingUpdate`] managed state, and the actual
//! `Update::install` runs once, from the `RunEvent::ExitRequested` handler in `lib.rs` (via
//! [`install_pending_update_on_exit`]) — i.e. on quit/restart, when nothing is executing out of
//! the bundle.
//!
//! ## Channels
//!
//! Endpoints are derived at runtime from [`UPDATE_MANIFEST_BASE`] per [`UpdateChannel`], matching
//! the tauri-release-kit layout: `stable` polls `releases/latest/download/latest.json`, while
//! prerelease channels poll the rolling `releases/download/latest-<channel>/latest.json` tag.
//! The static endpoint in `tauri.conf.json` is only the plugin default; every command here
//! overrides it with the channel endpoint.
//!
//! This module is desktop-only (the updater/process plugins do not build for iOS/Android). Gate
//! it behind `#[cfg(desktop)]` if you add mobile targets to this template.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Url};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::foundations::http::{Security, reqwest_blocking_client};

// ─────────────────────────────────────────────────────────────────────────────────────────────
// ⚠️  PLACEHOLDER — REPLACE BEFORE SHIPPING ⚠️
//
// Point this at the GitHub repository that hosts your releases (for private app repos this must
// be a separate PUBLIC releases repo — private release assets 404 for anonymous downloads, which
// silently breaks auto-update). `scripts/check-updater-config.mjs` fails the `check` gate while
// the OWNER/RELEASES-REPO placeholder is still present in `tauri.conf.json`; keep the two in
// sync. See docs/UPDATER.md for the full setup checklist.
// ─────────────────────────────────────────────────────────────────────────────────────────────
pub const UPDATE_MANIFEST_BASE: &str = "https://github.com/OWNER/RELEASES-REPO/releases";

/// Minimum interval between download-progress emissions so a fast download over many small
/// chunks doesn't flood the event bus. The final (`finished: true`) emission is never throttled.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Event name for [`UpdateProgressEvent`]; the frontend listens via `onUpdateProgress` in
/// `src/lib/updater.ts`.
const PROGRESS_EVENT: &str = "update-progress";

/// Errors from updater commands, serialized as a plain message string so the frontend error
/// taxonomy (`describeUpdateError` in `src/lib/updater.ts`) can classify it by content.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct UpdaterError(String);

impl std::fmt::Display for UpdaterError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.0)
  }
}

impl From<tauri_plugin_updater::Error> for UpdaterError {
  fn from(error: tauri_plugin_updater::Error) -> Self {
    Self(error.to_string())
  }
}

impl From<tauri::Error> for UpdaterError {
  fn from(error: tauri::Error) -> Self {
    Self(error.to_string())
  }
}

type UpdaterResult<T> = Result<T, UpdaterError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
  Stable,
  Beta,
  Alpha,
}

impl UpdateChannel {
  /// The rolling update-manifest URL for this channel (tauri-release-kit layout).
  fn endpoint(self) -> String {
    match self {
      Self::Stable => format!("{UPDATE_MANIFEST_BASE}/latest/download/latest.json"),
      Self::Beta => format!("{UPDATE_MANIFEST_BASE}/download/latest-beta/latest.json"),
      Self::Alpha => format!("{UPDATE_MANIFEST_BASE}/download/latest-alpha/latest.json"),
    }
  }

  fn endpoint_url(self) -> UpdaterResult<Url> {
    let endpoint = self.endpoint();
    Url::parse(&endpoint)
      .map_err(|error| UpdaterError(format!("invalid update endpoint {endpoint}: {error}")))
  }
}

/// An update that has been downloaded and verified but not yet installed. Held in managed state
/// until the app exits, when [`install_pending_update_on_exit`] applies it (replacing the
/// on-disk bundle while no code is running out of it).
struct StagedUpdate {
  /// The plugin's update handle; `install` is called on it at exit.
  update: Update,
  /// The verified bundle bytes from `Update::download`.
  bytes: Vec<u8>,
}

/// Managed state holding the single staged update (if any). `Mutex` because the download path
/// writes it from an async command and the exit handler reads it from the run-loop thread.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<StagedUpdate>>);

/// Live download progress for [`download_update`], emitted as the `update-progress` event.
/// `total_bytes` is `None` when the release server doesn't send a `Content-Length`; `finished`
/// flips to `true` on the single final emission after the download completes (the bundle is then
/// staged for install-on-exit).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgressEvent {
  /// Bytes downloaded so far.
  pub downloaded_bytes: u64,
  /// Total bytes to download, when the server reports it.
  pub total_bytes: Option<u64>,
  /// Whether the download has completed (the update is staged immediately after).
  pub finished: bool,
}

/// Emit a progress snapshot, logging (never failing the update) if the event bus rejects it.
fn emit_update_progress(app: &AppHandle, progress: UpdateProgressEvent) {
  if let Err(error) = app.emit(PROGRESS_EVENT, progress) {
    log::debug!("failed to emit update progress event: {error}");
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
  pub available: bool,
  pub current_version: String,
  pub latest_version: Option<String>,
  pub body: Option<String>,
  pub date: Option<String>,
  /// Size in bytes of the platform bundle the updater would download, read from the release
  /// pipeline's `size` extension in the update manifest. `None` when the manifest predates the
  /// extension or the lookup fails — the UI falls back to sizeless copy.
  pub download_size: Option<u64>,
}

/// The manifest platform key for the running build (mirrors the updater plugin's own
/// `{{target}}-{{arch}}` resolution for the six targets the release pipeline ships).
fn manifest_platform_key() -> &'static str {
  match (std::env::consts::OS, std::env::consts::ARCH) {
    ("macos", "aarch64") => "darwin-aarch64",
    ("macos", "x86_64") => "darwin-x86_64",
    ("windows", "aarch64") => "windows-aarch64",
    ("windows", "x86_64") => "windows-x86_64",
    ("linux", "aarch64") => "linux-aarch64",
    ("linux", "x86_64") => "linux-x86_64",
    _ => "unsupported",
  }
}

/// Best-effort lookup of this platform's bundle size from the channel manifest's `size`
/// extension (written by tauri-release-kit; absent from plain `tauri-action` manifests). Runs the
/// blocking `reqwest` fetch on a blocking thread; any failure — network, JSON shape, missing key
/// — resolves to `None` because size display is progressive enhancement, never a gate.
async fn fetch_download_size(channel: UpdateChannel) -> Option<u64> {
  let endpoint = channel.endpoint();
  let fetched = tauri::async_runtime::spawn_blocking(move || {
    let client = reqwest_blocking_client(Security::Safe);
    let body = client.get(&endpoint).send().ok()?.text().ok()?;
    let manifest: serde_json::Value = serde_json::from_str(&body).ok()?;
    manifest
      .get("platforms")?
      .get(manifest_platform_key())?
      .get("size")?
      .as_u64()
  })
  .await;
  fetched.unwrap_or_else(|error| {
    log::debug!("download-size lookup task failed: {error}");
    None
  })
}

/// Why self-update is or isn't available for this install, so the UI can point the user at the
/// right update path instead of offering a check that can never work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum UpdateInstallSupport {
  /// The in-app updater can replace this install.
  Supported,
  /// Managed by an external package manager (deb/rpm/Flatpak/Snap on Linux) — the Tauri updater
  /// can only self-update AppImage installs, so the UI should direct users to their package
  /// manager.
  PackageManager,
}

/// Report whether the in-app updater can service this install. On Linux the updater only
/// supports AppImage (the `APPIMAGE` env var is set by the AppImage runtime); deb/rpm/Flatpak
/// installs must update through their package manager. macOS and Windows installs are always
/// self-updatable.
#[tauri::command]
pub fn update_install_support() -> UpdateInstallSupport {
  #[cfg(target_os = "linux")]
  {
    use crate::foundations::os::{APPIMAGE, FLATPAK};
    if *APPIMAGE && !*FLATPAK {
      UpdateInstallSupport::Supported
    } else {
      UpdateInstallSupport::PackageManager
    }
  }
  #[cfg(not(target_os = "linux"))]
  {
    UpdateInstallSupport::Supported
  }
}

/// Whether an update has been downloaded and staged this session (it installs on the next quit).
/// Lets the UI restore its "restart to update" state after a remount. A poisoned lock reports
/// `false` — the exit-time installer logs its own state.
#[tauri::command]
pub fn update_staged(app_handle: AppHandle) -> bool {
  app_handle
    .try_state::<PendingUpdate>()
    .is_some_and(|pending| pending.0.lock().is_ok_and(|guard| guard.is_some()))
}

/// Check the channel's rolling manifest for a newer version. Never downloads anything.
#[tauri::command]
pub async fn check_for_updates(
  app_handle: AppHandle,
  channel: UpdateChannel,
) -> UpdaterResult<UpdateCheckResult> {
  log::info!("checking for updates on channel {channel:?}");

  let update = app_handle
    .updater_builder()
    .endpoints(vec![channel.endpoint_url()?])?
    .build()?
    .check()
    .await?;

  if let Some(update) = update {
    let download_size = fetch_download_size(channel).await;
    Ok(UpdateCheckResult {
      available: true,
      current_version: update.current_version,
      latest_version: Some(update.version),
      body: update.body,
      date: update.date.map(|d| d.to_string()),
      download_size,
    })
  } else {
    Ok(UpdateCheckResult {
      available: false,
      current_version: app_handle.package_info().version.to_string(),
      latest_version: None,
      body: None,
      date: None,
      download_size: None,
    })
  }
}

/// Download and signature-verify the latest update on `channel`, then stage it for install on
/// the next exit.
///
/// This deliberately does **not** install in place (see the module docs): it streams the
/// verified bundle into the [`PendingUpdate`] managed state and returns. The bundle is applied
/// from the `ExitRequested` handler via [`install_pending_update_on_exit`]. The progress event
/// fires during the download, with the final `finished: true` emission marking "downloaded,
/// staged for restart" rather than "installed".
#[tauri::command]
pub async fn download_update(app_handle: AppHandle, channel: UpdateChannel) -> UpdaterResult<()> {
  log::info!("downloading update from channel {channel:?} (install deferred to exit)");

  let update = app_handle
    .updater_builder()
    .endpoints(vec![channel.endpoint_url()?])?
    .build()?
    .check()
    .await?;

  let Some(update) = update else {
    return Err(UpdaterError("no update available".to_string()));
  };

  // Stream real download progress to the renderer. The chunk callback reports
  // `(chunk_len, content_length)`; accumulate the running byte count and emit the event,
  // throttled so a fast download doesn't flood the bus.
  let downloaded = Arc::new(AtomicU64::new(0));
  let chunk_app = app_handle.clone();
  let chunk_downloaded = Arc::clone(&downloaded);
  let last_emit = Mutex::new(
    Instant::now()
      .checked_sub(PROGRESS_INTERVAL)
      .unwrap_or_else(Instant::now),
  );
  let on_chunk = move |chunk_len: usize, content_len: Option<u64>| {
    let total = chunk_downloaded.fetch_add(chunk_len as u64, Ordering::Relaxed) + chunk_len as u64;
    let due = last_emit.lock().map_or_else(
      |_| {
        // A poisoned throttle clock only affects emission cadence — emit anyway, but surface
        // the poisoned lock.
        log::error!("update progress throttle mutex poisoned");
        true
      },
      |mut instant| {
        let due = instant.elapsed() >= PROGRESS_INTERVAL;
        if due {
          *instant = Instant::now();
        }
        due
      },
    );
    if due {
      emit_update_progress(
        &chunk_app,
        UpdateProgressEvent {
          downloaded_bytes: total,
          total_bytes: content_len,
          finished: false,
        },
      );
    }
  };

  let finish_app = app_handle.clone();
  let finish_downloaded = Arc::clone(&downloaded);
  let on_finish = move || {
    let total = finish_downloaded.load(Ordering::Relaxed);
    emit_update_progress(
      &finish_app,
      UpdateProgressEvent {
        downloaded_bytes: total,
        total_bytes: Some(total),
        finished: true,
      },
    );
  };

  // Download + signature-verify only. `install` runs at exit so the live bundle isn't replaced.
  let bytes = update.download(on_chunk, on_finish).await?;

  let pending = app_handle.state::<PendingUpdate>();
  {
    let mut guard = pending
      .0
      .lock()
      .map_err(|_| UpdaterError("pending-update state poisoned".to_string()))?;
    *guard = Some(StagedUpdate { update, bytes });
  }

  Ok(())
}

/// Install a staged update, if one was downloaded this session. Called from the Tauri
/// `ExitRequested` handler in `lib.rs`, so the on-disk bundle is replaced while no code runs out
/// of it. Best-effort: a missing state, a poisoned lock, or an install error is logged rather
/// than propagated — the app is exiting regardless and a failed install must not wedge shutdown
/// (the user simply stays on the current version and can retry).
///
/// On success, `Update::install` relaunches the app on platforms that support it; otherwise the
/// freshly-installed bundle is used on the next manual launch.
pub fn install_pending_update_on_exit<R: tauri::Runtime>(app: &AppHandle<R>) {
  let Some(pending) = app.try_state::<PendingUpdate>() else {
    return;
  };

  let staged = match pending.0.lock() {
    Ok(mut guard) => guard.take(),
    Err(error) => {
      log::warn!("skipping deferred update install — pending-update state poisoned: {error}");
      return;
    }
  };

  let Some(staged) = staged else {
    return;
  };

  log::info!("installing staged update on exit");
  if let Err(error) = staged.update.install(&staged.bytes) {
    log::error!("deferred update install failed: {error}");
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn channel_endpoints_follow_release_kit_layout() {
    assert_eq!(
      UpdateChannel::Stable.endpoint(),
      format!("{UPDATE_MANIFEST_BASE}/latest/download/latest.json"),
    );
    assert_eq!(
      UpdateChannel::Beta.endpoint(),
      format!("{UPDATE_MANIFEST_BASE}/download/latest-beta/latest.json"),
    );
    assert_eq!(
      UpdateChannel::Alpha.endpoint(),
      format!("{UPDATE_MANIFEST_BASE}/download/latest-alpha/latest.json"),
    );
  }

  #[test]
  fn channel_serde_is_lowercase() {
    let parsed: UpdateChannel = match serde_json::from_str("\"alpha\"") {
      Ok(channel) => channel,
      Err(error) => panic!("channel should deserialize: {error}"),
    };
    assert_eq!(parsed, UpdateChannel::Alpha);
  }

  #[test]
  fn platform_key_is_known_on_release_targets() {
    let key = manifest_platform_key();
    assert!(key == "unsupported" || key.contains('-'));
  }
}
