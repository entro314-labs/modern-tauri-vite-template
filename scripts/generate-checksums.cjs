#!/usr/bin/env node

const fs = require('node:fs')
const path = require('node:path')
const crypto = require('node:crypto')

const projectRoot = path.resolve(__dirname, '..')
const bundleDir = path.join(projectRoot, 'src-tauri', 'target', 'release', 'bundle')

const artifactSuffixes = [
  '.exe',
  '.msi',
  '.dmg',
  '.app.tar.gz',
  '.pkg',
  '.appimage',
  '.deb',
  '.rpm',
  '.apk',
  '.aab',
  '.ipa',
]

function isArtifact(fileName) {
  const lower = fileName.toLowerCase()
  return artifactSuffixes.some((suffix) => lower.endsWith(suffix))
}

function walkFiles(directory) {
  if (!fs.existsSync(directory)) {
    return []
  }

  const output = []
  const entries = fs.readdirSync(directory, { withFileTypes: true })

  for (const entry of entries) {
    const fullPath = path.join(directory, entry.name)

    if (entry.isDirectory()) {
      output.push(...walkFiles(fullPath))
      continue
    }

    if (!entry.isFile()) {
      continue
    }

    if (entry.name.endsWith('.sha256') || entry.name === 'CHECKSUMS.txt') {
      continue
    }

    if (isArtifact(entry.name)) {
      output.push(fullPath)
    }
  }

  return output
}

function sha256(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256')
    const stream = fs.createReadStream(filePath)

    stream.on('data', (chunk) => hash.update(chunk))
    stream.on('end', () => {
      resolve(hash.digest('hex'))
    })
    stream.on('error', reject)
  })
}

async function main() {
  const artifacts = walkFiles(bundleDir)

  if (artifacts.length === 0) {
    process.exit(1)
  }

  const pkgJson = JSON.parse(fs.readFileSync(path.join(projectRoot, 'package.json'), 'utf8'))
  const { version } = pkgJson
  const generatedAt = new Date().toISOString()
  const lines = ['# SHA256 Checksums', `# Generated: ${generatedAt}`, `# tauri-app v${version}`, '']

  for (const artifact of artifacts) {
    const hash = await sha256(artifact)
    const relative = path.relative(bundleDir, artifact)
    const checksumLine = `${hash}  ${relative}`

    lines.push(checksumLine)
    fs.writeFileSync(`${artifact}.sha256`, `${hash}  ${path.basename(artifact)}\n`, 'utf8')
  }

  fs.writeFileSync(path.join(bundleDir, 'CHECKSUMS.txt'), `${lines.join('\n')}\n`, 'utf8')
}

main().catch((err) => {
  process.stderr.write(
    `Failed to generate checksums: ${err instanceof Error ? err.message : String(err)}\n`,
  )
  process.exit(1)
})
