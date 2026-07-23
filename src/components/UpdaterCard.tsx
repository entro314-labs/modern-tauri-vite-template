/**
 * Self-contained settings card for the auto-updater. All updater IPC goes through
 * `src/lib/updater.ts`; this component owns only presentation state.
 *
 * States: idle → checking → upToDate | noReleases | available → downloading → staged | error.
 * Managed Linux installs (deb/rpm/Flatpak) render a package-manager notice instead of controls.
 */

import { useCallback, useEffect, useRef, useState } from 'react'

import {
  checkForUpdates,
  describeUpdateError,
  downloadUpdate,
  formatUpdateSize,
  getInstallSupport,
  getUpdateSettings,
  isUpdateStaged,
  NoReleasesOnChannelError,
  onUpdateProgress,
  remindLater,
  restartToUpdate,
  saveUpdateSettings,
  skipVersion,
  switchChannel,
  UPDATE_CHANNELS,
  type UpdateChannel,
  type UpdateCheckResult,
  type UpdateErrorInfo,
  type UpdateInstallSupport,
  type UpdateProgressEvent,
  type UpdateSettings,
} from '../lib/updater'
import { Button } from './ui/button'

type CardPhase =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'upToDate' }
  | { kind: 'noReleases'; channel: UpdateChannel }
  | { kind: 'available'; result: UpdateCheckResult }
  | { kind: 'downloading'; result: UpdateCheckResult; progress: UpdateProgressEvent | null }
  | { kind: 'staged'; latestVersion: string | null }
  | { kind: 'error'; info: UpdateErrorInfo; retry: () => void }

const asMegabytes = (bytes: number): string => (bytes / (1024 * 1024)).toFixed(1)

function ProgressBar({ progress }: { progress: UpdateProgressEvent | null }) {
  const total = progress?.totalBytes ?? null
  const percent =
    progress && total !== null && total > 0
      ? Math.min(100, Math.round((progress.downloadedBytes / total) * 100))
      : null

  return (
    <div className="space-y-1">
      <div
        role="progressbar"
        aria-label="Update download progress"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent ?? undefined}
        className="h-2 w-full overflow-hidden rounded-full bg-[var(--color-background)]"
      >
        <div
          className={`h-full rounded-full bg-[var(--color-accent)] ${
            percent === null ? 'w-1/3 animate-pulse' : ''
          }`}
          style={percent === null ? undefined : { width: `${percent}%` }}
        />
      </div>
      <p className="text-xs opacity-70">
        {progress === null
          ? 'Starting download…'
          : percent === null
            ? `Downloaded ${asMegabytes(progress.downloadedBytes)} MB…`
            : `${percent}% — ${asMegabytes(progress.downloadedBytes)} of ${asMegabytes(total ?? 0)} MB`}
      </p>
    </div>
  )
}

export function UpdaterCard() {
  const [phase, setPhase] = useState<CardPhase>({ kind: 'idle' })
  const [settings, setSettings] = useState<UpdateSettings | null>(null)
  const [installSupport, setInstallSupport] = useState<UpdateInstallSupport | null>(null)
  const [currentVersion, setCurrentVersion] = useState<string>('')
  const isMounted = useRef(true)

  useEffect(function trackMountedFlag() {
    isMounted.current = true
    return () => {
      isMounted.current = false
    }
  }, [])

  // Bootstrap: settings, install support, current version, and a possibly already-staged update
  // (restores "restart to update" after a remount).
  useEffect(function bootstrapUpdaterCard() {
    const bootstrap = async (): Promise<void> => {
      try {
        const [loadedSettings, support, staged] = await Promise.all([
          getUpdateSettings(),
          getInstallSupport(),
          isUpdateStaged(),
        ])
        const { getAppInfo } = await import('../lib/tauri')
        const info = await getAppInfo()
        if (!isMounted.current) {
          return
        }
        setSettings(loadedSettings)
        setInstallSupport(support)
        setCurrentVersion(info.version)
        if (staged) {
          setPhase({ kind: 'staged', latestVersion: null })
        }
      } catch {
        // Outside a Tauri host (plain browser dev) the card stays in idle with no version.
      }
    }
    void bootstrap()
  }, [])

  const runCheck = useCallback((channel?: UpdateChannel) => {
    setPhase({ kind: 'checking' })
    const doCheck = async (): Promise<void> => {
      try {
        const result = channel ? await switchChannel(channel) : await checkForUpdates()
        if (!isMounted.current) {
          return
        }
        setPhase(result.available ? { kind: 'available', result } : { kind: 'upToDate' })
      } catch (err: unknown) {
        if (!isMounted.current) {
          return
        }
        if (err instanceof NoReleasesOnChannelError) {
          setPhase({ kind: 'noReleases', channel: err.channel })
          return
        }
        setPhase({
          kind: 'error',
          info: describeUpdateError(err),
          retry: () => {
            runCheck(channel)
          },
        })
      }
    }
    void doCheck()
  }, [])

  const runDownload = useCallback((result: UpdateCheckResult) => {
    setPhase({ kind: 'downloading', result, progress: null })
    let unlisten: (() => void) | null = null
    const doDownload = async (): Promise<void> => {
      try {
        unlisten = await onUpdateProgress((progress) => {
          if (isMounted.current) {
            setPhase({ kind: 'downloading', result, progress })
          }
        })
        await downloadUpdate()
        if (isMounted.current) {
          setPhase({ kind: 'staged', latestVersion: result.latestVersion })
        }
      } catch (err: unknown) {
        if (isMounted.current) {
          setPhase({
            kind: 'error',
            info: describeUpdateError(err),
            retry: () => {
              runDownload(result)
            },
          })
        }
      } finally {
        unlisten?.()
      }
    }
    void doDownload()
  }, [])

  const handleSkip = useCallback((result: UpdateCheckResult) => {
    const doSkip = async (): Promise<void> => {
      if (result.latestVersion) {
        await skipVersion(result.latestVersion)
      }
      if (isMounted.current) {
        setPhase({ kind: 'idle' })
      }
    }
    void doSkip()
  }, [])

  const handleRemindLater = useCallback(() => {
    const doRemind = async (): Promise<void> => {
      await remindLater()
      if (isMounted.current) {
        setPhase({ kind: 'idle' })
      }
    }
    void doRemind()
  }, [])

  const updateSetting = useCallback((patch: Partial<UpdateSettings>) => {
    const doSave = async (): Promise<void> => {
      const next = await saveUpdateSettings(patch)
      if (isMounted.current) {
        setSettings(next)
      }
    }
    void doSave()
  }, [])

  const busy = phase.kind === 'checking' || phase.kind === 'downloading'

  return (
    <div className="space-y-4 rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-base font-semibold">Updates</h2>
          <p className="text-sm opacity-80">
            Current version: <span className="font-mono">{currentVersion || 'unknown'}</span>
          </p>
        </div>
        {installSupport !== 'packageManager' && (
          <Button
            onClick={() => {
              runCheck()
            }}
            disabled={busy}
          >
            {phase.kind === 'checking' ? 'Checking…' : 'Check for Updates'}
          </Button>
        )}
      </div>

      {installSupport === 'packageManager' ? (
        <p aria-live="polite" className="text-sm opacity-80">
          This install is managed by your system package manager (deb/rpm/Flatpak). Update it there
          — in-app updates only apply to AppImage, macOS, and Windows builds.
        </p>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-4 text-sm">
            <label className="flex items-center gap-2">
              <span>Channel</span>
              <select
                value={settings?.channel ?? 'stable'}
                disabled={busy}
                onChange={(event) => {
                  const channel = event.target.value as UpdateChannel
                  updateSetting({ channel })
                  runCheck(channel)
                }}
                className="rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-2 py-1"
              >
                {UPDATE_CHANNELS.map((channel) => (
                  <option key={channel} value={channel}>
                    {channel}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={settings?.autoCheck ?? true}
                onChange={(event) => {
                  updateSetting({ autoCheck: event.target.checked })
                }}
              />
              <span>Check automatically</span>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={settings?.autoDownload ?? false}
                onChange={(event) => {
                  updateSetting({ autoDownload: event.target.checked })
                }}
              />
              <span>Download automatically</span>
            </label>
          </div>

          <div aria-live="polite" className="space-y-3 text-sm">
            {phase.kind === 'upToDate' && <p className="opacity-80">You’re up to date.</p>}

            {phase.kind === 'noReleases' && (
              <p className="opacity-80">
                No releases have been published on the {phase.channel} channel yet.
              </p>
            )}

            {phase.kind === 'available' && (
              <div className="space-y-3 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] p-3">
                <p className="font-medium">
                  Version {phase.result.latestVersion ?? '?'} is available
                  {phase.result.date ? ` (${phase.result.date.slice(0, 10)})` : ''}.
                </p>
                {phase.result.body ? (
                  <div className="max-h-48 overflow-y-auto whitespace-pre-wrap text-xs opacity-80">
                    {phase.result.body}
                  </div>
                ) : null}
                <div className="flex flex-wrap gap-2">
                  <Button
                    onClick={() => {
                      runDownload(phase.result)
                    }}
                  >
                    {formatUpdateSize(phase.result.downloadSize)
                      ? `Download (${formatUpdateSize(phase.result.downloadSize)})`
                      : 'Download'}
                  </Button>
                  <Button
                    variant="ghost"
                    onClick={() => {
                      handleSkip(phase.result)
                    }}
                  >
                    Skip this version
                  </Button>
                  <Button variant="ghost" onClick={handleRemindLater}>
                    Remind me later
                  </Button>
                </div>
              </div>
            )}

            {phase.kind === 'downloading' && <ProgressBar progress={phase.progress} />}

            {phase.kind === 'staged' && (
              <div className="space-y-2 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] p-3">
                <p className="font-medium">
                  {phase.latestVersion
                    ? `Version ${phase.latestVersion} is ready to install.`
                    : 'An update is ready to install.'}
                </p>
                <p className="text-xs opacity-80">
                  It installs automatically when you quit the app — or restart now.
                </p>
                <Button onClick={() => void restartToUpdate()}>Restart to Update</Button>
              </div>
            )}

            {phase.kind === 'error' && (
              <div className="space-y-2 rounded-md border border-red-500/40 bg-[var(--color-background)] p-3">
                <p className="font-medium text-red-500">{phase.info.title}</p>
                <p className="text-xs opacity-80">{phase.info.detail}</p>
                {phase.info.retryable && (
                  <Button variant="secondary" onClick={phase.retry}>
                    Retry
                  </Button>
                )}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  )
}
