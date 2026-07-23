/**
 * Frontend updater service — the single place UI code talks to the Rust updater module
 * (`src-tauri/src/updater.rs`).
 *
 * UX contract:
 * - Two-phase install: `downloadUpdate` stages the verified bundle; it installs on the next
 *   quit. `restartToUpdate` relaunches immediately (explicit, user-initiated only).
 * - Silent-auto / loud-manual: background checks (`maybeBackgroundCheck`) never surface errors
 *   and honor per-version skip + remind-later; manual checks always run and always report.
 *
 * Settings persist in localStorage (the template has no @tauri-apps/plugin-store dependency;
 * swap `readSettings`/`writeSettings` if you add one).
 */

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { relaunch } from '@tauri-apps/plugin-process'

import { getAppInfo } from './tauri'

export type UpdateChannel = 'stable' | 'beta' | 'alpha'

export interface UpdateCheckResult {
  available: boolean
  currentVersion: string
  latestVersion: string | null
  body: string | null
  date: string | null
  /**
   * Size in bytes of this platform's update bundle, from the release pipeline's manifest `size`
   * extension. Null when the manifest predates the extension or the lookup failed — UI copy
   * falls back to sizeless wording.
   */
  downloadSize: number | null
}

export interface UpdateProgressEvent {
  downloadedBytes: number
  totalBytes: number | null
  finished: boolean
}

export type UpdateInstallSupport = 'supported' | 'packageManager'

export interface UpdateSettings {
  channel: UpdateChannel
  autoCheck: boolean
  autoDownload: boolean
  /** Minimum hours between background checks (wall-clock gate). */
  checkIntervalHours: number
  /** Epoch ms of the last completed check (manual or background). */
  lastChecked?: number
  /**
   * A version the user explicitly skipped ("Skip this version"): background checks stay silent
   * about exactly this version. A newer release clears the suppression by not matching.
   */
  skippedVersion?: string
  /**
   * "Remind me later": background checks stay silent (for any version) until this epoch-ms
   * timestamp passes. Manual checks always bypass it.
   */
  remindAfter?: number
}

export const UPDATE_CHANNELS: UpdateChannel[] = ['stable', 'beta', 'alpha']

export const DEFAULT_UPDATE_SETTINGS: UpdateSettings = {
  channel: 'stable',
  autoCheck: true,
  autoDownload: false,
  checkIntervalHours: 24,
}

const SETTINGS_STORAGE_KEY = 'updater:settings'

/**
 * Derive the update channel a build should default to from its own version string: a prerelease
 * build must check its own prerelease channel, not `stable` (whose endpoint 404s until a stable
 * release exists). Pure — a persisted user choice always wins over this derived default.
 */
export function deriveUpdateChannel(version: string): UpdateChannel {
  if (version.includes('-alpha')) {
    return 'alpha'
  }
  if (version.includes('-beta')) {
    return 'beta'
  }
  return 'stable'
}

/** Format a byte count for update UI ("42.3 MB"); non-finite/negative in, undefined out. */
export function formatUpdateSize(bytes: number | null | undefined): string | undefined {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) {
    return undefined
  }
  const mb = bytes / (1024 * 1024)
  return mb >= 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(1)} MB`
}

/** Human-readable classification of an update failure, for UI copy + a Retry affordance. */
export interface UpdateErrorInfo {
  title: string
  detail: string
  /** False for failures where retrying is useless or unsafe (e.g. a bad signature). */
  retryable: boolean
}

/**
 * Map a raw update error onto friendly copy (network / not-found / signature / disk / generic).
 * Pure and substring-based: the updater plugin only surfaces stringly-typed errors over the wire.
 */
export function describeUpdateError(error: unknown): UpdateErrorInfo {
  const message = error instanceof Error ? error.message : String(error)
  const lower = message.toLowerCase()
  if (lower.includes('signature') || lower.includes('minisign')) {
    return {
      title: "The update couldn't be verified",
      detail:
        'The downloaded update failed its cryptographic signature check, so it was discarded. This can indicate a corrupted download or a compromised source — try again later; the current version keeps working.',
      retryable: false,
    }
  }
  if (
    lower.includes('error sending request') ||
    lower.includes('connection') ||
    lower.includes('timed out') ||
    lower.includes('timeout') ||
    lower.includes('dns') ||
    lower.includes('network')
  ) {
    return {
      title: "Couldn't reach the update server",
      detail: 'Check your internet connection and try again.',
      retryable: true,
    }
  }
  if (lower.includes('404') || lower.includes('could not fetch a valid release json')) {
    return {
      title: 'No update found on this channel',
      detail: 'The release channel has no published update right now.',
      retryable: true,
    }
  }
  if (lower.includes('no space') || lower.includes('disk')) {
    return {
      title: 'Not enough disk space',
      detail: 'Free up some space and try downloading the update again.',
      retryable: true,
    }
  }
  return { title: 'Update failed', detail: message, retryable: true }
}

/**
 * The channel endpoint had no valid release JSON — exactly what a channel with no published
 * releases yet produces (e.g. `stable` before the first stable release ships). Not a real error:
 * callers show a calm "no releases on this channel yet" state instead of a failure.
 */
export class NoReleasesOnChannelError extends Error {
  readonly channel: UpdateChannel

  constructor(channel: UpdateChannel) {
    super(`No releases have been published on the ${channel} channel yet.`)
    this.name = 'NoReleasesOnChannelError'
    this.channel = channel
  }
}

/**
 * Whether a raw command failure is the updater plugin's `ReleaseNotFound` — the endpoint
 * answered but had no usable release JSON (a 404ing channel endpoint lands here). The plugin's
 * message is stable and is the only signal the wire error carries.
 */
const isMissingReleaseError = (error: unknown): boolean =>
  String(error).includes('Could not fetch a valid release JSON')

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null

const isChannel = (value: unknown): value is UpdateChannel =>
  typeof value === 'string' && (UPDATE_CHANNELS as string[]).includes(value)

function readSettings(): UpdateSettings | null {
  const raw = localStorage.getItem(SETTINGS_STORAGE_KEY)
  if (raw === null) {
    return null
  }
  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch {
    // Corrupt persisted settings — fall back to defaults rather than wedging the updater UI.
    localStorage.removeItem(SETTINGS_STORAGE_KEY)
    return null
  }
  if (!isRecord(parsed) || !isChannel(parsed.channel)) {
    return null
  }
  return {
    ...DEFAULT_UPDATE_SETTINGS,
    ...parsed,
    channel: parsed.channel,
  }
}

function writeSettings(settings: UpdateSettings): void {
  localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings))
}

/** Memoized version-derived default channel, so getSettings doesn't re-IPC on every call. */
let derivedDefaultChannel: UpdateChannel | null = null

async function resolveDefaultChannel(): Promise<UpdateChannel> {
  if (derivedDefaultChannel) {
    return derivedDefaultChannel
  }
  try {
    const { version } = await getAppInfo()
    derivedDefaultChannel = deriveUpdateChannel(version)
  } catch {
    // The default channel must never block settings loading (e.g. running outside Tauri).
    derivedDefaultChannel = DEFAULT_UPDATE_SETTINGS.channel
  }
  return derivedDefaultChannel
}

/**
 * Current updater settings. A persisted choice (user-made or previously captured) always wins;
 * otherwise the default channel is derived from the running app's version so an alpha/beta build
 * checks its own channel instead of `stable`.
 */
export async function getUpdateSettings(): Promise<UpdateSettings> {
  const persisted = readSettings()
  if (persisted) {
    return persisted
  }
  return { ...DEFAULT_UPDATE_SETTINGS, channel: await resolveDefaultChannel() }
}

export async function saveUpdateSettings(patch: Partial<UpdateSettings>): Promise<UpdateSettings> {
  const next = { ...(await getUpdateSettings()), ...patch }
  writeSettings(next)
  return next
}

/**
 * Check the channel's manifest for a newer version (never downloads). Stamps `lastChecked`.
 * Throws {@link NoReleasesOnChannelError} when the channel has no published release yet.
 */
export async function checkForUpdates(channel?: UpdateChannel): Promise<UpdateCheckResult> {
  const settings = await getUpdateSettings()
  const checkChannel = channel ?? settings.channel
  let result: UpdateCheckResult
  try {
    result = await invoke<UpdateCheckResult>('check_for_updates', { channel: checkChannel })
  } catch (err: unknown) {
    if (isMissingReleaseError(err)) {
      throw new NoReleasesOnChannelError(checkChannel)
    }
    throw err instanceof Error ? err : new Error(String(err))
  }
  await saveUpdateSettings({ lastChecked: Date.now() })
  return result
}

/**
 * Download + signature-verify the update and stage it for install on the next quit. Subscribe
 * to {@link onUpdateProgress} before calling to render live progress.
 */
export async function downloadUpdate(channel?: UpdateChannel): Promise<void> {
  const settings = await getUpdateSettings()
  try {
    await invoke('download_update', { channel: channel ?? settings.channel })
  } catch (err: unknown) {
    throw err instanceof Error ? err : new Error(String(err))
  }
}

/**
 * Relaunch now so the staged update installs. Explicit, user-initiated only — this discards any
 * unsaved state. Without it, the staged update still installs on the next natural quit.
 */
export async function restartToUpdate(): Promise<void> {
  await relaunch()
}

/** Whether an update has been downloaded and staged this session (installs on the next quit). */
export const isUpdateStaged = (): Promise<boolean> => invoke<boolean>('update_staged')

/** Memoized install-support verdict (constant for a process lifetime). */
let installSupport: UpdateInstallSupport | null = null

/** Whether the in-app updater can service this install (Linux deb/rpm/Flatpak → 'packageManager'). */
export async function getInstallSupport(): Promise<UpdateInstallSupport> {
  if (installSupport) {
    return installSupport
  }
  const result = await invoke<{ kind: string }>('update_install_support')
  installSupport = result.kind === 'packageManager' ? 'packageManager' : 'supported'
  return installSupport
}

/** Subscribe to download progress; resolves to the unlisten function. */
export const onUpdateProgress = (
  handler: (progress: UpdateProgressEvent) => void,
): Promise<UnlistenFn> =>
  listen<UpdateProgressEvent>('update-progress', (event) => {
    handler(event.payload)
  })

/** Switch channels and immediately check the new channel. */
export async function switchChannel(channel: UpdateChannel): Promise<UpdateCheckResult> {
  await saveUpdateSettings({ channel })
  return await checkForUpdates(channel)
}

/** Suppress background notifications about exactly this version ("Skip this version"). */
export async function skipVersion(version: string): Promise<void> {
  await saveUpdateSettings({ skippedVersion: version, remindAfter: undefined })
}

/** Suppress background update notifications for `hours` (default 24) — "Remind me later". */
export async function remindLater(hours = 24): Promise<void> {
  await saveUpdateSettings({ remindAfter: Date.now() + hours * 60 * 60 * 1000 })
}

/** Wall-clock gate: true when auto-check is on and the check interval has elapsed. */
async function shouldCheckForUpdates(): Promise<boolean> {
  const settings = await getUpdateSettings()
  if (!settings.autoCheck) {
    return false
  }
  if (!settings.lastChecked) {
    return true
  }
  const hoursSinceLastCheck = (Date.now() - settings.lastChecked) / (1000 * 60 * 60)
  return hoursSinceLastCheck >= settings.checkIntervalHours
}

/**
 * Whether background surfacing of `version` is suppressed by a per-version skip or an active
 * remind-later window. Also self-heals: a skip recorded for a version that is no longer the
 * latest is cleared, so stale skips never linger.
 */
async function isSuppressed(version: string | null): Promise<boolean> {
  const settings = await getUpdateSettings()
  if (settings.remindAfter && Date.now() < settings.remindAfter) {
    return true
  }
  if (!settings.skippedVersion) {
    return false
  }
  if (version && settings.skippedVersion === version) {
    return true
  }
  // A newer version shipped since the skip was recorded — drop the stale suppression.
  await saveUpdateSettings({ skippedVersion: undefined })
  return false
}

interface UpdateAvailableEvent {
  /**
   * `available` — a newer version exists but is not downloaded; `staged` — it has been
   * downloaded (auto-download path) and installs on the next quit.
   */
  kind: 'available' | 'staged'
  result: UpdateCheckResult
}

type UpdateNotifier = (event: UpdateAvailableEvent) => void

/** Re-entrancy guard: startup/focus/interval ticks can coincide; one cycle at a time. */
let backgroundCheckInFlight = false

/**
 * One gated background check cycle: honors the wall-clock gate, per-version skip, and
 * remind-later suppression, auto-downloads when enabled, and surfaces results via `notify`.
 * Silent on every expected non-result (suppressed, up to date, offline blip, package-manager
 * install). Use for startup/interval checks — never for manual checks.
 */
export async function maybeBackgroundCheck(notify: UpdateNotifier): Promise<void> {
  if (backgroundCheckInFlight) {
    return
  }
  backgroundCheckInFlight = true
  try {
    if (!(await shouldCheckForUpdates())) {
      return
    }
    if ((await getInstallSupport()) === 'packageManager') {
      // Managed installs update through their package manager — never nag in-app.
      return
    }
    const settings = await getUpdateSettings()
    const result = await checkForUpdates()
    if (!result.available || (await isSuppressed(result.latestVersion))) {
      return
    }
    if (settings.autoDownload) {
      // Stage only — never force a relaunch from a background path (that would discard the
      // user's unsaved state). The staged bundle installs on the next natural quit.
      await downloadUpdate()
      notify({ kind: 'staged', result })
    } else {
      notify({ kind: 'available', result })
    }
  } catch (err: unknown) {
    // Background checks are silent toward the user on failure (offline blips retry on the next
    // gate pass) but never swallowed entirely — keep the console trace for diagnostics.
    if (!(err instanceof NoReleasesOnChannelError)) {
      // oxlint-disable-next-line no-console -- deliberate: the only failure trace for silent background checks
      console.warn('[updater] background update check failed:', err)
    }
  } finally {
    backgroundCheckInFlight = false
  }
}
