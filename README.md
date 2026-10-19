# Boarding House Treasurer

A phone-first, offline record-keeping app for one boarding-house treasurer, built on a
[Better-T-Stack](https://github.com/AmanVarshney01/create-better-t-stack) monorepo
(Expo / React Native, Hono, tRPC, Drizzle).

The treasurer can keep a boarder list, manage bill types, split monthly shared bills
equally or with custom amounts, record payments and change to return, follow balances per
month and per boarder, and export/restore everything as a JSON file.

All records live **only on the phone** in an app-private SQLite database. The app does not
use accounts, cloud sync, analytics, or the bundled tRPC server.

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

Start the Expo development server:

```bash
bun run dev:native
```

Then press `a` for Android or `i` for iOS, or run `bun run android` / `bun run ios` from
`apps/native`. `expo-sqlite`, `expo-document-picker`, and `expo-sharing` work in a
development build; see
[Expo development builds](https://docs.expo.dev/develop/development-builds/introduction/)
for device setup.

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

Pure money and calculation logic is tested with Bun's built-in test runner:

```bash
bun test apps/native/src/features/treasurer
```

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
    src/features/treasurer # Domain logic, repository, queries, backup, components
    drizzle/               # Generated SQL migrations + entrypoint
  server/                  # Hono + tRPC API (not used by the treasurer app)
packages/
  api/                     # tRPC router
  db/                      # Drizzle schema shared by server and native
  config/                  # Shared TypeScript configuration
```

## Available Scripts

- `bun run dev`: Start all applications in development mode
- `bun run dev:native`: Start the Expo development server
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

Bun's automatic env loading is disabled in `bunfig.toml`; the framework integration or
server bootstrap loads Varlock. Node deployments must include Varlock and its dependencies
alongside the app schema.

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
