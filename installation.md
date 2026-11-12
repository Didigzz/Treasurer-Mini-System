# Installing the phone app (Cargo, no Gradle)

The phone app is `apps/mobile`, written in Rust with Slint. It is built and packaged by
Cargo + [xbuild](https://github.com/rust-mobile/xbuild); there is no Gradle project, no
`gradlew`, and no `expo prebuild` step.

## Prerequisites

- Rust with the Android target:
  ```bash
  rustup target add aarch64-linux-android
  ```
- Android SDK and NDK (the NDK linker is required; this machine has
  `%LOCALAPPDATA%\Android\Sdk\ndk\27.1.12297006`):
  ```bash
  export ANDROID_HOME="$LOCALAPPDATA/Android/Sdk"
  export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/27.1.12297006"
  ```
- JDK 17. The app contains no Java or Kotlin, but xbuild uses `keytool` and `apksigner`
  (JVM programs) to sign the debug APK.
- `adb` from the Android SDK platform-tools.

## Build the debug APK

```bash
cargo install --git https://github.com/rust-mobile/xbuild.git   # once
(cd apps/mobile && x build --platform android --arch arm64 --format apk)
```

The APK is written under the workspace `target/x/debug/android/` directory; xbuild prints
the exact path.

## Install it

With a USB-connected phone and developer mode enabled:

```bash
adb install -r target/x/debug/android/boarding-house-treasurer.apk
```

Or copy the APK to the phone and open it, allowing "install unknown apps" for the file
manager.

The package id is `com.anonymous.mybettertapp` — the same as the old Expo app — so
installing over it keeps the existing `treasurer.db`. Android only allows that when the
signing key matches. If the install is rejected:

1. Open the old app → Settings → Export backup.
2. Uninstall the old app (this deletes its local database).
3. Install the Rust app.
4. Open Settings → Import newest backup from app folder.

Backups live in the app's external files folder:
`Android/data/com.anonymous.mybettertapp/files/`. Copy them off the phone over USB to
keep them safe.

## Run on the desktop (UI preview)

```bash
cargo run -p treasurer-mobile --bin treasurer
```

This uses `%APPDATA%\com.boardinghouse.treasurer\treasurer.db` and the same code paths,
so it is the fastest way to check a screen without a phone.

## Tests

```bash
cargo test -p treasurer-core
```

This covers the ported money, split, payment/change, dashboard, history, and backup
logic, including a full export/restore round trip.
