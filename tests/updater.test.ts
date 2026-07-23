import {
  DEFAULT_UPDATE_SETTINGS,
  deriveUpdateChannel,
  describeUpdateError,
  formatUpdateSize,
  NoReleasesOnChannelError,
} from '../src/lib/updater'

describe(deriveUpdateChannel, () => {
  it('derives the channel from prerelease suffixes', () => {
    expect(deriveUpdateChannel('1.0.0')).toBe('stable')
    expect(deriveUpdateChannel('0.2.0-alpha.4')).toBe('alpha')
    expect(deriveUpdateChannel('2.1.0-beta.1')).toBe('beta')
  })
})

describe(formatUpdateSize, () => {
  it('formats megabytes and gigabytes', () => {
    expect(formatUpdateSize(42.3 * 1024 * 1024)).toBe('42.3 MB')
    expect(formatUpdateSize(2.5 * 1024 * 1024 * 1024)).toBe('2.50 GB')
  })

  it('returns undefined for missing or invalid sizes', () => {
    expect(formatUpdateSize(null)).toBeUndefined()
    expect(formatUpdateSize(undefined)).toBeUndefined()
    expect(formatUpdateSize(-1)).toBeUndefined()
    expect(formatUpdateSize(Number.NaN)).toBeUndefined()
  })
})

describe(describeUpdateError, () => {
  it('classifies signature failures as non-retryable', () => {
    const info = describeUpdateError(new Error('invalid minisign signature'))
    expect(info.retryable).toBe(false)
    expect(info.title).toContain("couldn't be verified")
  })

  it('classifies network failures as retryable', () => {
    const info = describeUpdateError(new Error('error sending request for url'))
    expect(info.retryable).toBe(true)
    expect(info.title).toContain('update server')
  })

  it('classifies missing releases', () => {
    const info = describeUpdateError('Could not fetch a valid release JSON from the remote')
    expect(info.title).toBe('No update found on this channel')
  })

  it('falls back to the raw message', () => {
    const info = describeUpdateError(new Error('something exotic'))
    expect(info).toEqual({ title: 'Update failed', detail: 'something exotic', retryable: true })
  })
})

describe(NoReleasesOnChannelError, () => {
  it('carries the channel it was raised for', () => {
    const error = new NoReleasesOnChannelError('beta')
    expect(error.channel).toBe('beta')
    expect(error.message).toContain('beta')
  })
})

describe('default update settings', () => {
  it('checks daily and never auto-downloads by default', () => {
    expect(DEFAULT_UPDATE_SETTINGS).toMatchObject({
      channel: 'stable',
      autoCheck: true,
      autoDownload: false,
      checkIntervalHours: 24,
    })
  })
})
