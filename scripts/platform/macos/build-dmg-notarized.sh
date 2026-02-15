#!/usr/bin/env bash
set -euo pipefail

# Build/prepare a signed + notarized + stapled DMG from an existing .app bundle.
#
# Required env:
#   APPLE_SIGN_ID
#   APPLE_NOTARY_KEY_ID
#   APPLE_NOTARY_ISSUER_ID
#   APPLE_NOTARY_KEY_PATH
#
# Usage:
#   bash scripts/platform/macos/build-dmg-notarized.sh \
#     --app "src-tauri/target/release/bundle/macos/tauri-app.app" \
#     --out "build/tauri-app-v0.1.0-macos-aarch64-signed.dmg"
#
# Optional:
#   --volname "tauri-app"
#   --eula "./EULA.txt"
#   --skip-applescript

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUPPORT_DIR="$SCRIPT_DIR/support"
APPLESCPT_TEMPLATE="$SUPPORT_DIR/template.applescript"
EULA_TEMPLATE_XML="$SUPPORT_DIR/eula-resources-template.xml"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script is macOS-only."
  exit 1
fi

APP_PATH=""
OUT_DMG=""
VOLNAME=""
EULA_FILE=""
SKIP_APPLESCRIPT=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app) APP_PATH="$2"; shift 2 ;;
    --out) OUT_DMG="$2"; shift 2 ;;
    --volname) VOLNAME="$2"; shift 2 ;;
    --eula) EULA_FILE="$2"; shift 2 ;;
    --skip-applescript) SKIP_APPLESCRIPT=1; shift ;;
    -h|--help)
      echo "Usage: $0 --app /path/to/App.app --out /path/to/output.dmg [--volname NAME] [--eula /path/to/EULA.txt] [--skip-applescript]"
      exit 0
      ;;
    *)
      echo "Unknown argument: $1"
      exit 1
      ;;
  esac
done

if [[ -z "$APP_PATH" || -z "$OUT_DMG" ]]; then
  echo "--app and --out are required"
  exit 1
fi

APP_PATH="$(cd "$(dirname "$APP_PATH")" && pwd)/$(basename "$APP_PATH")"
OUT_DMG="$(cd "$(dirname "$OUT_DMG")" && pwd)/$(basename "$OUT_DMG")"

if [[ -z "$VOLNAME" ]]; then
  VOLNAME="$(basename "$APP_PATH" .app)"
fi

if [[ ! -d "$APP_PATH" || "${APP_PATH: -4}" != ".app" ]]; then
  echo "--app must point to an existing .app bundle"
  exit 1
fi

if [[ "${OUT_DMG: -4}" != ".dmg" ]]; then
  echo "--out must be a .dmg path"
  exit 1
fi

if [[ -n "$EULA_FILE" && ! -f "$EULA_FILE" ]]; then
  echo "EULA file not found: $EULA_FILE"
  exit 1
fi

: "${APPLE_SIGN_ID:?Missing APPLE_SIGN_ID}"
: "${APPLE_NOTARY_KEY_ID:?Missing APPLE_NOTARY_KEY_ID}"
: "${APPLE_NOTARY_ISSUER_ID:?Missing APPLE_NOTARY_ISSUER_ID}"
: "${APPLE_NOTARY_KEY_PATH:?Missing APPLE_NOTARY_KEY_PATH}"

command -v xcrun >/dev/null 2>&1 || { echo "xcrun not found"; exit 1; }
xcrun notarytool --help >/dev/null 2>&1 || { echo "notarytool unavailable (need Xcode 13+)"; exit 1; }

WORK_ROOT="$(mktemp -d -t tauri-app-dmg.XXXXXX)"
STAGE_DIR="$WORK_ROOT/stage"
ZIP_PATH="$WORK_ROOT/$(basename "$APP_PATH" .app)-notary.zip"
RW_DMG="$WORK_ROOT/rw.dmg"
MOUNT_DIR=""
DEV_NAME=""

cleanup() {
  set +e
  if [[ -n "$DEV_NAME" ]]; then
    hdiutil detach "$DEV_NAME" >/dev/null 2>&1 || true
  fi
  rm -rf "$WORK_ROOT" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "==> Signing app"
codesign --force --deep --timestamp --options runtime --sign "$APPLE_SIGN_ID" "$APP_PATH"
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

echo "==> Notarizing app"
ditto -c -k --keepParent "$APP_PATH" "$ZIP_PATH"
xcrun notarytool submit "$ZIP_PATH" \
  --key "$APPLE_NOTARY_KEY_PATH" \
  --key-id "$APPLE_NOTARY_KEY_ID" \
  --issuer "$APPLE_NOTARY_ISSUER_ID" \
  --wait

echo "==> Stapling app"
xcrun stapler staple "$APP_PATH"
xcrun stapler validate "$APP_PATH"

echo "==> Staging app"
mkdir -p "$STAGE_DIR"
cp -R "$APP_PATH" "$STAGE_DIR/"
xattr -cr "$STAGE_DIR/$(basename "$APP_PATH")" || true
ln -s /Applications "$STAGE_DIR/Applications"

echo "==> Creating RW DMG"
hdiutil create -volname "$VOLNAME" -srcfolder "$STAGE_DIR" -ov -format UDRW "$RW_DMG" >/dev/null

echo "==> Mounting RW DMG"
PLIST="$(hdiutil attach -readwrite -noverify -noautoopen -nobrowse -plist "$RW_DMG")"
MOUNT_INFO="$(echo "$PLIST" | plutil -extract system-entities xml1 -o - - | xpath -q -e '//dict[key="mount-point"]')"
DEV_NAME="$(echo "$MOUNT_INFO" | xpath -q -e '//key[.="dev-entry"]/following-sibling::string[1]/text()')"
MOUNT_DIR="$(echo "$MOUNT_INFO" | xpath -q -e '//key[.="mount-point"]/following-sibling::string[1]/text()')"

if [[ -z "$DEV_NAME" || -z "$MOUNT_DIR" ]]; then
  echo "Failed to mount RW DMG"
  exit 1
fi

if [[ "$SKIP_APPLESCRIPT" -eq 0 ]]; then
  TMP_SCPT="$(mktemp -t tauri-app-dmg.scpt.XXXXXX)"
  POSITION_CLAUSE=$(cat <<EOF
set position of item "$(basename "$APP_PATH")" to {140, 170}
set position of item "Applications" to {420, 170}
EOF
)

  cat "$APPLESCPT_TEMPLATE" \
    | sed -e "s/WINX/200/g" -e "s/WINY/120/g" -e "s/WINW/560/g" -e "s/WINH/360/g" \
          -e "s/ICON_SIZE/128/g" -e "s/TEXT_SIZE/14/g" \
    | perl -0777 -pe "s/POSITION_CLAUSE/$POSITION_CLAUSE/s" \
    | perl -0777 -pe "s/BACKGROUND_CLAUSE//s" \
    | perl -0777 -pe "s/REPOSITION_HIDDEN_FILES_CLAUSE//s" \
    | perl -0777 -pe "s/HIDING_CLAUSE//s" \
    | perl -0777 -pe "s/APPLICATION_CLAUSE//s" \
    | perl -0777 -pe "s/QL_CLAUSE//s" \
    > "$TMP_SCPT"

  sleep 2
  /usr/bin/osascript "$TMP_SCPT" "$(basename "$MOUNT_DIR")" || echo "AppleScript layout step failed; continuing"
  rm -f "$TMP_SCPT"
fi

rm -rf "$MOUNT_DIR/.fseventsd" >/dev/null 2>&1 || true

echo "==> Detaching RW DMG"
hdiutil detach "$DEV_NAME" >/dev/null
DEV_NAME=""
MOUNT_DIR=""

echo "==> Converting to compressed DMG"
mkdir -p "$(dirname "$OUT_DMG")"
hdiutil convert "$RW_DMG" -format UDZO -imagekey zlib-level=9 -ov -o "$OUT_DMG" >/dev/null

if [[ -n "$EULA_FILE" ]]; then
  [[ -f "$EULA_TEMPLATE_XML" ]] || { echo "Missing EULA template"; exit 1; }
  EULA_FORMAT="TEXT"
  if [[ "$(file -b "$EULA_FILE")" == Rich\ Text\ Format* ]]; then
    EULA_FORMAT="RTF "
  fi

  EULA_DATA="$(openssl base64 -in "$EULA_FILE" | tr -d '\n' | awk '{gsub(/.{52}/,"&\\n")}1' | sed 's/^/\t\t\t/')"
  TMP_EULA_XML="$(mktemp -t tauri-app-eula.XXXXXX).xml"
  perl -0777 -pe "s/\\\$\\\{EULA_FORMAT\\\}/$EULA_FORMAT/g; s/\\\$\\\{EULA_DATA\\\}/$EULA_DATA/g" "$EULA_TEMPLATE_XML" > "$TMP_EULA_XML"
  hdiutil udifrez -xml "$TMP_EULA_XML" '' -quiet "$OUT_DMG"
  rm -f "$TMP_EULA_XML"
fi

echo "==> Signing DMG"
codesign --force --timestamp --sign "$APPLE_SIGN_ID" "$OUT_DMG"
codesign --verify --verbose=2 "$OUT_DMG"

echo "==> Notarizing DMG"
xcrun notarytool submit "$OUT_DMG" \
  --key "$APPLE_NOTARY_KEY_PATH" \
  --key-id "$APPLE_NOTARY_KEY_ID" \
  --issuer "$APPLE_NOTARY_ISSUER_ID" \
  --wait

echo "==> Stapling DMG"
xcrun stapler staple "$OUT_DMG"
xcrun stapler validate "$OUT_DMG"

echo "Done: $OUT_DMG"