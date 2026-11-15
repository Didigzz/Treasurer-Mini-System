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
  export PATH="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/windows-x86_64/bin:$PATH"
  ```
  xbuild downloads its own NDK headers/libraries, but calls `clang`, `llvm-ar`, and
  `lld` by name. Without the NDK toolchain on `PATH` the build fails with
  ``linker `clang` not found``.
- JDK 17. The app contains no Java or Kotlin, but xbuild uses `keytool` and `apksigner`
  (JVM programs) to sign the debug APK.
- `adb` from the Android SDK platform-tools.

## Build the debug APK

```bash
cargo install --git https://github.com/rust-mobile/xbuild.git   # once
(cd apps/mobile && x build --platform android --arch arm64 --format apk)
```

The APK is written to `target/x/debug/android/treasurer-mobile.apk`; xbuild prints the
exact path. Package id, label, and version come from `apps/mobile/manifest.yaml` (xbuild
0.2.0 does not read `[package.metadata.android]` from `Cargo.toml`).

## Install it

With a USB-connected phone and developer mode enabled:

```bash
adb install -r target/x/debug/android/treasurer-mobile.apk
```

Or build, install, and launch in one step:

```bash
(cd apps/mobile && x run --device adb:<serial>)   # serial from `x devices`
```

You can also copy the APK to the phone and open it, allowing "install unknown apps" for the
file manager.

The package id is `com.anonymous.mybettertapp` — the same as the old Expo app. Android only
allows updating in place when the signing key matches; the Expo build is signed with a
different key than xbuild's debug key, so an existing install must be removed first. The old
Expo app kept its database in the app-internal `files/SQLite/treasurer.db`; this app uses
`databases/treasurer.db`, so copy it before uninstalling if you need the data:

```bash
adb shell run-as com.anonymous.mybettertapp \
  sh -c 'tar -cf - files/SQLite' > appdata.tar
# uninstall, install the new APK, then:
adb push appdata.tar /data/local/tmp/appdata.tar
adb shell run-as com.anonymous.mybettertapp \
  sh -c 'mkdir -p databases && tar -xf /data/local/tmp/appdata.tar \
  && cp files/SQLite/treasurer.db* databases/'
```

Alternatively, open the old app → Settings → Export backup before uninstalling, then import
it from the new app's Settings screen. Backups live in the app's external files folder:
`Android/data/com.anonymous.mybettertapp/files/`.

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
