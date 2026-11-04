# Mobile clients

Native UI on each platform, with the code that talks to the server shared in Kotlin:

- `shared/`: Kotlin Multiplatform module (the mobile api-client). Built as an Android library
  and as the `Shared` framework for iOS. HTTP goes through Ktor (OkHttp on Android,
  NSURLSession on iOS) and JSON through kotlinx.serialization.
- `androidApp/`: Android app in Kotlin with Jetpack Compose.
- `iosApp/`: iOS app in SwiftUI, linking the `Shared` framework.

For now both apps have one screen that calls `GET /api/v1/health` through `HomeSpaceClient`.

The Rust `clients/api-client` stays the client for web and desktop. Server routes are mirrored
by hand in `shared/src/commonMain/kotlin/app/homespace/shared/Paths.kt`.

## Spike: the Rust client through UniFFI

Both apps have a second tab, **Rust**, that does the same server check plus sign-in and folder
browsing through `clients/mobile-core` instead of the Kotlin `shared` module. `mobile-core`
wraps the Rust `api-client` and `client-core` (the code desktop and web run) and exports them
with [UniFFI](https://mozilla.github.io/uniffi-rs/): Rust `async fn` becomes a Kotlin `suspend
fun` and a Swift `async throws` func, and Rust errors become typed exceptions. The bindings are
generated on every build and are not committed.

- iOS: the `RustCore` aggregate target runs `mobile-core/scripts/build-ios.sh`, which builds
  `HomeSpaceCore.xcframework` (device + simulator static libraries) and the Swift bindings into
  `build/rust-core/ios`. Run the script once before the first `xcodegen generate`, since
  XcodeGen needs the generated folder to exist. `iosAppUITests` checks the server through both
  tabs (needs the server on `localhost:8080`).
- Android: `androidApp/build.gradle.kts` runs `mobile-core/scripts/build-android.sh`: the
  Kotlin bindings come from a host build, the `.so` files (arm64-v8a, x86_64) from `cargo ndk`.
  The bindings load the library through JNA. `./gradlew :androidApp:testDebugUnitTest` runs the
  bindings on the JVM against the host build of the library.

Requirements on top of the ones below: `rustup target add aarch64-apple-ios
aarch64-apple-ios-sim aarch64-linux-android x86_64-linux-android`, `cargo install cargo-ndk`,
and the Android NDK (`sdkmanager "ndk;30.0.16248370"`).

Findings so far:

- One client: routes, JSON, session refresh, error mapping, sorting and size/date labels come
  from the Rust crates; nothing is mirrored by hand. The FFI layer (`mobile-core/src/lib.rs`) is
  a thin mapping to FFI-friendly records (ids as strings, times as `SystemTime`).
- Size: Rust adds about 5 MB to the stripped iOS binary (4.0 MB to 9.0 MB) with the `mobile`
  Cargo profile (`opt-level = "s"`, fat LTO), and about 14 MB with the plain `release` profile.
- Builds: the first Rust build per target takes about 30 s; Xcode's environment had to be
  cleared for cargo's C builds (aws-lc-sys), and the Rust build needs its own target so Xcode
  does not read the xcframework while it is being replaced.
- UniFFI: no error field may be named `message` (it clashes with Kotlin's `Throwable.message`).
- Open: HTTPS on Android. reqwest verifies certificates with `rustls-platform-verifier`, which
  on Android must be initialised from JNI with the app context and ships a small Kotlin
  component; plain HTTP to the dev server works without it. iOS needs nothing extra.
- Open: background transfers. Neither reqwest nor Ktor uploads while an iOS app is suspended;
  that needs a background `URLSession` in Swift whichever client is chosen.

## Requirements

- JDK 17 or newer
- Android: Android Studio (or the Android SDK with `ANDROID_HOME` set), API 36
- iOS: a Mac with Xcode 16 or newer, and [XcodeGen](https://github.com/yonaskolb/XcodeGen)
  (`brew install xcodegen`)

## Shared module

```sh
cd clients/mobile
./gradlew :shared:testAndroidHostTest   # commonTest on the JVM
./gradlew :shared:iosSimulatorArm64Test # commonTest on the iOS simulator (Mac only)
```

## Android

Open `clients/mobile` in Android Studio and run `androidApp`, or:

```sh
./gradlew :androidApp:installDebug
```

The app defaults to `http://10.0.2.2:8080`, which is the host machine as seen from the emulator.
Plain HTTP is allowed only to `10.0.2.2` and `localhost`
(`androidApp/src/main/res/xml/network_security_config.xml`).

The server must listen on the host's loopback (`127.0.0.1` or `0.0.0.0`), since `10.0.2.2` maps
there; a server bound only to a LAN address is unreachable from the emulator.

On Android 17 (API 37) and later, connections to private addresses (the LAN, `10.0.2.2`) need
the runtime permission `ACCESS_LOCAL_NETWORK` ("nearby devices"), which the app asks for at
launch. Without it the connection is silently dropped and looks like a timeout, while
`adb shell` can still reach the server.

## iOS

The Xcode project is generated from `iosApp/project.yml` and is not committed:

```sh
cd clients/mobile/iosApp
xcodegen generate
open iosApp.xcodeproj
```

A pre-build step runs `./gradlew :shared:embedAndSignAppleFrameworkForXcode`, which compiles the
Kotlin framework for the selected configuration and SDK, so building in Xcode is enough. The app
defaults to `http://localhost:8080`, and App Transport Security allows plain HTTP only to local
addresses. Run `xcodegen generate` again after adding or removing Swift files.
