# In-App Updater and Settings Update UI — Specification

**Status:** Draft, ready for implementation once §11 open questions are closed
**Date:** 2026-11-16
**App:** Boarding House Treasurer 1.3.0 → 1.4.0 (`version_code` 4 → 5)
**Scope:** Release check, background APK download, Android install handoff, Settings "App updates" section, release CI
**Request:** "add an updater, add ui in the settings that checks release, auto update"

This document records what the app does today, the decisions made during the clarification
interview, the exact UX and copy, the feed contract, the Android install mechanics, the edge
cases, and the acceptance checks. No code has been changed yet.

---

## 1. Summary of decisions

| Topic | Decision |
|---|---|
| Release source | GitHub Releases **latest** for `Didigzz/Treasurer-Mini-System` |
| Platform | **Android APK only.** Check/download also run in the desktop preview so the flow is testable; the install handoff is Android-gated |
| "Auto update" | Check on launch → **download automatically** → prompt to install via Android's installer |
| Check timing | **Every app start**, plus a manual **Check for updates** button |
| Auto-check toggle | None. Checking is always on; there is no opt-out switch |
| Settings placement | New **App updates** section **above Backup & restore** |
| Release info depth | New `version_name` + APK size + a **one-line release summary**; no full notes viewer |
| Pending-update visibility | **Settings only**, plus a small dot on the **Settings item in the "+" menu** |
| Signing | Spec **requires a stable release keystore** from 1.4.0 on; signature failures surface a clear, actionable error |
| Install permission | Pre-flight `canRequestPackageInstalls()`; if off, explain and **open the app's "Install unknown apps" system screen** |
| Integrity | **SHA-256** from a **sidecar `.sha256` release asset**; refuse to install on mismatch |
| Data before install | **Reassurance only** — "Your records stay on this device and aren't affected by an update." No auto-backup, no backup prompt |
| Download UI | **Determinate progress bar** with percent and MB downloaded/total |
| Download scope | Downloads **cancel when Settings closes or the app is backgrounded**; partial files are removed, no resume |
| APK retention | **At most one APK** in an app-private update folder, cleaned on startup, replaced on newer version, deleted after handoff |
| Asset selection | **First `.apk` asset** in API order; log the chosen name in the status detail; error if none |
| "Newer" test | **Integer `version_code` only**; `version_name` is display text |
| Version code source | Release **tag encodes it**: `v1.4.0+5` (see §5.2) |
| Failure surfacing | Launch check failures are **silent**; the Settings section shows the failure with Retry. Download failures are visible in the section |
| Offline claim | Add `INTERNET`; keep "records never leave the device"; **disclose the update check in the README** |
| Publishing | **Add a GitHub Actions release workflow** (tag push → signed APK + `.sha256` uploaded to the release) |
| Version | 1.4.0 (`version_code` 5) in `manifest.yaml`, `Cargo.toml` metadata, and a new Rust version module |
| Schema / backup | **No schema change, no backup format change, no new tables or columns** |
| Dependencies | Add `ureq` (rustls, no OpenSSL) + `sha2` to the mobile crate; JNI access via the existing `jni` transitive crates |

---

## 2. Context and current behavior

### 2.1 Architecture

- Rust + Slint phone-first app. UI: `apps/mobile/ui/*.slint`; wiring and state: `apps/mobile/src/app/*.rs`.
- Domain, SQLite storage, money math, backup: `crates/treasurer-core`.
- Android glue lives in `apps/mobile/src/lib.rs` (`android_main`) and `apps/mobile/src/platform.rs`
  (a `OnceLock<(PathBuf, PathBuf)>` holding the database and backup dirs).
- Settings is screen id 4, rendered by `apps/mobile/ui/screens/settings.slint` and wired by
  `apps/mobile/src/app/settings.rs`.
- Design tokens live in `theme.slint`; shared controls (`SectionHeader`, `AppButton`, `Card`,
  `Banner`, `Segmented`, `ProgressBar`, `Sheet`, `Toggle`) in `components.slint` / `layout.slint`.

### 2.2 What exists today

- **No updater of any kind.** There is no network code anywhere in the workspace, no HTTP client
  dependency, and no `INTERNET` permission. `Cargo.lock` contains no `ureq`, `reqwest`,
  `self_update`, `rustls`, or `native-tls` entry.
- **Settings ends with a version label.** `SettingsScreen` renders `root.app-version` as a centered
  caption at the very bottom. The string is set in `App::start()`:
  `ui.set_app_version("Boarding House Treasurer 1.3.0".into())`.
- **The version number is duplicated in four places** and drifts: `apps/mobile/Cargo.toml`
  `[package] version = "0.1.0"`, `apps/mobile/Cargo.toml` `[package.metadata.android]`
  (`version_name = "1.3.0"`, `version_code = 4`), `apps/mobile/manifest.yaml`
  (`version_name: 1.3.0`, `version_code: 4`), and the hard-coded string in `mod.rs`.
  `manifest.yaml` is the file xbuild actually packages from.
- **Backup & restore** is the last maintenance section: export writes JSON into
  `platform::backup_dir()`, restore reads the newest JSON there. This is the pattern the update
  code should follow for file placement (`platform` helper + `std::fs`).
- **Threading:** the whole `App` is an `Rc<RefCell<App>>`; all UI mutation happens on the UI
  thread. There is no async runtime, no `Send` state, and no timer. `slint::Timer` is available.
- **Android manifest** is generated by xbuild from `apps/mobile/manifest.yaml`; it currently
  declares only package, version, and application label — no permissions.
- **Repo and publishing:** `origin` is `https://github.com/Didigzz/Treasurer-Mini-System.git`.
  There is no `.github/` directory, no CI, and no releases.
- **README promise:** "There are no accounts, cloud sync, or analytics." Limitations list
  "No cloud sync or online backup". Both need a small, honest amendment once the app talks to
  GitHub.

### 2.3 Why this is not trivial

1. Android never installs an APK silently for an ordinary app. It shows a system installer
   prompt, and only proceeds if the file is signed with the same key as the installed app.
   The app is currently debug-signed by xbuild, so 1.4.0 must establish a stable release key or
   every future update is un-installable over the previous build.
2. Installing from inside the app requires `REQUEST_INSTALL_PACKAGES` **and** the user granting
   "Install unknown apps" for this app. The grant lives in a system settings screen; the app can
   only deep-link to it.
3. Handing a downloaded file to Android requires either a `FileProvider` content URI or a
   `PackageInstaller` session — both need Android-side glue that this project has never had.
   §7 specifies the intended mechanism and its fallback.
4. The app has zero network permissions today. Adding one changes the app's privacy story, so
   the disclosure is part of the scope, not an afterthought.

---

## 3. Glossary and state model

### 3.1 Terms

- **Feed** — `GET https://api.github.com/repos/Didigzz/Treasurer-Mini-System/releases/latest`.
- **Release** — the JSON object returned by the feed; drafts and prereleases are already excluded
  by GitHub's `/releases/latest` endpoint.
- **Installed version** — compiled-in `VERSION_NAME` / `VERSION_CODE` (§8.1).
- **Update folder** — app-private directory holding at most one APK and its `.part` temp file.

### 3.2 `UpdateState`

One enum owns the section's state; every visual state in §6.3 maps to exactly one variant.

```rust
enum UpdateState {
    Idle,                       // nothing has run yet this session
    Checking,                   // feed request in flight
    UpToDate { latest: String, code: i64 },
    Ahead { installed: i64, latest: i64 },
    Available { release: ReleaseInfo },     // newer release known, download not started
    Downloading { release: ReleaseInfo, downloaded: u64, total: u64 },
    Ready { release: ReleaseInfo, path: PathBuf },
    Installing,                 // installer handoff launched
    NeedsPermission,            // canRequestPackageInstalls() == false
    Failed { kind: FailureKind, message: String, retry: bool },
}
```

`ReleaseInfo` is produced by the core feed parser (§5) and carries: `version_name`,
`version_code`, `apk_name`, `apk_url`, `apk_size`, `sha256_url`, `summary`, `html_url`,
`tag_name`.

`FailureKind`: `Network`, `RateLimited`, `BadFeed`, `MissingVersionCode`, `MissingApk`,
`MissingChecksum`, `ChecksumMismatch`, `Download`, `Write`, `InstallBlocked`,
`InstallSignatureMismatch`, `InstallFailed`.

### 3.3 Whole-flow sequence

```
app start ──► check_for_updates(Launch)          │ silent on failure
Settings opened ──► render current state
Check for updates ──► check_for_updates(Manual)  │ failure visible
        │
        ├─ not newer ────────────► UpToDate / Ahead
        └─ newer ────────────────► Available ──► auto start download
                                        │
                                        ▼
                                  Downloading (progress)
                                        │  verify SHA-256
                                        ▼
                                     Ready ──► [Install update]
                                        │
                                        ▼
                    NeedsPermission? ──► Open settings ──► back ──► retry
                                        │
                                        ▼
                            Android installer prompt ──► app replaced
                                        │
                                        └─ cancelled/failed ──► Ready, with error if failed
```

---

## 4. Requirement 1 — Platform integration (network, permissions, version constants)

### 4.1 Dependencies

Add to `apps/mobile/Cargo.toml`:

- `ureq` with the **rustls** TLS backend (`default-features = false`, features enabling rustls and
  JSON is optional — the feed may be parsed with `serde_json`, already a dependency). Rustls is
  required because Android has no OpenSSL and `native-tls` cannot be used.
- `sha2` for SHA-256.
- `jni` as an Android-only direct dependency (`[target.'cfg(target_os = "android")'.dependencies]`),
  pinned to the version already resolved in `Cargo.lock` to avoid a second jni in the graph.
- `serde` with `derive` for the feed structs, or hand-rolled `serde_json::Value` access. Prefer
  typed structs in core (§5).

No new dependency in `treasurer-core` beyond what it already has (`serde`, `serde_json`).

### 4.2 Android permissions and manifest

`apps/mobile/manifest.yaml` gains, under `android.manifest`:

- `android.permission.INTERNET`
- `android.permission.REQUEST_INSTALL_PACKAGES`

Both are install-time permissions; neither shows a runtime prompt. `REQUEST_INSTALL_PACKAGES` is
also what makes the "Install unknown apps" toggle appear for the app.

Verification step (part of the implementation, not optional): after building, confirm the generated
`AndroidManifest.xml` inside the APK contains both `<uses-permission>` entries, and that the
activity is still whatever xbuild generated (a `NativeActivity` variant). If xbuild's manifest
subset cannot express `permissions`, switch the Android build to xbuild's gradle mode with an
explicit `AndroidManifest.xml` — record which path was taken in the implementation notes.

### 4.3 Version constants (single source of truth)

New module `apps/mobile/src/version.rs`:

```rust
pub const VERSION_NAME: &str = "1.4.0";
pub const VERSION_CODE: i32 = 5;
pub const APP_LABEL: &str = "Boarding House Treasurer";

/// "Boarding House Treasurer 1.4.0"
pub fn version_label() -> String { format!("{APP_LABEL} {VERSION_NAME}") }
```

- `App::start()` sets `app_version` from `version::version_label()` instead of the literal string.
- The updater compares `VERSION_CODE` with the release's code.
- `apps/mobile/manifest.yaml`, `apps/mobile/Cargo.toml` `[package.metadata.android]`, and
  `apps/mobile/Cargo.toml` `[package] version` are all set to `1.4.0` / `5`.
- Add a `#[cfg(test)]` test in `version.rs` that `include_str!("../manifest.yaml")` and asserts the
  manifest's `version_name` and `version_code` match the constants, so a forgotten bump fails a
  test instead of shipping a wrong label.

### 4.4 Update folder plumbing

Extend `apps/mobile/src/platform.rs`:

- Android: `updates_dir()` returns `<internal data>/../cache/updates`
  (i.e. `/data/data/<pkg>/cache/updates`), created on demand. The path is captured in
  `android_main` and stored in the existing `OnceLock` pattern (extend the tuple or add a second
  `OnceLock`; the former is preferred so there is one initialization point).
- Desktop: `updates_dir()` returns `desktop_dir().join("updates")`, so the flow can be exercised in
  the preview.
- `cleanup_updates_dir()` removes every file except the APK belonging to the newest known release,
  and removes stale `.part` files. Called once on startup.

---

## 5. Requirement 2 — Feed parsing and the check

### 5.1 Feed request

```
GET https://api.github.com/repos/Didigzz/Treasurer-Mini-System/releases/latest
Accept: application/vnd.github+json
X-GitHub-Api-Version: 2022-11-28
User-Agent: boarding-house-treasurer/<VERSION_NAME>
```

- GitHub requires a `User-Agent`; requests without one are rejected.
- No token is embedded. Unauthenticated calls are limited to 60/hour per IP, which is far above one
  call per launch for a single-user app.
- Timeouts: connect 10 s, whole check 20 s, read 20 s. Redirects are followed.
- Response handling:
  - `200` → parse.
  - `403` or `429` with `x-ratelimit-remaining: 0` → `Failed { RateLimited }`.
  - Any other non-2xx, DNS failure, TLS failure, or timeout → `Failed { Network }`.
  - Invalid JSON or unexpected shape → `Failed { BadFeed }`.
- The check never writes to the database and never touches user records.
- Dev hook: `TREASURER_UPDATE_FEED` env var, when set, replaces the URL. Intended for the desktop
  preview against a local `file://` or a scratch endpoint; ignored on Android builds.

### 5.2 Feed contract (what a release must contain)

| Field | Source | Required | Use |
|---|---|---|---|
| `version_name` | `tag_name` before `+` | yes | Display text ("1.4.0") |
| `version_code` | `tag_name` after `+` | yes | **Only** comparison key |
| `tag_name` | release | yes | Stored for diagnostics |
| APK asset | `assets[]` where `name` ends `.apk` (case-insensitive), first in API order | yes | Download target |
| APK size | chosen asset `size` | yes | Progress + display |
| APK URL | chosen asset `browser_download_url` | yes | Download |
| `.sha256` asset | `assets[]` named `<apk file name>.sha256` | yes | Integrity |
| `summary` | `body`, first non-empty line, whitespace collapsed, ≤ 160 chars + `…` | no | One-line summary |
| `html_url` | release | no | Not shown in 1.4.0; retained for future |

**Tag format is the release contract:** `v<version_name>+<version_code>`, e.g. `v1.4.0+5`.
A leading `v` is optional. `version_name` must be three dot-separated numeric components.
If the tag does not match, the check fails with `MissingVersionCode` and the copy in §6.5.
`/releases/latest` already excludes drafts and prereleases, so no extra filtering is required.

Checksum file format: the **first 64-hex-character token** in the file, lowercased. Both
`<hex>` and `<hex>  boarding-house-treasurer-1.4.0.apk` (sha256sum output) parse. A file with no
such token is `MissingChecksum`.

### 5.3 Pure helpers (core, tested)

New `crates/treasurer-core/src/update.rs`, exported from `lib.rs` (same pattern as `selection.rs`).
No network, no database, no filesystem:

```rust
pub struct ReleaseAsset { pub name: String, pub download_url: String, pub size: u64 }
pub struct ReleaseInfo {
    pub version_name: String,
    pub version_code: i64,
    pub tag_name: String,
    pub apk_name: String,
    pub apk_url: String,
    pub apk_size: u64,
    pub sha256_url: String,
    pub summary: String,
    pub html_url: String,
}

pub struct ReleaseResponse { pub tag_name: String, pub body: Option<String>,
                             pub html_url: String, pub assets: Vec<ReleaseAsset> }

/// Parses `v1.4.0+5` / `1.4.0+5`. Returns None for anything else.
pub fn parse_release_tag(tag: &str) -> Option<(String, i64)>;

/// True only when the release code is strictly greater than the installed code.
pub fn is_newer(release_code: i64, installed_code: i32) -> bool;

/// First asset whose name ends with ".apk" (case-insensitive), in API order.
pub fn pick_apk_asset(assets: &[ReleaseAsset]) -> Option<&ReleaseAsset>;

/// The sidecar name for an APK asset: "<apk name>.sha256".
pub fn checksum_asset_name(apk_name: &str) -> String;

/// First 64-hex token, lowercased.
pub fn parse_sha256(text: &str) -> Option<String>;

/// First non-empty line, whitespace-collapsed, truncated to `max` chars with '…'.
pub fn release_summary(body: &str, max: usize) -> String;

/// "29.1 MB" / "812 KB" / ">1 KB" style, one decimal, using 1024-based units.
pub fn format_size(bytes: u64) -> String;

/// Case-insensitive hex comparison of the computed digest with the published one.
pub fn checksum_matches(computed_hex: &str, expected_hex: &str) -> bool;

/// Turns a full GitHub release payload into ReleaseInfo, or a typed error.
pub fn build_release_info(response: ReleaseResponse) -> Result<ReleaseInfo, UpdateFeedError>;
```

`UpdateFeedError` is a small enum (`MissingVersionCode`, `MissingApk`, `MissingChecksum`) so the
mobile layer maps it to `FailureKind` without string matching.

### 5.4 Where the check runs

- `App::start()` calls `self.check_for_updates(CheckSource::Launch)` after the settings/UI are
  loaded, so the Settings screen can never render a stale "checking" state on first paint.
- The Settings button calls the same function with `CheckSource::Manual`.
- A launch check must not block the UI: it runs on a worker thread (§7.1) and the app is fully
  usable while it runs.
- Failure at launch: state becomes `Failed { .. }` but **nothing is shown outside Settings** — no
  banner, no message, no interruption.
- Success at launch with a newer release: the download starts automatically (§6.4), still without
  any interruption outside Settings.
- No timestamp is persisted. "Last checked" is in-memory for the session.

---

## 6. Requirement 3 — Download

### 6.1 Behavior

Triggered automatically when a check finds a newer release (`Available` → `Downloading`
immediately), and again by Retry after a failed download. There is no separate "Download" button in
the happy path; the summary/status line carries the progress.

- Target path: `<updates_dir>/<version_code>-<apk file name>`, written to
  `<target>.part` first and renamed only after the checksum verifies.
- `total` = the asset's `size` from the feed. If a `Content-Length` is also sent and differs, use
  the `Content-Length` for the progress denominator and warn in the status detail only.
- Progress is reported as bytes; percent = `downloaded * 100 / total`, clamped to 0..=100.
- On completion, compute SHA-256 over the file and compare with the sidecar.
  - Match → rename `.part` → target, state `Ready`.
  - Mismatch → delete both files, `Failed { ChecksumMismatch, retry: true }`.
- A failed or cancelled download deletes the `.part` file. **No resume** in this release.
- Before starting, delete any other APK in the update folder (one APK only).
- Sanity cap: refuse to download an asset larger than 150 MB (guards against a wrong asset upload);
  `Failed { Download, "That update is larger than expected." }`.

### 6.2 Cancellation

- Leaving Settings (`set_screen` to anything other than 4) sets the download's `cancel` flag.
- Backgrounding the app sets the same flag (Android `on_pause` equivalent; if the activity
  lifecycle callback is unavailable through Slint's Android backend, cancelling on screen change
  plus on window hidden is the required minimum and must be reported as such).
- The worker checks the flag between read chunks (≤ 64 KiB granularity) and exits promptly.
- Cancelled state: partial file deleted, state back to `Available`, and the status line reads
  "Download cancelled." until the next visit or Retry.
- No explicit Cancel button (per interview). Cancellation is implicit by leaving.

### 6.3 Progress rendering

- Progress bar: reuse `ProgressBar` from `components.slint` (`value` 0..1, tone accent).
- Label: `"Downloading update… 42% · 12.4 MB of 29.1 MB"` — `format_size` for both numbers.
- Update at most every 150 ms (or every 1%), whichever comes first, so the UI thread is never
  flooded and the Slint property churn stays cheap.
- Nothing animates beyond the bar's existing width animation.

### 6.4 Retention rules

| Event | Effect |
|---|---|
| App start | Delete every file in the update folder except nothing (state is unknown), then recreate on demand; stale `.part` files always removed |
| Newer release found while an APK is present | Delete the old APK before downloading the new one |
| Download completes | Exactly one APK in the folder |
| Checksum mismatch | Both files deleted |
| Install handoff launched | APK **kept** until the installer reports success or the app restarts; deleted on the next startup if the version changed, or immediately on a reported failure |
| Settings left mid-download | `.part` deleted |

Retention is deliberately conservative: the file is removed once it can no longer be needed, and
never kept across versions.

---

## 7. Requirement 4 — Install handoff (Android)

### 7.1 Threading model

`App` is `Rc<RefCell<App>>` and not `Send`, so the worker never touches it directly.

```rust
struct UpdateShared {          // Arc<Mutex<..>> captured by the worker
    state: UpdateState,        // written by the worker only
    downloaded: u64,
    total: u64,
    error: Option<String>,
    cancel: Arc<AtomicBool>,
    last_checked_at: Option<Instant>,
}
```

- Worker: `std::thread::spawn` for the HTTP request, the download, and the hashing.
- UI thread: one `slint::Timer` (started lazily when a check begins, 150 ms tick) reads the shared
  state and calls `App::apply_update_state(..)`, which sets the Slint properties and decides
  whether a transition (e.g. `Available → Downloading`, `Ready`) needs a follow-up action.
- The timer stops when the state is terminal (`UpToDate`, `Ahead`, `Ready`, `Failed`,
  `NeedsPermission`) and no download is running.
- Never call `slint::invoke_from_event_loop` from the worker with UI handles; only plain data may
  cross the thread boundary.

### 7.2 Permission pre-flight

On tapping **Install update**:

1. Call `PackageManager.canRequestPackageInstalls()` (API 26+). Below API 26 this returns true by
   default and the check is skipped.
2. If false → state `NeedsPermission`; the section shows the explanation and an **Open settings**
   button that starts
   `Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES` with `Uri.parse("package:<pkg>")`.
3. On returning to the app (screen re-entered or resume), re-check. If granted, the button becomes
   **Install update** again. Keep the downloaded APK so the treasurer does not re-download.
4. If the settings activity does not resolve on the device, fall back to
   `Settings.ACTION_MANAGE_APPLICATIONS_SETTINGS` and show the manual instruction
   "Allow installs from this app, then come back." Never claim the permission is granted without
   re-checking.

### 7.3 Handing the file to Android

Shipped mechanism — `MediaStore` copy plus `Intent.ACTION_VIEW` (no Java/Kotlin class required):

- A `PackageInstaller` session does **not** work for this app. `commit()` only delivers
  `PackageInstaller.STATUS_PENDING_USER_ACTION` to the status receiver, and the app must start the
  confirmation Intent found in `Intent.EXTRA_INTENT` itself (see AOSP's `InstallApkSessionApi`
  sample). That needs a `BroadcastReceiver`, or an activity `onNewIntent`, plus a `FileProvider`
  for the content URI. xbuild produces an APK with no `classes.dex`, so neither can be shipped.
  Committing a session without handling that status shows nothing at all and reports no error.
- Copy the verified APK into `MediaStore.Downloads` (`is_pending` = 1 while writing, 0 after). This
  is the only system content provider that can hand the installer the bytes; the app's cached file
  is app-private, and `file://` URIs are rejected from Android 7 on.
- Launch `Intent.ACTION_VIEW` with `setDataAndType(uri, "application/vnd.android.package-archive")`,
  `FLAG_GRANT_READ_URI_PERMISSION`, and `FLAG_ACTIVITY_NEW_TASK`. Android then shows its own
  installer UI, which is the confirmation the user sees.
- Before writing, delete any row with the same `_display_name`, so Downloads holds at most one
  update file. A failed hand-off deletes its row again.
- `MediaStore.Downloads` is API 29+; below that the hand-off reports
  "Installing from inside the app needs Android 10 or newer" instead of failing silently.
- Keep every `unsafe` block in `install.rs` and document the object lifetimes.

### 7.4 After the install

- Android replaces the app and usually kills the process. Nothing needs to run in the app.
- The database path is unchanged by an update; records are untouched. This is why the dialog only
  reassures (§6.5 copy) instead of forcing a backup.
- If the user cancels the system prompt: state returns to `Ready` and the APK is kept for a retry.
- Signature mismatch (`INSTALL_FAILED_UPDATE_INCOMPATIBLE`) → `Failed { InstallSignatureMismatch }`
  with the copy in §8.5. The APK is deleted; retrying cannot help.
- Other failures (`INSTALL_FAILED_INSUFFICIENT_STORAGE`, `INSTALL_FAILED_INVALID_APK`,
  `INSTALL_FAILED_VERSION_DOWNGRADE`) → `Failed { InstallFailed }` showing Android's status name
  verbatim in the status detail so it can be diagnosed.

### 7.5 Desktop behavior

- The check and download run, so the section is testable in `cargo run -p treasurer-mobile --bin treasurer`.
- The install action does not exist. `Ready` on desktop reads
  "Update downloaded. Installing is only supported on Android."
- No `cfg`-gated stubs may make the desktop build fail to compile; gate with
  `#[cfg(target_os = "android")]` at the module and call-site level only.

---

## 8. Requirement 5 — Settings UI

### 8.1 Placement

`apps/mobile/ui/screens/settings.slint`, a new `Section` **above** "Backup & restore" and below
"Bill types":

```
Money
Bill types
App updates        ← new
Backup & restore
Boarding House Treasurer 1.4.0   (existing footer, kept)
```

The section is always visible (never conditionally hidden), because the manual Check button lives
there and the version row must always be readable.

### 8.2 Structure

```slint
Section {
    SectionHeader {
        title: "App updates";
        caption: "Checks GitHub for new versions when the app opens";
    }
    Card {
        VerticalLayout {
            padding: Theme.space-4;
            spacing: Theme.space-4;

            // Installed version row
            HorizontalLayout { Text { "Installed version" } Text { "1.4.0 (5)" } }

            Divider { }

            // Status block
            Text { status }                       // §8.3 primary line
            if summary != "" { Text { summary } } // one-line release summary, elided
            if downloading { ProgressBar { value } Text { progress-label } }

            // Actions
            HorizontalLayout {
                if can-check { AppButton "Check for updates" (secondary, compact) }
                if can-install { AppButton "Install update" (primary) }
                if needs-permission { AppButton "Open settings" (primary) }
                if can-retry { AppButton "Retry" (secondary, compact) }
            }
            Text { detail }   // last-checked / failure detail, caption, muted
        }
    }
}
```

- Only `Theme` tokens; no new colors, radii, or sizes. `ProgressBar`, `Card`, `SectionHeader`,
  `AppButton`, `Divider` are reused as-is.
- Buttons are full-width stacked on very narrow screens? No — the existing `HorizontalLayout`
  with `AppButton` hugs its label; at 320 px the action row must not overflow, so at most two
  buttons are visible per state (§8.3) and they wrap using a `VerticalLayout` when the window is
  under 360 px (`Responsive.width` already exists for this).
- 44 px minimum touch targets come from `AppButton`'s 48 px default/min 40 px compact.

### 8.3 State → UI matrix

| State | Status line | Summary | Progress | Actions | Detail |
|---|---|---|---|---|---|
| `Idle` | "Not checked yet." | — | — | Check for updates | — |
| `Checking` | "Checking for updates…" | — | — | Check (disabled) | — |
| `UpToDate` | "You're on the latest version." | — | — | Check for updates | "Checked just now" |
| `Ahead` | "You're on a newer build than the latest release." | — | — | Check for updates | "Installed 1.4.0 (5) · latest release 1.3.0 (4)" |
| `Available` | "Version 1.4.0 is available (29.1 MB)." | summary | — | Check (disabled) | "Download starting…" |
| `Downloading` | "Downloading update…" | summary | bar + "42% · 12.4 MB of 29.1 MB" | Check (disabled) | "Leaving Settings cancels the download." |
| `Ready` | "Update ready to install." | summary | — | Install update (primary), Check (disabled) | "Version 1.4.0 · 29.1 MB" |
| `Installing` | "Opening Android's installer…" | summary | — | — | "Your records stay on this device and aren't affected by an update." |
| `NeedsPermission` | "Allow installs from this app to continue." | — | — | Open settings (primary) | "Android needs permission to install the update. Nothing is downloaded again." |
| `Failed(Network)` | "Couldn't check for updates." | — | — | Retry + Check | "Check your connection and try again." |
| `Failed(RateLimited)` | "Couldn't check for updates." | — | — | Retry + Check | "GitHub is limiting requests right now. Try again later." |
| `Failed(BadFeed)` | "Couldn't read the update information." | — | — | Retry + Check | "The latest release is not in the expected format." |
| `Failed(MissingVersionCode)` | "Couldn't read the update information." | — | — | Retry + Check | "The latest release is missing its version code." |
| `Failed(MissingApk)` | "This release can't be installed." | — | — | Retry + Check | "The latest release has no APK file." |
| `Failed(MissingChecksum)` | "This release can't be verified." | — | — | Retry + Check | "The latest release has no checksum file, so it won't be installed." |
| `Failed(ChecksumMismatch)` | "The download didn't match its checksum." | — | — | Retry + Check | "The file was deleted. Try the download again." |
| `Failed(Download)` | "Couldn't download the update." | summary | — | Retry + Check | "Check your connection and try again." |
| `Failed(Write)` | "Couldn't save the update." | — | — | Retry + Check | "The phone may be low on storage." |
| `Failed(InstallSignatureMismatch)` | "Android couldn't install this update." | — | — | Check | "It's signed with a different key than the installed app. Export a backup in Backup & restore, uninstall the app, then install the update again." |
| `Failed(InstallFailed)` | "Android couldn't install this update." | — | — | Retry | Android's status name, e.g. `INSTALL_FAILED_INSUFFICIENT_STORAGE` |
| cancelled (back to `Available`) | "Version 1.4.0 is available (29.1 MB)." | summary | — | Check | "Download cancelled." |

- `UpToDate`/`Ahead`/`Failed` all allow a fresh manual check; `Checking`, `Downloading`, and
  `Ready` keep Check disabled to avoid concurrent work.
- "Checked just now" is derived from `last_checked_at`: `< 60 s` → "Checked just now",
  `< 60 min` → "Checked N minutes ago", otherwise "Checked N hours ago". Session only.

### 8.4 Settings menu dot

- When the state is `Ready` (an update is downloaded and waiting), the **Settings** item in the
  "+" menu shows a 6 px accent dot.
- `MenuItem` in `layout.slint` gains `in property <bool> dot: false;` rendered as an absolute
  accent circle at the trailing edge; `app.slint` passes `dot: root.update-attention`.
- No dot on tabs, the top bar, or the dashboard. No banner outside Settings (interview decision).
- The dot clears when the user installs, or when the version becomes current (e.g. after a manual
  check following an update).

### 8.5 Copy deck (exact strings)

| Purpose | String |
|---|---|
| Section title | `App updates` |
| Section caption | `Checks GitHub for new versions when the app opens` |
| Version row label | `Installed version` |
| Version row value | `1.4.0 (5)` (`version_name` + ` (` + code + `)`) |
| Check button | `Check for updates` |
| Install button | `Install update` |
| Retry button | `Retry` |
| Permission button | `Open settings` |
| Latest | `You're on the latest version.` |
| Ahead | `You're on a newer build than the latest release.` |
| Available | `Version {name} is available ({size}).` |
| Downloading | `Downloading update…` |
| Ready | `Update ready to install.` |
| Installing | `Opening Android's installer…` |
| Permission needed | `Allow installs from this app to continue.` |
| Data reassurance | `Your records stay on this device and aren't affected by an update.` |
| Desktop ready | `Update downloaded. Installing is only supported on Android.` |

Copy is English only, matching the rest of the UI, and never promises a silent install.

---

## 9. Requirement 6 — Versioning, docs, and release process

### 9.1 Version bump for this work

- `apps/mobile/manifest.yaml`: `version_name: 1.4.0`, `version_code: 5`.
- `apps/mobile/Cargo.toml`: `[package] version = "1.4.0"` and
  `[package.metadata.android] version_name = "1.4.0"`, `version_code = 5`.
- `apps/mobile/src/version.rs`: `VERSION_NAME = "1.4.0"`, `VERSION_CODE = 5` (guarded by the test
  in §4.3).
- Settings footer keeps reading `Boarding House Treasurer 1.4.0` from the shared constant.

### 9.2 README

- Add a feature bullet: "**Updates** — Settings checks GitHub for a newer release on launch,
  downloads it in the background, and hands it to Android's installer. Only one APK is kept, and
  the download is verified against a published SHA-256."
- Amend the offline statement, without rewriting the structure:
  - Intro: after "All records live **only on the device**…", add "The only network request the app
    makes is to GitHub, to check for and download a newer version."
  - Limitations: add "Updates require the internet; nothing about the treasurer's records is
    uploaded."
- Add the release command to the Commands block only if it is short; otherwise link `docs/installation.md`.

### 9.3 Release signing (prerequisite, not optional)

- A release keystore is created once and **kept forever**; losing or rotating it means every user
  must uninstall (and restore a backup) to update again.
- xbuild release builds are signed with that keystore instead of the debug key. The exact
  keystore flags/env names must be confirmed with `x build --help` during implementation and
  recorded in `docs/installation.md`.
- Because 1.3.0 installs were debug-signed, **1.4.0 cannot install over them**. The release notes
  must say: export a backup, uninstall the old build, install 1.4.0, restore the backup. After
  that, in-app updates work because the key is stable.
- `docs/installation.md` gains a short "Release builds and signing" subsection listing the keystore
  location convention, the environment variables, and the tag format.

### 9.4 CI release workflow

New `.github/workflows/release.yml`:

- Trigger: `on: push: tags: ["v*"]`.
- Steps: checkout → install Rust + `aarch64-linux-android` target → set up NDK + JDK 17 →
  install xbuild (git, per `docs/installation.md`) → decode the keystore from
  `ANDROID_KEYSTORE_BASE64` into a temp file → `x build --platform android --arch arm64 --release --format apk`
  with the keystore settings → locate the APK → rename to `boarding-house-treasurer-<version>.apk`
  → `sha256sum` into `boarding-house-treasurer-<version>.apk.sha256` → `gh release upload` both to
  the tag's release (create it if absent) → upload as workflow artifacts too.
- Required secrets: `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`,
  `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`.
- The workflow fails loudly if the tag does not match `v<name>+<code>` or if the built APK's
  `versionCode` differs from the tag's code. (Read it back with `aapt2 dump badging` or
  `apkanalyzer`; if neither is available in the runner image, skip with an explicit warning rather
  than silently passing.)
- No `pull_request` trigger, no publishing to any store.

### 9.5 Release checklist (manual, documented in `docs/installation.md`)

1. Bump the three version locations (§9.1); run `cargo test -p treasurer-core`.
2. `cargo check -p treasurer-mobile`; spot-check Settings in the desktop preview against a scratch
   feed.
3. Commit, tag `v1.4.0+5`, push the tag.
4. Confirm the workflow produced the APK + `.sha256` on the GitHub Release and that
   `/releases/latest` returns it.
5. Install 1.4.0 once by hand on a device, then verify the in-app updater on the *next* release.

---

## 10. Edge cases

### 10.1 Feed and version

- **Tag without `+code`** → `MissingVersionCode`; no download, no guess from `version_name`.
- **`version_name` with a `v` prefix** → stripped; `v1.4.0+5` and `1.4.0+5` both work.
- **Release code equal to installed** → `UpToDate`, not Available.
- **Release code lower than installed** (sideloaded newer build) → `Ahead`, with both codes shown.
- **Two `.apk` assets** → the first in API order wins; the chosen name is kept in the status
  detail. A wrong upload is still guarded by the checksum only if its sidecar matches.
- **No `.apk` asset** → `MissingApk`, nothing downloaded.
- **Sidecar missing** → `MissingChecksum`, nothing downloaded (never install unverified bytes).
- **Empty release body** → no summary line; the layout must not reserve a blank row.
- **Very long first line** → truncated to 160 chars with `…` in core, then `overflow: elide` in
  Slint.
- **Repo private or API blocked** → `Network`/`RateLimited`; the app must not embed a token.

### 10.2 Network and download

- **Offline at launch** → silent failure; Settings shows the failure with Retry when opened.
- **Network drops mid-download** → `Failed { Download }`, `.part` deleted, Retry available.
- **Slow network** → 20 s read timeout applies per request; progress keeps the UI honest.
- **Zero-byte or truncated response** → checksum mismatch (or Download failure) and deletion.
- **App killed mid-download** → stale `.part` removed on next startup.
- **Download folder unwritable** → `Failed { Write }` with the low-storage hint; nothing crashes.
- **User leaves Settings mid-download** → cancelled, `.part` removed, state `Available`.
- **User re-enters Settings** → auto-restart is **not** triggered by re-entry alone; a manual Check
  or Retry starts it again (prevents surprise bandwidth after a cancel).

### 10.3 Install

- **"Install unknown apps" off** → `NeedsPermission`; the APK is kept; returning re-checks.
- **Permission granted then revoked** → pre-flight runs on every install attempt.
- **User cancels the system prompt** → `Ready` again, APK kept.
- **Signature mismatch** → clear uninstall/restore instructions; the APK is deleted; Retry hidden.
- **Downgrade attempt** (feed regression) → cannot happen because only `code > installed` proceeds;
  `INSTALL_FAILED_VERSION_DOWNGRADE` is still mapped to `InstallFailed`.
- **Install succeeds but the process survives** → on the next UI tick the state sees the app was
  replaced only after a restart; the section keeps `Ready` until then. Acceptable: the installer
  kills the process in practice.
- **Battery saver / storage full** → `InstallFailed` with Android's status name.
- **Airplane mode at install time** → install still works; the APK is local.
- **Data** → untouched by any update path; the database path is stable and no migration runs.

### 10.4 Desktop preview

- No install button; no JNI types compiled.
- `TREASURER_UPDATE_FEED` can point at a local file so states can be forced without publishing.
- Progress and cancellation behave identically to Android.

---

## 11. Acceptance criteria

### 11.1 Check

1. With a release whose tag is `v1.4.1+6` and the app on 1.4.0 (5), launching the app silently
   reaches `Available` and starts the download without any dialog.
2. `v1.4.0+5` is `UpToDate`; `v1.4.0+4` is `Ahead` with both codes visible.
3. A tag without `+code` shows the missing-version-code detail and downloads nothing.
4. Offline at launch shows no banner anywhere; opening Settings shows the failure and Retry.
5. Tapping **Check for updates** while `UpToDate` re-runs the request and updates "Checked …".

### 11.2 Download

6. Progress shows a determinate bar plus `42% · 12.4 MB of 29.1 MB`-style text, and the numbers
   match the asset size.
7. Leaving Settings mid-download cancels it, deletes the partial file, and returns to
   `Available` with "Download cancelled."
8. A corrupt/truncated download is rejected by the SHA-256 check, both files are deleted, and the
   failure is shown.
9. A release with no `.apk` or no `.sha256` never starts a download.
10. At most one APK exists in the update folder after any sequence of checks and downloads.

### 11.3 Install

11. **Install update** on a device with "Install unknown apps" off shows the explanation and opens
    the app's system settings page; returning re-checks the permission.
12. With permission granted, the Android installer prompt appears for the downloaded APK and the
    update installs in place over the same signing key.
13. Cancelling the prompt leaves the app on 1.4.0 with `Ready`, and Install works again without
    re-downloading.
14. Installing an APK signed with a different key shows the signature-mismatch instructions, not a
    raw Android error code.
15. After a successful update, records are intact — boarders, bills, payments, settings, and
    currency all unchanged.

### 11.4 UI

16. The App updates section appears above Backup & restore, with the installed version, status,
    and exactly one primary action per state.
17. Long summaries elide; nothing overflows at 320 px; all tokens come from `Theme`.
18. Check is disabled during `Checking`, `Downloading`, and `Ready`.
19. The Settings item in the "+" menu shows a dot only while an update is `Ready`.
20. On desktop, the Ready state reads that installing is Android-only and no install button exists.

### 11.5 Release

21. Tagging `v1.4.0+5` produces a signed APK plus `.sha256` on the GitHub Release, and
    `/releases/latest` returns them.
22. README states the app contacts GitHub only for updates and that records are never uploaded.

---

## 12. Test plan

### 12.1 Automated (Rust core)

Run: `cargo test -p treasurer-core`.

New unit tests in `crates/treasurer-core/src/update.rs` (or `tests/update.rs`):

- `parse_release_tag`: `v1.4.0+5`, `1.4.0+5`, `v1.4` (reject), `v1.4.0` (reject),
  `v1.4.0+abc` (reject), extra whitespace (trim then accept).
- `is_newer`: greater/equal/lower; negative or zero codes rejected by the builder upstream.
- `pick_apk_asset`: single APK; APK after another asset; uppercase `.APK`; no APK; two APKs →
  first wins.
- `checksum_asset_name`: appends `.sha256`.
- `parse_sha256`: bare hex, `sha256sum`-style two-space output, uppercase hex → lowercase,
  63/65 chars rejected, unrelated text rejected.
- `checksum_matches`: case-insensitive equality; short/long strings rejected.
- `release_summary`: first non-empty line, collapses whitespace, truncates with `…` at the limit,
  empty body → empty string, never splits a multi-byte character.
- `format_size`: bytes/KB/MB/GB boundaries, one decimal, `150 MB` cap message path.
- `build_release_info`: full payload → fields populated; missing tag code / APK / sidecar →
  each typed error.

Version guard test in `apps/mobile/src/version.rs`: manifest values match `VERSION_NAME`/
`VERSION_CODE`.

No core test touches the network, the filesystem, or Android.

### 12.2 Compile checks

```bash
cargo check -p treasurer-mobile
```

Must pass for the host (desktop) target. The Android target is exercised by the release build;
`cargo check` for `aarch64-linux-android` is optional but recommended if the NDK is configured.

### 12.3 Desktop manual script

Point `TREASURER_UPDATE_FEED` at a local JSON file and walk every state:

1. Feed with `v1.4.1+6` → launch → open Settings → observe automatic download, progress, `Ready`.
2. Feed with `v1.4.0+5` → launch → `UpToDate`.
3. Feed with `v1.4.0+4` → `Ahead` with both codes.
4. Feed with a tag missing `+code`, then one with no APK, then one with no `.sha256` → each shows
   its own detail and downloads nothing.
5. Bad host in the env var → launch is silent; Settings shows Network failure and Retry.
6. Start a download and switch tabs → cancelled; re-enter Settings → `Available` with
   "Download cancelled."; Retry restarts.
7. Corrupt the sidecar hash → download rejected, files deleted, checksum-mismatch copy shown.
8. `Ready` on desktop → no install button; desktop-only message shown.

### 12.4 Android device checklist (required)

Build/install per `docs/installation.md`, then:

- Launch with a newer release published → auto-download runs; the app stays responsive.
- Leave Settings mid-download → cancelled, partial file gone (`adb shell run-as <pkg> ls cache/updates`).
- Tap Install with "Install unknown apps" off → explanation + system settings screen; grant and
  return → Install succeeds.
- Tap Install with permission on → system prompt appears; complete it; app is replaced by the
  new version; relaunch → `UpToDate` and all records intact.
- Cancel the prompt → `Ready` returns and Install works without re-download.
- Install an APK signed with a different key (simulate by building with the debug key) →
  signature-mismatch copy shown.
- Confirm both permissions are present in the installed manifest
  (`adb shell dumpsys package <pkg> | grep -i permission`).
- Confirm no dialog, banner, or navigation change happens on a silent launch failure in airplane
  mode.
- If no device is available, report that limitation explicitly rather than claiming an Android pass.

### 12.5 Definition of done

- Acceptance criteria in §11.1–§11.5 pass (Android items on a real device or explicitly reported
  unverified).
- `cargo test -p treasurer-core` and `cargo check -p treasurer-mobile` pass.
- Desktop script passes.
- Release workflow runs successfully on a test tag and produces APK + `.sha256`.
- Version is 1.4.0 / code 5 in all three locations, guarded by the test.
- README and `docs/installation.md` updated (updates + signing).
- No backup format, schema, split, payment, or undo behavior changes are included.

---

## 13. Implementation plan (suggested order)

1. **Version and permissions groundwork**
   - Add `version.rs`, use it in `start()`, bump manifest + Cargo metadata, add the guard test.
   - Add `INTERNET` + `REQUEST_INSTALL_PACKAGES`; verify the generated manifest.
2. **Feed core**
   - Add `crates/treasurer-core/src/update.rs` with the pure helpers and their tests; export it.
   - Decide and document the tag contract in `docs/installation.md`.
3. **Mobile update module**
   - `apps/mobile/src/update/mod.rs` (state machine, timer, cancellation), `feed.rs` (ureq request
     + `build_release_info`), `download.rs` (stream, progress, sha2, cleanup), `install.rs`
     (Android-only JNI, permission pre-flight).
   - Extend `platform.rs` with `updates_dir()` and startup cleanup.
4. **Wiring**
   - `App` fields for the shared state, `check_for_updates`, `apply_update_state`,
     `start_download`, `cancel_download`, `install_update`, `open_install_permission_settings`.
   - Call the launch check from `start()`; cancel from `set_screen` when leaving 4.
5. **Settings UI**
   - Add the section, status/summary/progress/action rendering, and the callbacks/properties in
     `app.slint`.
   - Add the `dot` property to `MenuItem` and feed it from `update-attention`.
6. **Docs and CI**
   - README disclosure + feature bullet; `docs/installation.md` signing section and checklist.
   - `.github/workflows/release.yml`.
7. **Verify**
   - Core tests, mobile check, desktop state sweep with `TREASURER_UPDATE_FEED`, Android checklist,
     test-tag workflow run.

---

## 14. Out of scope

- Automatic **silent** install (needs device-owner rights; Android always prompts).
- Delta/patch updates, split APKs, ABI-specific assets, or app bundles.
- Resumable downloads, download queues, or Wi-Fi-only rules.
- Desktop binary self-replacement.
- Release channels (beta/stable), staged rollout, or a minimum-supported-version gate.
- An in-app changelog/history screen or a "What's new" screen after updating.
- Auto-backup before installing, or any migration step (none is needed).
- A toggle to disable update checks.
- Store publishing (Play Store, F-Droid) or code signing for any other platform.
- Telemetry, crash reporting, or analytics of any kind.
- Changes to bills, payments, splits, change handling, undo, backup format, or the database schema.

---

## 15. Resolved interview decisions (audit)

- Feed: GitHub Releases **latest** for the existing repo.
- Platform: Android APK only; desktop is a preview and never installs.
- Auto behavior: check → automatic download → prompt to install.
- Timing: every launch plus a manual button; no opt-out toggle.
- UI: new App updates section above Backup & restore; version + size + one-line summary.
- Visibility: Settings only, with a dot on the Settings menu item.
- Signing: adopt a stable release keystore; surface signature mismatch with recovery steps.
- Permission: explain and deep-link to the app's "Install unknown apps" screen, then re-check.
- Integrity: SHA-256 from a sidecar `.sha256` asset; refuse unverified bytes.
- Data: reassurance only, no automatic backup.
- Download: determinate progress bar with percent and MB; cancel by leaving Settings; no resume.
- Retention: at most one APK, cleaned on startup, replaced/deleted per §6.4.
- Assets: first `.apk` in API order; explicit errors when absent.
- Comparison: integer `version_code` only.
- Failures: launch silent, Settings visible with Retry.
- Privacy: add INTERNET and disclose the single GitHub request in the README.
- Publishing: add a tag-driven GitHub Actions release workflow.

## 16. Assumptions and open questions

### 16.1 Assumptions (no further input needed)

- The repository can be read anonymously. If it stays private, an unauthenticated APK cannot read
  `/releases/latest`; the feed must then be a public endpoint (§16.2).
- `version_code` is the release tag's integer suffix; the tag is written by the release workflow.
- The updater only ever moves the app **forward**; there is no downgrade path.
- Android will kill the process during an in-place update, so no post-install callback is required.
- xbuild can produce a release-signed APK with a supplied keystore; if it cannot, the workflow must
  use another packaging path and the spec's install criteria still apply.
- The existing single-binary desktop build keeps compiling; platform gates live in `install.rs`.
- English copy only.

### 16.2 Open questions (close before implementing)

1. **Repository visibility.** If `Treasurer-Mini-System` is private, the in-app check needs either
   a public mirror/release-only repo, or a hosted `update.json`. Which one?
2. **Install mechanism.** `PackageInstaller` session (preferred, no Java class) versus
   `FileProvider` + `ACTION_VIEW`. Both need a device test; confirm which to implement first.
3. **Release keystore custody.** Confirm the keystore is created and backed up by the user, and
   that CI secrets can be set on the repo.
4. **Transition plan for 1.3.0 installs.** The first in-app update (1.4.0 → 1.4.1) works, but
   moving from today's debug-signed build requires one manual uninstall. Acceptable?
5. **Checksum sidecar naming.** `<apk>.sha256` is the assumption; confirm the workflow can emit
   exactly that name.
6. **GitHub API version header.** Pinning `2022-11-28` is recommended; confirm no proxy strips it.
7. **Background cancellation on Android.** If Slint's backend does not expose a pause callback,
   is screen-change cancellation alone acceptable for 1.4.0?
8. **Should the launch check respect a "no update" response at all?** Current design always shows
   "You're on the latest version" when opening Settings; no "stay quiet" preference exists.
