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
