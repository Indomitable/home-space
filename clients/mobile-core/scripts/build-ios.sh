#!/usr/bin/env bash
# Builds mobile-core for iOS: HomeSpaceCore.xcframework (device + simulator static libraries)
# and the generated Swift bindings, under clients/mobile/build/rust-core/ios.
#
# Usage: build-ios.sh [debug|mobile]   (default: mobile, the size-tuned profile)
# Xcode runs it as a pre-build step (see clients/mobile/iosApp/project.yml).
set -euo pipefail

profile="${1:-mobile}"
clients="$(cd "$(dirname "$0")/../.." && pwd)"
out="$clients/mobile/build/rust-core/ios"
targets=(aarch64-apple-ios aarch64-apple-ios-sim)

# Run from Xcode, the build settings in the environment (SDKROOT, deployment targets, ...)
# send cargo's C builds to the wrong SDK, so start over with a clean environment. Xcode also
# does not inherit the login shell's PATH, hence ~/.cargo/bin.
if [ -z "${HOMESPACE_CLEAN_ENV:-}" ]; then
  exec env -i HOME="$HOME" HOMESPACE_CLEAN_ENV=1 IPHONEOS_DEPLOYMENT_TARGET=17.0 \
    PATH="$HOME/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin" "$0" "$@"
fi

cd "$clients"
cargo_flags=(-p mobile-core --lib)
[ "$profile" != debug ] && cargo_flags+=(--profile "$profile")
for target in "${targets[@]}"; do
  cargo build "${cargo_flags[@]}" --target "$target"
done

# Library-mode bindgen reads the metadata from any one of the built libraries.
# Everything is written to $staging and swapped in at the end: Xcode checks that the
# xcframework exists before its pre-build scripts run, so a failed build must not remove it.
lib="target/${targets[0]}/$profile/libhomespace_core.a"
final="$out"
out="$(dirname "$final")/ios.staging"
rm -rf "$out" && mkdir -p "$out/swift" "$out/headers"
cargo run -q -p mobile-core --features bindgen --bin uniffi-bindgen -- \
  generate --library "$lib" --language swift --out-dir "$out/gen"
mv "$out/gen/"*.swift "$out/swift/"
mv "$out/gen/"*.h "$out/headers/"
# The C module the Swift bindings import; the xcframework wants it named module.modulemap.
mv "$out/gen/"*.modulemap "$out/headers/module.modulemap"
rm -rf "$out/gen"

args=()
for target in "${targets[@]}"; do
  args+=(-library "target/$target/$profile/libhomespace_core.a" -headers "$out/headers")
done
xcodebuild -create-xcframework "${args[@]}" -output "$out/HomeSpaceCore.xcframework" >/dev/null
rm -rf "$final" && mv "$out" "$final"
echo "Built $final/HomeSpaceCore.xcframework ($profile)"
