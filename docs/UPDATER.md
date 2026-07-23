# Auto-updater

This scaffold ships a complete two-phase auto-update module:

| Piece                                                     | File                                                                          |
| --------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Rust commands (check / download+stage / support / staged) | `src-tauri/src/updater.rs`                                                    |
| Exit-time install hook                                    | `src-tauri/src/lib.rs` (`RunEvent::ExitRequested`)                            |
| Frontend service (settings, gates, error taxonomy)        | `src/lib/updater.ts`                                                          |
| Settings UI card                                          | `src/components/UpdaterCard.tsx` (rendered in `src/routes/settings.tsx`)      |
| Startup background check                                  | `src/routes/__root.tsx`                                                       |
| Placeholder-config guard                                  | `scripts/check-updater-config.mjs` (`pnpm run check:updater`, also a CI step) |

It ships **configured with placeholders**. Until you complete the setup below, the guard script
warns (and, once you rename the package from `tauri-app`, fails `pnpm check`).

## Setup

1. **Generate a minisign keypair** (back it up — losing it strands every shipped install on its
   current version):

   ```bash
   pnpm tauri signer generate -w ~/.tauri/myapp.key
   ```

   The command prints the public key and writes the private key to the given path.

2. **Fill in the public key** in `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`
   (replace `REPLACE_WITH_YOUR_MINISIGN_PUBLIC_KEY`).

3. **Point the endpoints at your releases repository** — in BOTH places:
   - `src-tauri/tauri.conf.json` → `plugins.updater.endpoints` (the plugin's static default), and
   - `src-tauri/src/updater.rs` → `UPDATE_MANIFEST_BASE` (the runtime per-channel override every
     command actually uses).

   If your app repo is **private**, releases must live in a separate **public** repo — private
   release assets 404 for anonymous downloads, which silently breaks both downloads and
   auto-update.

4. **Consume the release pipeline.** The channel/manifest layout matches
   [tauri-release-kit](https://github.com/OWNER/tauri-release-kit)'s reusable
   `release.yml`: call it from a tag-triggered workflow with the
   `TAURI_SIGNING_PRIVATE_KEY` (+ optional password) secrets set. It builds the 6-target matrix,
   signs updater artifacts, and publishes rolling per-channel manifests:
   - `stable` → `releases/latest/download/latest.json`
   - `beta` → `releases/download/latest-beta/latest.json`
   - `alpha` → `releases/download/latest-alpha/latest.json`

   Each manifest carries a per-platform `size` (bytes) extension — the Tauri plugin ignores it,
   but `check_for_updates` reads it so the UI can show the download size up front — and the
   release's changelog section as `notes`, which the card renders as release notes.

   If you use a different pipeline, keep that layout (or adjust `UpdateChannel::endpoint`), and
   note that `downloadSize` degrades to `null` when the manifest has no `size` field.

5. **Verify the guard passes for real:** `pnpm run check:updater` (set `CHECK_UPDATER_STRICT=1`
   to get the strict behavior before renaming the package).

Platform notes: `bundle.createUpdaterArtifacts: true` is set in `tauri.conf.json`;
`tauri.macos.conf.json` includes the `app` target (the `.app.tar.gz` updater artifact — keep it
alongside `dmg`). On Windows, prefer NSIS (WiX/MSI rejects prerelease semver like `-alpha.1`).

## Disabling the updater

If this app should not self-update, disable it consciously:

1. Remove the `plugins.updater` block and set `bundle.createUpdaterArtifacts` to `false` in
   `src-tauri/tauri.conf.json` (the guard then passes as "disabled").
2. Optionally strip the module: delete `src-tauri/src/updater.rs`, its registrations in
   `lib.rs`/`build.rs`/`capabilities/default.json`, `src/lib/updater.ts`,
   `src/components/UpdaterCard.tsx`, and the `check:updater` script.

## The UX contract

- **Two-phase install (download now, install on exit).** `download_update` downloads and
  signature-verifies the bundle, then _stages_ it in managed state. The actual install runs from
  `RunEvent::ExitRequested` — replacing the bundle of a live process breaks the macOS code
  signature/XPC services of the still-running app. "Restart to Update" just relaunches; the swap
  itself always happens on the way out. A staged update that is never restarted installs on the
  next natural quit.
- **Deferral is first-class.** "Skip this version" suppresses background notifications for
  exactly that version (self-healing: a newer release clears the skip). "Remind me later"
  suppresses all background nagging for a time window. Both only gate _background_ checks.
- **Silent-auto / loud-manual.** Background checks (startup, gated by the auto-check toggle +
  wall-clock interval) never surface errors — offline blips retry on the next gate pass — and
  only notify when there is genuinely something new. Manual checks always run, always report,
  and map failures onto a friendly taxonomy (network / no-release / signature / disk) with a
  Retry affordance where retrying can help.
- **Auto-download stages, never restarts.** With "Download automatically" on, a background check
  downloads and stages silently, then tells the user it will install on quit. A forced relaunch
  from a background path would discard unsaved work — only the explicit button relaunches.
- **Managed installs are respected.** On Linux, only AppImage can self-update; deb/rpm/Flatpak
  installs get a package-manager notice and are never nagged in-app.
- **Prerelease builds default to their own channel.** A `-alpha`/`-beta` version derives its
  default channel from itself, so it doesn't poll a `stable` endpoint that 404s until the first
  stable release exists. A user's explicit channel choice always wins.

## What this module deliberately does NOT do

- **Delta/differential updates** — every update downloads the full platform bundle. Simpler,
  and the Tauri updater has no delta support anyway.
- **Rollback / downgrade** — there is no "install previous version" path. If a release is bad,
  publish a fixed newer version; the updater only ever moves forward (switching to a channel
  whose latest is older than the installed version just reports "up to date").
- **Update-server accounts, phased rollouts, or metrics** — it polls a static JSON manifest on
  GitHub releases; there is no server-side targeting.
- **In-place install while running** — by design (see the UX contract above).
