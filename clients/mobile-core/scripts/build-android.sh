#!/usr/bin/env bash
# Builds mobile-core for Android. Gradle runs it (see clients/mobile/androidApp/build.gradle.kts).
#
#   build-android.sh bindings <out-dir>          Kotlin bindings, generated from a host build
#                                                (no NDK needed).
#   build-android.sh jni <out-dir> [debug|mobile]  libhomespace_core.so per ABI, laid out as
#                                                jniLibs (<out-dir>/<abi>/), via cargo-ndk.
set -euo pipefail

mode="$1"
out="$2"
profile="${3:-mobile}"
clients="$(cd "$(dirname "$0")/../.." && pwd)"
# Android Studio does not inherit the login shell's PATH.
export PATH="$HOME/.cargo/bin:$PATH"
cd "$clients"

case "$mode" in
  bindings)
    cargo build -p mobile-core --lib
    case "$(uname -s)" in
      Darwin) lib=target/debug/libhomespace_core.dylib ;;
      *) lib=target/debug/libhomespace_core.so ;;
    esac
    rm -rf "$out"
    cargo run -q -p mobile-core --features bindgen --bin uniffi-bindgen -- \
      generate --library "$lib" --language kotlin --no-format --out-dir "$out"
    ;;
  jni)
    if [ -z "${ANDROID_NDK_HOME:-}" ] && ! ls -d "${ANDROID_HOME:-/nonexistent}"/ndk/*/ >/dev/null 2>&1; then
      echo "error: no Android NDK under ${ANDROID_HOME:-\$ANDROID_HOME}/ndk." >&2
      echo "Install it: sdkmanager \"ndk;30.0.16248370\" (or Android Studio > SDK Manager > SDK Tools > NDK)." >&2
      exit 1
    fi
    # Empty for debug; the ${flags[@]+...} form below keeps macOS's bash 3.2 happy under set -u.
    flags=()
    [ "$profile" != debug ] && flags+=(--profile "$profile")
    rm -rf "$out"
    # arm64 for devices, x86_64 for the emulator on Intel hosts; -P matches minSdk.
    cargo ndk -t arm64-v8a -t x86_64 -P 26 -o "$out" build -p mobile-core --lib ${flags[@]+"${flags[@]}"}
    ;;
  *)
    echo "usage: $0 bindings|jni <out-dir> [debug|mobile]" >&2
    exit 2
    ;;
esac
