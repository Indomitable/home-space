# Mobile clients

Native UI on each platform, with the code that talks to the server in Rust:

- `../mobile-core/`: the Rust client for mobile. It wraps `api-client` and `client-core` (the
  code the desktop and web clients run) and exports them with
  [UniFFI](https://mozilla.github.io/uniffi-rs/). Rust `async fn` becomes a Kotlin `suspend fun`
  and a Swift `async throws` func, and Rust errors become typed exceptions (`CoreException` in
  Kotlin, `CoreError` in Swift). The bindings are generated on every build and are not committed.
  Every async call runs on a runtime in `mobile-core`, never on the app's UI thread.
- `androidApp/`: Android app in Kotlin with Jetpack Compose.
- `iosApp/`: iOS app in SwiftUI.

Both apps sign in with a password or single sign-on, whichever the server offers, stay signed in
between runs, browse folders, and select several rows. They also manage files: details, rename,
move, delete (to the recycle bin) and save to a chosen location. Tapping a file downloads it and
opens it in the system's viewer.

Routes, JSON, session refresh, error mapping, sorting and size labels all come from the Rust crates,
so nothing is mirrored by hand. Dates are the exception: Rust exports the time, and each app formats
it with the platform's formatter for the device's locale and time zone. `mobile-core/src/lib.rs` is a thin mapping to FFI-friendly
records (ids as strings, times as `SystemTime`). Add a call there, and both apps get it on their
next build.

## Requirements

- Rust targets and tools:
  ```sh
  rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-linux-android x86_64-linux-android
  cargo install cargo-ndk
  ```
- JDK 17 or newer
- Android: Android Studio (or the Android SDK with `ANDROID_HOME` set), API 37, and the NDK
  (`sdkmanager "ndk;30.0.16248370"`, or SDK Manager > SDK Tools > NDK)
- iOS: a Mac with Xcode 16 or newer, and [XcodeGen](https://github.com/yonaskolb/XcodeGen)
  (`brew install xcodegen`)

## Android

Open `clients/mobile` in Android Studio and run `androidApp`, or:

```sh
./gradlew :androidApp:installDebug
./gradlew :androidApp:testDebugUnitTest   # the bindings on the JVM, against the host library
```

`androidApp/build.gradle.kts` runs `mobile-core/scripts/build-android.sh`. The Kotlin bindings
come from a host build, and the `.so` files (arm64-v8a, x86_64) from `cargo ndk`. Debug builds
use Cargo's debug profile, and release builds use `mobile`. The bindings load the library through
JNA.

The app defaults to `http://10.0.2.2:8080`, which is the host machine as seen from the emulator.

The server must listen on the host's loopback (`127.0.0.1` or `0.0.0.0`), since `10.0.2.2` maps
there. A server bound only to a LAN address is unreachable from the emulator.

On Android 17 (API 37) and later, connections to private addresses (the LAN, `10.0.2.2`) need
the runtime permission `ACCESS_LOCAL_NETWORK` ("nearby devices"), which the app asks for at
launch. Without it, Android silently drops the connection, which looks like a timeout, while
`adb shell` can still reach the server.

## iOS

The Xcode project is generated from `iosApp/project.yml` and is not committed. XcodeGen needs the
generated bindings folder to exist, so build the Rust client once first:

```sh
../mobile-core/scripts/build-ios.sh debug
cd iosApp
xcodegen generate
open iosApp.xcodeproj
```

The `RustCore` aggregate target runs `mobile-core/scripts/build-ios.sh` before the app builds
(`debug` for the Debug configuration, `mobile` otherwise). It writes `HomeSpaceCore.xcframework`
(device and simulator static libraries) and the Swift bindings to `build/rust-core/ios`, so
building in Xcode is enough. Run `xcodegen generate` again after adding or removing Swift files.

The app defaults to `http://localhost:8080`. `iosAppUITests` checks that the app offers sign-in
once the server answers, so it needs the server running on `localhost:8080` with password sign-in
on, and nobody signed in on the simulator.

## Opened files

Tapping a file keeps a copy on the device, in `filesDir/downloads` on Android and Application
Support `/downloads` on iOS (excluded from backups). `mobile-core/src/files.rs` owns the layout,
which needs no database:

```
<downloads>/<user id>/<node id>/<sha256>/<name>    the copy; `v<version>` if the node has no hash
<downloads>/<user id>/<node id>/download.part      a download in progress
```

- The hash is in the path, so new content on the server simply misses: the badge goes, and the
  next tap downloads again. Older hash folders are removed once the new copy is in place.
- Each open asks the server for the current node, and only falls back to any local copy if the
  server can't be reached. Tapping a file with a current copy downloads nothing.
- The server has no Range support, so an interrupted download restarts on the next tap. Downloads
  stop when the OS suspends or kills the app.
- Cancelling is a flag checked between chunks. Requests are never aborted, because a token refresh
  in flight must finish or the rotated token is lost.
- Signing out deletes the whole folder, even if the server can't be reached.
- A file deleted elsewhere keeps its copy until sign-out, as do files inside a folder deleted in
  the app. Nothing tracks copies yet: see `docs/todo/mobile-file-tracking.md`.

Android hands files to viewers through a `FileProvider` scoped to `downloads/`, with a read-only
grant. iOS uses Quick Look. Save to… picks the location first on Android, but on iOS only after the
download, because the system's export picker needs a file that already exists.

## Single sign-on

The browser returns to the app at `app.homespace://oidc/callback`
(`protocol::paths::MOBILE_OIDC_REDIRECT`), which the server allows along with PKCE. It isn't
registered at the identity provider, which only ever sees the server's own callback.

To try it, start Keycloak and the server as in the root `README.md`. `HOMESPACE_PUBLIC_URL` must be
https or loopback, so `10.0.2.2` won't do on the Android emulator. Forward the ports instead, and
use `http://127.0.0.1:8080` as the server in the app:

```sh
adb reverse tcp:8080 tcp:8080
adb reverse tcp:8180 tcp:8180
```

The iOS simulator shares the host's network, so it needs no forwarding.

## Notes

- Size: Rust adds about 5 MB to the stripped iOS binary with the `mobile` Cargo profile
  (`opt-level = "s"`, fat LTO), and about 14 MB with the plain `release` profile.
- Builds: the first Rust build per target takes about 30 s. The iOS script clears Xcode's
  environment, because cargo's C builds (aws-lc-sys) break under it. The Rust build also has its
  own Xcode target, so Xcode does not read the xcframework while it is being replaced.
  Xcode copies the xcframework's headers before that target runs, so the first build after a
  change to the exported API can fail with missing `uniffi_homespace_core_*` symbols. Build again.
- UniFFI: no error field may be named `message`, because it clashes with Kotlin's
  `Throwable.message`.
- HTTPS on Android: reqwest verifies certificates with `rustls-platform-verifier`, whose Kotlin
  half comes from upstream's Maven repository (see `settings.gradle.kts`), at the version in
  `Cargo.lock`. `PlatformVerifier` initialises it at startup. Don't add a `networkSecurityConfig`
  to the app: it would replace the library's allow-list of revocation servers, and certificates
  without an OCSP responder, such as all of Let's Encrypt's, would be reported as revoked.
- Plain HTTP: the Rust client's sockets aren't subject to Android's network security config or
  iOS's App Transport Security, so the apps will sign in to any `http://` address they are given.
- Open: background transfers. reqwest cannot upload while an iOS app is suspended. That needs a
  background `URLSession` in Swift.
