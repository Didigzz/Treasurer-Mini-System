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
logic, including a full export/restore round trip. The mobile crate also asserts that
`manifest.yaml` carries the same `version_name`/`version_code` as
`apps/mobile/src/version.rs`:

```bash
cargo test -p treasurer-mobile --lib version
```

## In-app updates

The app checks
`https://api.github.com/repos/Didigzz/Treasurer-Mini-System/releases/latest` on every
launch and from Settings → App updates. A newer `version_code` is downloaded in the
background to the app-private cache (`cache/updates`, at most one APK), verified against
the release's `<apk>.sha256` asset, and handed to Android's installer. The check is silent
when it fails at launch; Settings shows the failure with Retry.

For desktop previews, point `TREASURER_UPDATE_FEED` at a scratch JSON payload (a `file://`
URL works) and the whole check/download flow runs without publishing a release. The
install handoff is Android-only.

The APK declares `INTERNET` (the release check) and `REQUEST_INSTALL_PACKAGES` (the
installer handoff). Nothing about the treasurer's records is uploaded. The app never
installs anything silently: Android always shows its own confirmation, and Android 8+
requires the user to allow "Install unknown apps" for this app once.

Tapping **Install update** copies the verified APK into the phone's Downloads collection
and opens it with the system installer, which is why the handoff needs Android 10 (API 29)
or newer. On older phones, download the APK from the release page and open it by hand.

## Release builds and signing

Android only installs an update in place when the new APK is signed with the same key as
the installed one. From 1.4.0 on, releases use one stable signing key.

This xbuild version signs APKs itself from a **PEM file** containing an RSA-2048 private
key and its certificate; `x build --pem <file>` replaces the built-in debug signer. There
is no keystore flag in `x build --help` and no keystore environment variable, so the PEM
is the release key:

```bash
# Once, ever. Keep both files private and backed up in two places.
openssl req -x509 -newkey rsa:2048 -nodes -days 36500 \
  -keyout release-key.pem -out release-cert.pem \
  -subj "/CN=Boarding House Treasurer"
cat release-key.pem release-cert.pem > release.pem

(cd apps/mobile && x build --platform android --arch arm64 --release --format apk \
  --pem ../../release.pem)
```

Losing or rotating `release.pem` means every user must export a backup, uninstall, install
the new APK, and restore — the same one-time step needed to move off the debug-signed
1.3.0 and Expo builds, whose key differs.

### Release tags

Releases are tagged `v<version_name>+<version_code>`, for example `v1.4.0+5`. The
`version_code` is the only comparison the updater uses; the release must also carry the
APK asset and a sidecar named exactly `<apk file name>.sha256` (first 64-hex token).

`.github/workflows/release.yml` treats the tag as the single source of truth. It rewrites
`apps/mobile/manifest.yaml`, the version fields in `apps/mobile/Cargo.toml`, and
`VERSION_NAME`/`VERSION_CODE` in `apps/mobile/src/version.rs`, refreshes `Cargo.lock`,
commits that bump to `main`, then builds and signs the APK, checks its `versionCode`, and
uploads both files to the GitHub Release. Re-running a tag is safe: the bump is skipped
when `main` already carries that version, and release assets are replaced.

### CI secrets

- `ANDROID_SIGNING_PEM_BASE64` — `base64 -w0 release.pem` (key + certificate).

`GITHUB_TOKEN` is used for creating the release and uploading assets; no store publishing
is configured.

### Release checklist

1. Make sure `main` is green. `.github/workflows/ci.yml` runs `cargo clippy -D warnings`
   and `cargo test --workspace` on every pull request and push to `main`; the
   manifest/version consistency test is part of that suite.
2. Commit everything the release should contain and push `main`.
3. Tag and push the tag: `git tag v1.4.2+7 && git push origin v1.4.2+7`.
4. Let `.github/workflows/release.yml` sync the version fields from the tag, commit the
   bump to `main`, build and sign the APK, check its `versionCode`, and publish the APK and
   `.sha256` on the GitHub Release. The bump commit is pushed with `GITHUB_TOKEN`, so it
   does not start another CI run. If `main` is protected against direct pushes, let
   GitHub Actions bypass that rule or the bump step fails.
5. Confirm `/releases/latest` returns the new assets.
6. Install 1.4.0 once by hand, then verify the in-app updater on the *next* release.
