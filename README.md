# Boarding House Treasurer

A phone-first, offline record-keeping app for one boarding-house treasurer, built on a
[Better-T-Stack](https://github.com/AmanVarshney01/create-better-t-stack) monorepo
(Expo / React Native, Hono, tRPC, Drizzle).

The treasurer can keep a boarder list, manage bill types, split monthly shared bills
equally or with custom amounts, record payments and change to return, follow balances per
month and per boarder, and export/restore everything as a JSON file.

All records live **only on the device** in a local SQLite database. The Expo app reads
SQLite directly; the Windows desktop app runs the bundled Hono/tRPC server against a
libsql file in its app-data folder. Neither app uses accounts, cloud sync, or analytics.

See [boarding-house-treasurer-spec.md](boarding-house-treasurer-spec.md) for the full
product and implementation spec.

## Features

- **Offline-first storage** — `expo-sqlite` + Drizzle ORM, database `treasurer.db`.
- **Boarders** — add, rename, archive, restore; history is preserved.
- **Bill types** — seeded Water, Electricity, Rent, Wi-Fi, Other; add, rename, archive,
  restore. Old bills keep the name they were created with.
- **Monthly bills** — calendar-month period, optional due date and note, duplicate
  warning, archive/unarchive.
- **Equal or custom splits** — leftover centavos go to the first boarders in selection
  order; custom shares must add up to the bill total.
- **Payments** — partial payments, overpayments, editable date/time, live balance and
  change preview.
- **Change tracking** — change is tracked per payment and marked returned separately;
  a settled bill can still have change outstanding.
- **Corrections** — void a payment with a required reason; history is retained.
- **Backup** — export all data as JSON through the share sheet; restore replaces all
  local data after confirmation and validation.
- **Dashboard** — selected-month totals, bill progress, and boarder balances with
  All / Unpaid / Paid / Change due filters.

## Getting Started

Install dependencies from the repository root:

```bash
bun install
```

### Desktop app (Windows)

The desktop app uses Tauri v2 with a React/Vite frontend and ships the Bun server as a
sidecar. Prerequisites: Rust (MSVC toolchain), Visual Studio C++ build tools, and
WebView2 (installed by default on Windows 10/11).

```bash
bun run tauri:dev     # builds the sidecar, starts Vite, opens the desktop window
bun run tauri:build   # release build + installer under apps/desktop/src-tauri/target
```

To iterate on the UI in a normal browser, run the API and Vite separately:

```bash
bun run dev:server    # http://127.0.0.1:3000
bun run dev:desktop   # Vite only → http://localhost:1420
```

The desktop database lives at `%APPDATA%\com.boardinghouse.treasurer\treasurer.db`.
Use Settings → Export backup before moving computers; import the same JSON on the other
machine. Moving between Expo and desktop is a manual backup export/import.

### Expo app

Start the Expo development server:

```bash
bun run dev:native
```

Then press `a` for Android or `i` for iOS, or run `bun run android` / `bun run ios` from
`apps/native`. `expo-sqlite`, `expo-document-picker`, and `expo-sharing` work in a
development build; see
[Expo development builds](https://docs.expo.dev/develop/development-builds/introduction/)
for device setup.

### Running on an Android phone

Every native module the app uses is bundled in **Expo Go** for SDK 58, so the fastest
way to run it on a phone is Expo Go with no Android build at all:

1. Install **Expo Go** from the Play Store (it must support SDK 58).
2. Put the phone and this computer on the same Wi-Fi network.
3. From the repository root, run `bun run dev:native` and leave it running. The terminal
   prints a QR code (the URL looks like `exp://192.168.x.x:8081`).
4. Scan the QR code with Expo Go (Android 11+ can scan it from the camera/quick settings;
   otherwise use *Scan QR code* inside Expo Go). The app bundles once, then opens.

The treasurer data is stored on the phone in the app's private SQLite database, so the
phone works offline after the first load. Stop the dev server and reopen Expo Go from the
recent list while the server is down and it will fail to load — that is expected for a
development build; it does not affect the stored records.

If the phone cannot reach `192.168.x.x:8081`, check that Windows Firewall allows Node
(private networks), that both devices are on the same subnet, and that the Wi-Fi network
does not use client isolation (guest networks usually do).

To run a **standalone app** that no longer needs the dev server, build an installable APK
instead (this machine already has JDK 17 and the Android SDK, so no EAS account is
needed):

```bash
cd apps/native
bunx expo prebuild --platform android   # generates the native android/ project
cd android && ./gradlew assembleRelease # APK in android/app/build/outputs/apk/release/
```

Or, with a USB-connected phone and developer mode enabled, `bun run android` from
`apps/native` builds and installs a debug development build in one step. The generated
`android/` and `ios/` folders are gitignored; they are build output, not source.

### Local database and migrations

The native app owns its schema and migrations:

```bash
cd apps/native
bun run db:generate   # regenerate SQL migrations after schema changes
bun run check-types
```

- Schema: `packages/db/src/schema/treasurer.ts` (generated SQL in `apps/native/drizzle`).
- Migrations run at startup through the Drizzle `expo-sqlite` migrator; the app blocks
  on a loading screen until they succeed and shows a retry screen on failure.
- Seed data (default settings and bill types) is inserted idempotently after migration.
- Uninstalling the app or clearing app data permanently deletes all records unless a
  backup was exported.

### Tests

Shared treasurer logic and the tRPC routers are tested with Bun's built-in test runner:

```bash
bun test --cwd packages/api
```

This covers money parsing, equal/custom splits, payment and change calculations, and
integration tests against an in-memory libsql database.

## Project Structure

```
apps/
  native/                  # Expo app (treasurer UI + local SQLite)
    app/                   # Expo Router routes
      (tabs)/              # Dashboard, Boarders, Settings
      bills/               # Create/edit bill, bill detail
      boarders/[id].tsx    # Boarder ledger
      payments/new.tsx     # Record payment modal
      bill-types.tsx       # Bill type management
    src/db/                # SQLite singleton, migrations gate, seed
    src/features/treasurer # Native repository, queries, components
    drizzle/               # Generated SQL migrations + entrypoint
  desktop/                 # Tauri v2 + React/Vite desktop app
    src/                   # Routes, screens, tRPC hooks, UI primitives
    src-tauri/             # Rust shell, sidecar config, icons, installer
    scripts/               # build-sidecar.ts (Bun compile + triple suffix)
  server/                  # Hono + tRPC API used by the desktop sidecar
packages/
  api/                     # Shared treasurer domain + tRPC routers + tests
  db/                      # Drizzle schema, startup migrator, libsql client
  config/                  # Shared TypeScript configuration
```

## Available Scripts

- `bun run dev`: Start all applications in development mode
- `bun run dev:native`: Start the Expo development server
- `bun run dev:desktop`: Start the desktop Vite dev server in a browser
- `bun run tauri:dev`: Build the sidecar and open the desktop app in Tauri
- `bun run tauri:build`: Build the Windows release app and installer
- `bun run dev:server`: Start only the Hono server
- `bun run check-types`: Check TypeScript types across all apps
- `bun run build`: Build all applications
- `bun run db:push`: Push schema changes to the server database
- `bun run db:generate`: Generate server database client/types
- `bun run db:migrate`: Run server migrations
- `bun run db:studio`: Open Drizzle Studio
- `bun run db:local`: Start the local server SQLite database

## Environment Configuration

Each app owns its environment schema in `.env.schema`. Varlock generates `src/env.ts`
during installation; run `bun run env:generate` after changing a schema. Commit schemas,
and keep secrets in ignored env files or your deployment platform.

Import the generated `ENV` accessor in application code. Shared database and auth packages
receive configuration or initialized clients from the application. See
[Varlock's monorepo guide](https://varlock.dev/guides/monorepos/).

The server reads `process.env` directly at startup (`apps/server/src/env.server.ts`) and
falls back to development defaults. This avoids Varlock's runtime auto-loader, which
crashes Bun 1.3.x on Windows; Varlock is still used for `.env.schema` and codegen. The
packaged desktop sidecar receives `PORT`, `DATABASE_URL`, and `CORS_ORIGIN` from the Tauri
shell, so no env file is needed in the installer.

Run standalone Node/Bun tools that use Varlock from the owning app directory so they load
that app's schema and env files. `env:generate` only generates TypeScript files; it does
not initialize environment values in a subsequent command.

## Backup Format

Exports are unencrypted JSON with `format: "boarding-house-treasurer"`, `version: 1`,
and a `data` object containing settings, bill types, boarders, bills, bill shares, and
payments. Timestamps are ISO strings. Restoring validates the file, checks referential
integrity and that each bill's shares equal its total, then replaces all local data in one
transaction. Restore never merges.

## Limitations

- Single boarding house, single treasurer, no login or app lock.
- No cloud sync or online backup; export a JSON backup regularly.
- Amounts are stored in integer minor units and displayed in the selected currency without
  conversion when the currency setting changes.
- Bill money fields lock after the first active payment; corrections are made by voiding a
  payment and recording a new one.
