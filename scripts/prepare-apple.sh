#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$#" -ne 2 ]; then
  echo "Usage: bash scripts/prepare-apple.sh TEAMID1234 com.yourname.routeloom"
  exit 1
fi
xcodebuild -version
if ! command -v flutter >/dev/null 2>&1; then
  echo "Install Flutter 3.50.0 and add flutter/bin to PATH, then run this script again."
  exit 1
fi
flutter --version
python3 scripts/prepare-apple.py "$1" "$2"
rustup toolchain install 1.100.0
rustup override set 1.100.0
bash scripts/build-native.sh macos
bash scripts/build-native.sh ios
node scripts/collect-licenses.mjs .local/dependency-licenses
node scripts/bundle-client-licenses.mjs
cd apps/client
flutter pub get
flutter gen-l10n
flutter build macos --debug --config-only
flutter build ios --debug --config-only --no-codesign
echo "Prepared both workspaces. Sign in to Xcode, select Runner and PacketTunnel signing, then Run on your Mac/iPhone."
open macos/Runner.xcworkspace
open ios/Runner.xcworkspace
