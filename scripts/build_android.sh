#!/usr/bin/env bash
set -euo pipefail

# Build and Deploy script for AudioMixingConsole Android Client (Oboe + AAudio / 3.5mm P2)
# Supported: Android 12+ (API level 31+) on aarch64-linux-android architecture

TARGET_ARCH="aarch64-linux-android"
MIN_SDK_VERSION="31" # Android 12

echo "=== AudioMixingConsole Android Build & Deploy ==="

# 1. Check Cargo NDK
if ! command -v cargo-ndk &> /dev/null; then
    echo "[INFO] cargo-ndk not found. Installing cargo-ndk..."
    cargo install cargo-ndk
fi

# 2. Check Rust target
if ! rustup target list | grep -q "${TARGET_ARCH} (installed)"; then
    echo "[INFO] Adding Rust target ${TARGET_ARCH}..."
    rustup target add "${TARGET_ARCH}"
fi

# 3. Check ANDROID_NDK_HOME
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -z "${NDK_HOME:-}" ]; then
    echo "[WARN] Neither ANDROID_NDK_HOME nor NDK_HOME is set."
    echo "[INFO] Searching standard Android SDK locations..."
    if [ -d "$HOME/Library/Android/sdk/ndk" ]; then
        NDK_DIR=$(find "$HOME/Library/Android/sdk/ndk" -maxdepth 1 -mindepth 1 | sort -V | tail -n 1)
        export ANDROID_NDK_HOME="$NDK_DIR"
        echo "[INFO] Found NDK at: $ANDROID_NDK_HOME"
    fi
fi

# 4. Build Client with Release Profile
echo "[INFO] Compiling client crate for ${TARGET_ARCH} (Android 12+)..."
cargo ndk -t arm64-v8a -p "${MIN_SDK_VERSION}" -- build --package client --release

echo "[INFO] Build completed successfully."
echo "[INFO] Binary output: target/${TARGET_ARCH}/release/client"

# 5. Optional ADB Push & Execution
if command -v adb &> /dev/null; then
    DEVICES=$(adb devices | grep -v "List of devices" | grep "device$" || true)
    if [ -n "$DEVICES" ]; then
        echo "[INFO] Connected Android device detected via ADB."
        read -p "Deploy binary to /data/local/tmp/client and execute? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            adb push "target/${TARGET_ARCH}/release/client" /data/local/tmp/client
            adb shell chmod +x /data/local/tmp/client
            echo "[INFO] Starting client binary on target device (P2 3.5mm routing)..."
            adb shell /data/local/tmp/client
        fi
    fi
fi
