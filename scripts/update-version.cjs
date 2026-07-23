#!/usr/bin/env node

const fs = require('node:fs')
const path = require('node:path')

const newVersion = process.argv[2]

if (!newVersion) {
  process.exit(1)
}

const semverPattern = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/

if (!semverPattern.test(newVersion)) {
  process.exit(1)
}

const projectRoot = path.resolve(__dirname, '..')

const packageJsonPath = path.join(projectRoot, 'package.json')
const tauriConfigPath = path.join(projectRoot, 'src-tauri', 'tauri.conf.json')
const optionalTauriConfigs = [
  path.join(projectRoot, 'src-tauri', 'tauri.macos.conf.json'),
  path.join(projectRoot, 'src-tauri', 'tauri.windows.conf.json'),
  path.join(projectRoot, 'src-tauri', 'tauri.linux.conf.json'),
  path.join(projectRoot, 'src-tauri', 'tauri.android.conf.json'),
  path.join(projectRoot, 'src-tauri', 'tauri.ios.conf.json'),
]
const cargoTomlPath = path.join(projectRoot, 'src-tauri', 'Cargo.toml')

function updateJsonVersion(filePath) {
  const json = JSON.parse(fs.readFileSync(filePath, 'utf8'))
  json.version = newVersion
  fs.writeFileSync(filePath, `${JSON.stringify(json, null, 2)}\n`, 'utf8')
}

function updateCargoVersion(filePath) {
  const content = fs.readFileSync(filePath, 'utf8')
  const updated = content.replace(/^version\s*=\s*"[^"]+"/m, `version = "${newVersion}"`)

  if (updated === content) {
    throw new Error('Could not find package version field in src-tauri/Cargo.toml')
  }

  fs.writeFileSync(filePath, updated, 'utf8')
}

try {
  updateJsonVersion(packageJsonPath)
  updateJsonVersion(tauriConfigPath)
  const updatedOptionalConfigs = optionalTauriConfigs.filter((configPath) =>
    fs.existsSync(configPath),
  )
  for (const configPath of updatedOptionalConfigs) {
    updateJsonVersion(configPath)
  }
  updateCargoVersion(cargoTomlPath)
} catch (err) {
  process.stderr.write(
    `Failed to update version: ${err instanceof Error ? err.message : String(err)}\n`,
  )
  process.exit(1)
}
