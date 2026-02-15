#!/usr/bin/env bash
set -euo pipefail

# Encode Android keystore for CI secret storage.

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <path-to-keystore.jks>"
  exit 1
fi

KEYSTORE_PATH="$1"

if [[ ! -f "$KEYSTORE_PATH" ]]; then
  echo "Keystore not found: $KEYSTORE_PATH"
  exit 1
fi

if ! command -v base64 >/dev/null 2>&1; then
  echo "base64 command not found"
  exit 1
fi

echo "Encoding: $KEYSTORE_PATH"
echo

if base64 --help 2>/dev/null | grep -q -- '--input'; then
  ENCODED="$(base64 --input "$KEYSTORE_PATH" | tr -d '\n')"
else
  ENCODED="$(base64 < "$KEYSTORE_PATH" | tr -d '\n')"
fi

echo "ANDROID_KEYSTORE_BASE64="
echo "$ENCODED"
echo
echo "Add these GitHub Actions secrets:"
echo "  - ANDROID_KEYSTORE_BASE64"
echo "  - ANDROID_KEYSTORE_PASSWORD"
echo "  - ANDROID_KEY_ALIAS"
echo "  - ANDROID_KEY_PASSWORD"