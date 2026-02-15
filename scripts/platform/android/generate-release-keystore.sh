#!/usr/bin/env bash
set -euo pipefail

# Generate an Android release keystore for signing APK/AAB builds.

KEYSTORE_DIR="${KEYSTORE_DIR:-src-tauri/gen/android/keystore}"
KEYSTORE_NAME="${KEYSTORE_NAME:-release-key.jks}"
KEYSTORE_PATH="$KEYSTORE_DIR/$KEYSTORE_NAME"
KEY_ALIAS="${ANDROID_KEY_ALIAS:-tauri-app}"

if ! command -v keytool >/dev/null 2>&1; then
  echo "keytool not found. Install a JDK first."
  exit 1
fi

mkdir -p "$KEYSTORE_DIR"

if [[ -f "$KEYSTORE_PATH" ]]; then
  read -r -p "Keystore exists at $KEYSTORE_PATH. Overwrite? (y/N): " answer
  if [[ ! "$answer" =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
  fi
  rm -f "$KEYSTORE_PATH"
fi

read -r -s -p "Keystore password (min 6 chars): " KEYSTORE_PASSWORD
echo
if [[ ${#KEYSTORE_PASSWORD} -lt 6 ]]; then
  echo "Password must be at least 6 characters."
  exit 1
fi

read -r -s -p "Key password (Enter to reuse keystore password): " KEY_PASSWORD
echo
if [[ -z "$KEY_PASSWORD" ]]; then
  KEY_PASSWORD="$KEYSTORE_PASSWORD"
fi

echo "Certificate subject fields:"
read -r -p "CN (name/org): " CN
read -r -p "OU (optional): " OU
read -r -p "O  (optional): " O
read -r -p "L  (optional): " L
read -r -p "ST (optional): " ST
read -r -p "C  (2-letter country code): " C

DN="CN=$CN"
[[ -n "$OU" ]] && DN="$DN, OU=$OU"
[[ -n "$O" ]] && DN="$DN, O=$O"
[[ -n "$L" ]] && DN="$DN, L=$L"
[[ -n "$ST" ]] && DN="$DN, ST=$ST"
DN="$DN, C=$C"

keytool -genkeypair -v \
  -keystore "$KEYSTORE_PATH" \
  -alias "$KEY_ALIAS" \
  -keyalg RSA \
  -keysize 2048 \
  -validity 10000 \
  -storepass "$KEYSTORE_PASSWORD" \
  -keypass "$KEY_PASSWORD" \
  -dname "$DN"

echo
echo "Keystore generated: $KEYSTORE_PATH"
echo "Alias: $KEY_ALIAS"
echo
echo "Export these for CI/local builds:"
echo "  export ANDROID_KEYSTORE_PATH=$KEYSTORE_PATH"
echo "  export ANDROID_KEYSTORE_PASSWORD='<your-keystore-password>'"
echo "  export ANDROID_KEY_ALIAS=$KEY_ALIAS"
echo "  export ANDROID_KEY_PASSWORD='<your-key-password>'"