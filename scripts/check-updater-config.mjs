#!/usr/bin/env node
/**
 * Guard against the placeholder-pubkey trap: a build shipped with the scaffold's placeholder
 * updater config can NEVER verify or install an update (and `createUpdaterArtifacts` would sign
 * artifacts against a key you don't own).
 *
 * Verdicts:
 *  - PASS  — updater fully disabled (no `plugins.updater` and `createUpdaterArtifacts` not true).
 *  - PASS  — updater configured with a real pubkey and non-placeholder endpoints.
 *  - WARN  — placeholders present but the project is still the pristine template
 *            (package.json name === "tauri-app"). The moment you rename the app, this becomes:
 *  - FAIL  — updater active with a placeholder pubkey or OWNER/RELEASES-REPO endpoints.
 *
 * Set CHECK_UPDATER_STRICT=1 to turn the WARN into a FAIL (recommended for release workflows).
 * See docs/UPDATER.md for the fill-in checklist and how to consciously disable the updater.
 */

import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')

const out = (line) => process.stdout.write(`${line}\n`)
const errOut = (line) => process.stderr.write(`${line}\n`)

/** @param {string} relativePath */
const readJson = (relativePath) => {
  const absolutePath = join(root, relativePath)
  try {
    return JSON.parse(readFileSync(absolutePath, 'utf8'))
  } catch (err) {
    errOut(`check-updater-config: cannot read ${relativePath}: ${err.message}`)
    process.exit(1)
  }
}

const tauriConf = readJson('src-tauri/tauri.conf.json')
const packageJson = readJson('package.json')

const updater = tauriConf.plugins?.updater
const createsArtifacts = tauriConf.bundle?.createUpdaterArtifacts === true

if (updater === undefined && !createsArtifacts) {
  out('check-updater-config: updater disabled (no plugins.updater, no updater artifacts) — OK')
  process.exit(0)
}

const problems = []

const pubkey = typeof updater?.pubkey === 'string' ? updater.pubkey : ''
const looksLikeMinisignKey = pubkey.startsWith('dW50cnVzdGVkIGNvbW1lbnQ6') // base64 "untrusted comment:"
if (updater === undefined) {
  problems.push(
    'bundle.createUpdaterArtifacts is true but plugins.updater is missing — artifacts would be built with no updater to consume them.',
  )
} else {
  if (!pubkey || pubkey.includes('REPLACE_WITH') || !looksLikeMinisignKey) {
    problems.push(
      `plugins.updater.pubkey is ${pubkey ? `"${pubkey.slice(0, 40)}…"` : 'empty'} — not a real minisign public key.`,
    )
  }
  const endpoints = Array.isArray(updater.endpoints) ? updater.endpoints : []
  if (endpoints.length === 0) {
    problems.push('plugins.updater.endpoints is empty.')
  }
  for (const endpoint of endpoints) {
    if (String(endpoint).includes('OWNER/RELEASES-REPO')) {
      problems.push(`endpoint still points at the placeholder repository: ${endpoint}`)
    }
  }
}

// The runtime endpoints live in Rust (UPDATE_MANIFEST_BASE overrides the config endpoint per
// channel) — keep them in sync with tauri.conf.json.
try {
  const updaterRs = readFileSync(join(root, 'src-tauri/src/updater.rs'), 'utf8')
  if (updaterRs.includes('OWNER/RELEASES-REPO')) {
    problems.push(
      'src-tauri/src/updater.rs UPDATE_MANIFEST_BASE still has the OWNER/RELEASES-REPO placeholder.',
    )
  }
} catch {
  // Module removed — the conf checks above still cover the plugin config.
}

if (problems.length === 0) {
  out('check-updater-config: updater configuration looks real — OK')
  process.exit(0)
}

const pristineTemplate =
  packageJson.name === 'tauri-app' && process.env.CHECK_UPDATER_STRICT !== '1'

const header = pristineTemplate
  ? 'check-updater-config: WARNING — updater config is still the template placeholder:'
  : 'check-updater-config: FAILED — updater is active but not actually configured:'
const log = pristineTemplate ? out : errOut

log(header)
for (const problem of problems) {
  log(`  - ${problem}`)
}
log('')
log('Fix: follow docs/UPDATER.md (generate a minisign keypair, set plugins.updater.pubkey,')
log('point endpoints + UPDATE_MANIFEST_BASE in src-tauri/src/updater.rs at your releases repo),')
log('or consciously disable the updater (remove plugins.updater and set')
log('bundle.createUpdaterArtifacts to false).')

if (pristineTemplate) {
  log('')
  log('Allowed for now because package.json name is still "tauri-app" (pristine template).')
  log('Renaming the package — or setting CHECK_UPDATER_STRICT=1 — turns this into a failure.')
  process.exit(0)
}

process.exit(1)
