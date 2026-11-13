#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
case "$1" in
 android)
    rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
    command -v cargo-ndk >/dev/null || cargo install cargo-ndk --locked
    cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -p 29 -o apps/client/android/app/src/main/jniLibs build --release -p routeloom-engine --locked
    ;;
 macos)
    rustup target add x86_64-apple-darwin aarch64-apple-darwin
    for target in x86_64-apple-darwin aarch64-apple-darwin; do cargo build --release -p routeloom-engine --target "$target" --locked; done
    mkdir -p apps/client/native/macosx
    lipo -create target/{x86_64,aarch64}-apple-darwin/release/librouteloom_engine.a -output apps/client/native/macosx/librouteloom_engine.a
    ;;
 ios)
    rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
    for target in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do cargo build --release -p routeloom-engine --target "$target" --locked; done
    mkdir -p apps/client/native/{iphoneos,iphonesimulator}
    cp target/aarch64-apple-ios/release/librouteloom_engine.a apps/client/native/iphoneos/
    lipo -create target/{aarch64-apple-ios-sim,x86_64-apple-ios}/release/librouteloom_engine.a -output apps/client/native/iphonesimulator/librouteloom_engine.a
    ;;
 *) echo "Usage: bash scripts/build-native.sh android|macos|ios"; exit 2 ;;
esac
