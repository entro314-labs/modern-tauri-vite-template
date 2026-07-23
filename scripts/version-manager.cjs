#!/usr/bin/env node

const fs = require('node:fs')
const path = require('node:path')

const projectRoot = path.resolve(__dirname, '..')
const buildCounterPath = path.join(projectRoot, '.build-counter.json')
const updateVersionScript = path.join(projectRoot, 'scripts', 'update-version.cjs')

function parseJson(filePath, fallback) {
  if (!fs.existsSync(filePath)) {
    return fallback
  }

  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'))
  } catch {
    return fallback
  }
}

function getCounter() {
  const data = parseJson(buildCounterPath, null)
  if (!data || typeof data.buildNumber !== 'number') {
    return { buildNumber: 1, lastBuildDate: new Date().toISOString() }
  }

  return {
    buildNumber: data.buildNumber,
    lastBuildDate:
      typeof data.lastBuildDate === 'string' ? data.lastBuildDate : new Date().toISOString(),
  }
}

function saveCounter(buildNumber) {
  fs.writeFileSync(
    buildCounterPath,
    `${JSON.stringify({ buildNumber, lastBuildDate: new Date().toISOString() }, null, 2)}\n`,
    'utf8',
  )
}

function generateVersion(buildNumber, format) {
  switch (format) {
    case 'build':
      return `0.0.1-build.${buildNumber}`
    case 'patch':
      return `0.0.${buildNumber}`
    case 'semantic':
    default:
      return `0.1.${buildNumber}`
  }
}

function runUpdateVersion(newVersion) {
  const script = fs.readFileSync(updateVersionScript, 'utf8')
  if (!script.includes('update-version.cjs')) {
    // No-op: sanity guard placeholder so the script file is at least loaded.
  }

  const { spawnSync } = require('node:child_process')
  const result = spawnSync(process.execPath, [updateVersionScript, newVersion], {
    cwd: projectRoot,
    stdio: 'inherit',
  })

  if (result.status !== 0) {
    process.exit(result.status ?? 1)
  }
}

function printHelp() {}

function main() {
  const [, , command = 'increment', ...flags] = process.argv

  if (command === 'help' || command === '--help' || command === '-h') {
    printHelp()
    return
  }

  const counter = getCounter()

  if (command === 'current' || command === 'info') {
    const version = generateVersion(counter.buildNumber, 'semantic')
    process.stdout.write(`Current version: ${version} (build ${counter.buildNumber})\n`)
    return
  }

  if (command !== 'increment' && command !== 'inc') {
    printHelp()
    process.exit(1)
  }

  const dryRun = flags.includes('--dry-run')
  const format = flags.includes('--build')
    ? 'build'
    : flags.includes('--patch')
      ? 'patch'
      : 'semantic'

  const nextBuildNumber = counter.buildNumber + 1
  const nextVersion = generateVersion(nextBuildNumber, format)

  if (dryRun) {
    return
  }

  runUpdateVersion(nextVersion)
  saveCounter(nextBuildNumber)
}

main()
