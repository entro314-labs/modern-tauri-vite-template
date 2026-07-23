#!/usr/bin/env node

const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')

const projectRoot = path.resolve(__dirname, '..')
const args = new Set(process.argv.slice(2))
const verbose = args.has('--verbose') || args.has('-v')

const errors = []
const warnings = []

function checkFileExists(relativePath, label) {
  const fullPath = path.join(projectRoot, relativePath)
  if (!fs.existsSync(fullPath)) {
    errors.push(`${label} is missing: ${relativePath}`)
  }
}

function checkPackageDependencies() {
  const packagePath = path.join(projectRoot, 'package.json')

  if (!fs.existsSync(packagePath)) {
    errors.push('package.json not found')
    return
  }

  const pkg = JSON.parse(fs.readFileSync(packagePath, 'utf8'))
  const deps = {
    ...pkg.dependencies,
    ...pkg.devDependencies,
  }

  if (!deps['@tauri-apps/cli']) {
    errors.push('@tauri-apps/cli is not installed in dependencies/devDependencies')
  }

  if (!deps['@tauri-apps/api']) {
    errors.push('@tauri-apps/api is not installed in dependencies/devDependencies')
  }
}

function checkPlatformHints() {
  const platform = os.platform()

  if (platform === 'linux') {
    warnings.push('Linux builds need distro-specific system libs (see Tauri prerequisites).')
  }

  if (platform === 'darwin') {
    warnings.push('macOS signing/notarization requires Apple certificates and notary credentials.')
  }

  if (platform === 'win32') {
    warnings.push('Windows signing requires signtool and valid signing certificate setup.')
  }
}

function main() {
  checkPackageDependencies()
  checkFileExists('src-tauri/tauri.conf.json', 'Tauri config')
  checkFileExists('src-tauri/Cargo.toml', 'Rust manifest')
  checkPlatformHints()

  const out = (line) => process.stdout.write(`${line}\n`)
  const errOut = (line) => process.stderr.write(`${line}\n`)

  if (verbose) {
    out(`Platform: ${os.platform()} ${os.arch()} (node ${process.version})`)
    out(`Checks: ${errors.length} error(s), ${warnings.length} warning(s)`)
  }

  if (warnings.length > 0) {
    for (const warning of warnings) {
      errOut(`WARN ${warning}`)
    }
  }

  if (errors.length > 0) {
    for (const error of errors) {
      errOut(`ERROR ${error}`)
    }
    process.exit(1)
  }
}

main()
