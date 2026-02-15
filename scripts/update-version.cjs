#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');

const newVersion = process.argv[2];

if (!newVersion) {
  console.error('Usage: node scripts/update-version.cjs <version>');
  process.exit(1);
}

const semverPattern = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

if (!semverPattern.test(newVersion)) {
  console.error(`Invalid version: "${newVersion}". Expected semver (e.g. 0.2.0).`);
  process.exit(1);
}

const projectRoot = path.resolve(__dirname, '..');

const packageJsonPath = path.join(projectRoot, 'package.json');
const tauriConfigPath = path.join(projectRoot, 'src-tauri', 'tauri.conf.json');
const cargoTomlPath = path.join(projectRoot, 'src-tauri', 'Cargo.toml');

function updateJsonVersion(filePath) {
  const json = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  json.version = newVersion;
  fs.writeFileSync(filePath, `${JSON.stringify(json, null, 2)}\n`, 'utf8');
}

function updateCargoVersion(filePath) {
  const content = fs.readFileSync(filePath, 'utf8');
  const updated = content.replace(/^version\s*=\s*"[^"]+"/m, `version = "${newVersion}"`);

  if (updated === content) {
    throw new Error('Could not find package version field in src-tauri/Cargo.toml');
  }

  fs.writeFileSync(filePath, updated, 'utf8');
}

try {
  updateJsonVersion(packageJsonPath);
  updateJsonVersion(tauriConfigPath);
  updateCargoVersion(cargoTomlPath);

  console.log(`Version synchronized to ${newVersion}`);
  console.log('Updated:');
  console.log('  - package.json');
  console.log('  - src-tauri/tauri.conf.json');
  console.log('  - src-tauri/Cargo.toml');
} catch (error) {
  console.error('Failed to update version:', error instanceof Error ? error.message : String(error));
  process.exit(1);
}