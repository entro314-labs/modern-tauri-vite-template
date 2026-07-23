#!/usr/bin/env node

const fs = require('node:fs')
const path = require('node:path')
const crypto = require('node:crypto')

function printHelp() {}

function readExpectedChecksum(checksumFile) {
  const content = fs.readFileSync(checksumFile, 'utf8').trim()
  if (!content) {
    throw new Error(`Checksum file is empty: ${checksumFile}`)
  }

  return content.split(/\s+/)[0].toLowerCase()
}

function calculateChecksum(filePath) {
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
  const args = process.argv.slice(2)

  if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
    printHelp()
    process.exit(0)
  }

  const filePath = path.resolve(args[0])
  const checksumFile = args[1] ? path.resolve(args[1]) : `${filePath}.sha256`

  if (!fs.existsSync(filePath)) {
    throw new Error(`File not found: ${filePath}`)
  }

  if (!fs.existsSync(checksumFile)) {
    throw new Error(`Checksum file not found: ${checksumFile}`)
  }

  const expected = readExpectedChecksum(checksumFile)
  const actual = await calculateChecksum(filePath)

  if (expected !== actual) {
    process.exit(1)
  }
}

main().catch((err) => {
  process.stderr.write(
    `Checksum verification failed: ${err instanceof Error ? err.message : String(err)}\n`,
  )
  process.exit(1)
})
