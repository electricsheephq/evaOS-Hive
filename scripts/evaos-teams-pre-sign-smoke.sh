#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <expected-full-git-sha> <absolute-evidence-json-path>" >&2
  exit 2
fi

EXPECTED_SHA=$1
EVIDENCE_PATH=$2
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "${SCRIPT_DIR}/.." && pwd)

if ! [[ "$EXPECTED_SHA" =~ ^[0-9a-f]{40}$ ]]; then
  echo "Expected source SHA must be 40 lowercase hexadecimal characters" >&2
  exit 2
fi
if [[ "$EVIDENCE_PATH" != /* ]]; then
  echo "Evidence path must be absolute" >&2
  exit 2
fi
case "$EVIDENCE_PATH" in
  "$REPO_ROOT"/*)
    echo "Evidence must be written outside the source worktree" >&2
    exit 2
    ;;
esac

cd "$REPO_ROOT"
if [[ "$(git rev-parse HEAD)" != "$EXPECTED_SHA" ]]; then
  echo "Checked-out source does not match expected SHA" >&2
  exit 1
fi
if [[ -n "$(git status --porcelain)" ]]; then
  echo "Pre-sign source worktree must be clean" >&2
  exit 1
fi
if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "Pre-sign smoke requires a clean Apple Silicon macOS runner" >&2
  exit 1
fi
if [[ "${EVAOS_TEAMS_PRE_SIGN_ISOLATED:-}" != "1" ]]; then
  echo "Set EVAOS_TEAMS_PRE_SIGN_ISOLATED=1 only on an isolated runner" >&2
  exit 1
fi

for name in \
  BUZZ_UPDATER_PUBLIC_KEY \
  BUZZ_UPDATER_ENDPOINT \
  APPLE_SIGNING_IDENTITY \
  APPLE_CERTIFICATE \
  APPLE_CERTIFICATE_PASSWORD \
  APPLE_API_ISSUER \
  APPLE_API_KEY \
  APPLE_API_KEY_PATH \
  OSX_CODESIGN_ROLE \
  CODESIGN_S3_BUCKET \
  CSC_LINK \
  CSC_KEY_PASSWORD \
  TAURI_SIGNING_PRIVATE_KEY \
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD
do
  if [[ -n "${!name:-}" ]]; then
    echo "$name must not be present in the credential-free pre-sign lane" >&2
    exit 1
  fi
done

# The activation path is resolved from the checked-out repository at runtime.
# shellcheck disable=SC1091
source "$REPO_ROOT/bin/activate-hermit"

CLEAN_ENV=(
  "HOME=$HOME"
  "PATH=$PATH"
  "TMPDIR=${TMPDIR:-/tmp}"
  "LANG=${LANG:-C}"
  "LC_ALL=${LC_ALL:-}"
  "CARGO_HOME=$CARGO_HOME"
  "HERMIT_ENV=$HERMIT_ENV"
)
if [[ "${CI:-}" == "true" ]]; then
  CLEAN_ENV+=(
    "CI=true"
    "TAURI_BUNDLER_DMG_IGNORE_CI=true"
  )
fi

TARGET=aarch64-apple-darwin
APP_PATH="$REPO_ROOT/desktop/src-tauri/target/$TARGET/release/bundle/macos/Hive.app"
DMG_DIR="$REPO_ROOT/desktop/src-tauri/target/$TARGET/release/bundle/dmg"
DEFAULT_DMG_PATH="$DMG_DIR/Hive_0.4.26-es.1_aarch64.dmg"
DMG_PATH="$DMG_DIR/Hive-0.4.26-es.1-arm64.dmg"

env -i "${CLEAN_ENV[@]}" cargo build \
  --release \
  --target "$TARGET" \
  -p buzz-acp \
  -p buzz-agent \
  -p buzz-dev-mcp \
  -p git-credential-nostr \
  -p buzz-cli
env -i "${CLEAN_ENV[@]}" \
  "$REPO_ROOT/scripts/bundle-sidecars.sh" "$TARGET"

# These exact ignored bundle outputs are rebuilt below. Removing them prevents
# a prior smoke from being mistaken for current-head package evidence.
rm -rf -- "$APP_PATH"
rm -f -- "$DEFAULT_DMG_PATH" "$DMG_PATH"

env -i "${CLEAN_ENV[@]}" \
  pnpm --dir desktop run tauri:build:evaos-teams --no-sign

env -i "${CLEAN_ENV[@]}" node \
  "$REPO_ROOT/scripts/verify-evaos-teams-app-bundle.mjs" \
  --expected-sha "$EXPECTED_SHA" \
  --app "$APP_PATH" \
  --dmg "$DMG_PATH" \
  --evidence "$EVIDENCE_PATH"
