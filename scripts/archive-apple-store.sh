#!/bin/bash
# Prepare an owner-signed Xcode archive. This script never exports keys or uploads.
set -euo pipefail

usage() {
  echo 'Usage: bash scripts/archive-apple-store.sh ios|macos|all [--unsigned] [--no-open]'
  echo 'Signed archives require ROUTELOOM_PRIVACY_URL=https://your-approved-policy.example/path'
  echo '--unsigned is for CI structure checks only; it does not produce an installable release.'
}
if [ "$#" -eq 0 ]; then usage; exit 2; fi
case "$1" in
  -h|--help) usage; exit 0 ;;
  ios|macos|all) platform="$1"; shift ;;
  *) usage; exit 2 ;;
esac
unsigned=false
open_archive=true
while [ "$#" -gt 0 ]; do
  case "$1" in
    --unsigned) unsigned=true; open_archive=false ;;
    --no-open) open_archive=false ;;
    *) usage; exit 2 ;;
  esac
  shift
done
if [ "$(uname -s)" != Darwin ]; then
  echo 'Archive preparation requires macOS with Xcode.' >&2
  exit 1
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
if [ -f "${CARGO_HOME:-$HOME/.cargo}/env" ]; then
  source "${CARGO_HOME:-$HOME/.cargo}/env"
fi
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
if command -v brew >/dev/null 2>&1; then
  rustup_prefix="$(brew --prefix rustup 2>/dev/null || true)"
  if [ -n "$rustup_prefix" ]; then export PATH="$rustup_prefix/bin:$PATH"; fi
fi
for tool in xcodebuild xcrun flutter rustup cargo node python3; do
  command -v "$tool" >/dev/null || { echo "Required tool missing: $tool" >&2; exit 1; }
done
xcodebuild -version
privacy_url="${ROUTELOOM_PRIVACY_URL:-}"
# Keep arrays nonempty: the macOS system Bash 3.2 treats empty arrays as unset
# when nounset is enabled.
signing_args=(ONLY_ACTIVE_ARCH=NO STRIP_INSTALLED_PRODUCT=NO COPY_PHASE_STRIP=NO)
verify_args=(--privacy-url "$privacy_url")
if "$unsigned"; then
  signing_args+=(CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO)
  verify_args+=(--unsigned)
fi
python3 scripts/apple-store-export.py preflight --root "$root" "${verify_args[@]}"

mkdir -p "$root/.local"
output="$(mktemp -d "$root/.local/apple-store-$(date +%Y%m%d-%H%M%S)-XXXXXX")"
echo "Archive output: $output"
platforms=("$platform")
if [ "$platform" = all ]; then platforms=(macos ios); fi
for target in "${platforms[@]}"; do
  bash scripts/build-native.sh "$target"
done
node scripts/collect-licenses.mjs .local/dependency-licenses
node scripts/bundle-client-licenses.mjs
(
  cd apps/client
  flutter pub get
  flutter gen-l10n
)

for target in "${platforms[@]}"; do
  archive="$output/RouteLoom-$target.xcarchive"
  flutter_args=(--release --config-only --dart-define=ROUTELOOM_APPLE_STORE=true \
    "--dart-define=ROUTELOOM_PRIVACY_URL=$privacy_url")
  if [ "$target" = macos ]; then
    destination='generic/platform=macOS'
    architectures='arm64 x86_64'
    config="$root/apps/client/macos/Flutter/ephemeral/Flutter-Generated.xcconfig"
  else
    destination='generic/platform=iOS'
    architectures='arm64'
    config="$root/apps/client/ios/Flutter/Generated.xcconfig"
    flutter_args+=(--no-codesign)
  fi
  (
    cd apps/client
    flutter build "$target" "${flutter_args[@]}"
  )
  # Keep the project's existing team, bundle identifiers, capabilities and signing
  # configuration. Organizer performs final distribution signing and validation.
  xcodebuild -workspace "$root/apps/client/$target/Runner.xcworkspace" \
    -scheme Runner -configuration Release -destination "$destination" \
    -derivedDataPath "$output/DerivedData-$target" -archivePath "$archive" \
    "ARCHS=$architectures" "${signing_args[@]}" archive
  python3 scripts/apple-store-export.py verify --root "$root" \
    --platform "$target" --archive "$archive" --flutter-config "$config" \
    --report "$output/$target-archive-report.json" \
    "${verify_args[@]}"
  if "$open_archive"; then open -a Xcode "$archive"; fi
done

if "$unsigned"; then
  echo 'Unsigned archive structure checks passed. These archives are not customer downloads.'
else
  echo 'Archives prepared. In Xcode Organizer, Validate App, then review TestFlight & App Store distribution.'
  echo 'Nothing has been uploaded. Account, privacy, export compliance, review and device testing remain required.'
fi
echo "Output: $output"
