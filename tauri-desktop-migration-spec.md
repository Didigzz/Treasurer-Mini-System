# Tauri Desktop Migration Spec

Status: Draft
Scope: Add a Tauri desktop app for the Boarding House Treasurer, reusing the existing Bun/Hono/tRPC server, with a new web frontend.
Related: `boarding-house-treasurer-spec.md`

---

## 1. Goal

Ship a Windows desktop (later macOS/Linux) build of the treasurer app without rewriting the domain logic twice.

- Keep `apps/server` (Hono + tRPC on Bun) as the backend.
- Add `apps/desktop`: a Tauri v2 shell hosting a new React/Vite web frontend.
- Keep `apps/native` (Expo) working as-is for mobile.
- Share money/calculation logic and the tRPC contract across mobile and desktop.

Non-goals for this phase:

- Replacing the Expo mobile app.
- Removing the server in favor of Rust commands or `tauri-plugin-sql`.
- Authentication, multi-user sync, or cloud deployment.
- Code signing / notarization (tracked as a follow-up).

---

## 2. Current State (verified)

| Area | Reality |
|---|---|
| Mobile app | `apps/native` is Expo Router + React Native + Uniwind + HeroUI Native. |
| Mobile data access | Local SQLite via `expo-sqlite` and `drizzle-orm/expo-sqlite`. **No tRPC calls.** |
| Domain logic | `apps/native/src/features/treasurer/repository.ts` (~950 lines), `calculations.ts`, `money.ts`, `types.ts`, `backup.ts`, `queries.ts`. |
| Backend | `apps/server` is Hono with one tRPC route (`/trpc`) and `appRouter` containing only `healthCheck`. |
| DB package | `packages/db` targets libsql (`@libsql/client` + `drizzle-orm/libsql`), schema in `packages/db/src/schema/treasurer.ts`. |
| Server runtime | Bun; `apps/server/package.json` already has a `compile` script producing a single Bun binary. |
| Env | `varlock` codegen for `apps/server`, `apps/native`, `packages/db`. Server vars: `NODE_ENV`, `CORS_ORIGIN`, `DATABASE_URL`. |
| Monorepo | Turborepo + Bun workspaces (`apps/*`, `packages/*`). |

Consequence: the server is effectively empty. "Reuse the existing backend" means **porting the native repository into tRPC routers**. That port is the bulk of the work; the Tauri wrapper is comparatively small.

---

## 3. Target Architecture

```
┌─────────────────────────────┐
│ Tauri v2 shell (Rust)       │
│  - window / webview         │
│  - spawns server sidecar    │
│  - passes server URL + env  │
└──────────────┬──────────────┘
               │ HTTP + CORS
┌──────────────▼──────────────┐
│ Web frontend (React + Vite) │
│  - tRPC React Query client  │
│  - TanStack Router/React    │
│  - Tailwind v4              │
└──────────────┬──────────────┘
               │ /trpc
┌──────────────▼──────────────┐
│ Bun + Hono + tRPC server    │
│  - treasurer routers        │
│  - libsql (file DB)         │
└──────────────┬──────────────┘
               │
      <appDataDir>/treasurer.db
```

### Runtime modes

**Development**
- `bun dev:server` → Hono on `http://localhost:3000`, database `file:local.db`.
- `bun tauri dev` → Vite dev server on `http://localhost:1420`; Tauri `devUrl` points at Vite.
- Frontend tRPC URL: `http://localhost:3000/trpc`.
- CORS allows `http://localhost:1420`.

**Production**
- Server binary compiled with the existing `bun build --compile` script.
- Registered as a Tauri **sidecar** under `bundle.externalBin`.
- Rust `setup` hook:
  1. Resolve `app_data_dir`.
  2. Pick a free localhost port.
  3. Spawn the sidecar with `DATABASE_URL=file:<app_data_dir>/treasurer.db` and `CORS_ORIGIN=<tauri origin>`.
  4. Poll `GET /` until healthy (timeout + error surface).
  5. Expose the base URL to the webview via `invoke("server_url")` or a `window.__SERVER_URL__` injection.
- Sidecar is terminated on app exit.

> Note: the chosen "Tauri shell around the existing server" option still requires a sidecar in production, because a desktop app cannot assume a separately installed Bun process. Treat shell vs. sidecar as dev vs. prod halves of the same design.

### Database location

- Dev: repo-root `local.db` (existing behavior).
- Prod: Tauri app-data directory, e.g. on Windows `%APPDATA%/<identifier>/treasurer.db`.
- Migrations: run server-side on startup against the resolved `DATABASE_URL`, using `drizzle-kit migrate` output or a startup migrator. Do not depend on expo migrations for desktop.

---

## 4. Target Repository Layout

```
apps/
  desktop/                          # NEW
    package.json
    vite.config.ts
    tailwind.config.ts
    index.html
    src/
      main.tsx
      app.tsx
      trpc.ts                       # tRPC + React Query client
      routes/                       # dashboard, bills, boarders, payments, settings
      components/                   # web UI rebuilt from native screens
      env.ts                        # server URL resolution (dev vs tauri)
    src-tauri/
      Cargo.toml
      tauri.conf.json
      build.rs
      capabilities/default.json
      icons/
      src/
        main.rs                     # sidecar spawn + health check + URL command
  server/                           # EXISTING, grown
    src/
      index.ts
      context.ts
      services.ts
  native/                           # EXISTING, unchanged (Phase 1 import updates only)
packages/
  api/
    src/
      index.ts                      # tRPC init
      context.ts
      routers/
        index.ts                    # combine routers
        settings.ts
        bill-types.ts
        boarders.ts
        bills.ts
        payments.ts
        backup.ts                   # optional import/export
      treasurer/
        calculations.ts             # MOVED from apps/native
        money.ts                    # MOVED from apps/native
        types.ts                    # MOVED from apps/native
        mapping.ts                  # row -> DTO helpers
  db/                               # EXISTING schema + relations
```

`apps/desktop` is added to Bun workspaces and Turborepo pipelines (`dev`, `build`, `check-types`).

---

## 5. Phase 0 — Prerequisites

1. Install Rust toolchain (`rustup`, MSVC Build Tools) and ensure WebView2 is present (default on current Windows).
2. Confirm Bun 1.3.x and `@tauri-apps/cli` v2 compatibility.
3. Add `apps/desktop` to:
   - `package.json` workspaces (already covered by `apps/*`).
   - `turbo.json` task graph (`dev`, `build`, `check-types`, optionally `dev:desktop`).
4. Add root scripts:
   - `dev:desktop`: `turbo run dev -F desktop --`
   - `build:desktop`: `turbo run build -F desktop --`
   - `tauri`: `bun --cwd apps/desktop tauri`

Acceptance: empty Tauri window opens in dev.

---

## 6. Phase 1 — Extract Shared Domain Logic

Move pure, platform-agnostic code out of `apps/native`:

| Source | Destination | Notes |
|---|---|---|
| `apps/native/src/features/treasurer/calculations.ts` | `packages/api/src/treasurer/calculations.ts` | Pure functions; no RN imports. |
| `apps/native/src/features/treasurer/money.ts` | `packages/api/src/treasurer/money.ts` | Pure formatting/parsing. |
| `apps/native/src/features/treasurer/types.ts` | `packages/api/src/treasurer/types.ts` | Shared DTOs. |
| `apps/native/src/features/treasurer/__tests__/*` | next to moved files | Keep Bun tests green. |

Keep `apps/native` imports pointed at `@my-better-t-app/api/treasurer/*` (add an `exports` entry if needed).

Do **not** move `repository.ts` or `backup.ts` yet; they are platform-coupled (`expo-sqlite`, `expo-crypto`, `expo-file-system`).

Acceptance:
- `bun check-types` passes for `apps/native`, `packages/api`.
- Existing tests (`calculations.test.ts`, `money.test.ts`) pass from the new location.
- Mobile app behavior unchanged.

---

## 7. Phase 2 — Build tRPC Routers

Port `apps/native/src/features/treasurer/repository.ts` from expo-sqlite to `packages/db` (libsql) inside `packages/api`.

### 7.1 Mechanical substitutions

| Native | Server |
|---|---|
| `import { db, withTransaction } from "@/src/db"` | `ctx.db` from `createDb(ENV)` in `apps/server/src/services.ts` |
| `sqlite.withTransactionAsync` | `db.transaction(async (tx) => ...)` |
| `randomUUID()` from `expo-crypto` | `crypto.randomUUID()` |
| `getSetting`/`setSetting` from `@/src/db/seed` | server-side settings helpers in `packages/api/src/treasurer/settings.ts` |
| `expo-file-system`, `expo-sharing`, `expo-document-picker` (in `backup.ts`) | defer; backup/import is a separate router |

### 7.2 Router contract (1:1 with repository exports)

**`settings`**
- `get` → `getAppSettings`
- `setCurrency` → `setCurrencyCode(currencyCode)`
- `setLastBackupAt` → `setLastBackupAt(value)`

**`billTypes`**
- `list` → `listBillTypes`
- `create` → `createBillType(name)`
- `rename` → `renameBillType(id, name)`
- `setArchived` → `setBillTypeArchived(id, isArchived)`
- `getById` → `getBillTypeById(id)`

**`boarders`**
- `listWithBalances` → `listBoardersWithBalances`
- `active` → `getActiveBoarders`
- `pickerList` → `getAllActiveBoardersForPicker`
- `get` → `getBoarder(id)` / `getBoarderRow(id)`
- `create` → `createBoarder(name)`
- `rename` → `renameBoarder(id, name)`
- `setArchived` → `setBoarderArchived(id, isArchived)`
- `ledger` → `getBoarderLedger(id)`

**`bills`**
- `dashboard({ year, month, includeArchived })` → `getDashboard`
- `get(id)` → `getBillDetail`
- `create(input: CreateBillInput)` → `createBill`
- `update(id, input: CreateBillInput)` → `updateBill`
- `setArchived(id, isArchived)` → `setBillArchived`
- `updateMeta(...)` → `updateBillMeta`
- `findDuplicates(...)` → `findDuplicateBills`

**`payments`**
- `context(shareId)` → `getSharePaymentContext`
- `record(input: RecordPaymentInput)` → `recordPayment`
- `void(paymentId, reason)` → `voidPayment`
- `markChangeReturned(...)` → `markChangeReturned`

**`backup` (optional in this phase)**
- `export` → returns the JSON payload currently produced by `createBackupPayload` + `parseBackupJson`.
- `import` → validates with `backupSchema` and restores into the server DB.
- Do **not** port `exportBackup`, `pickBackupFile`, or `restoreBackup` as-is; face those through Tauri dialog/fs plugins on desktop.

### 7.3 Validation

- Use `zod` schemas for procedure inputs, mirroring the existing TypeScript interfaces (`CreateBillInput`, `RecordPaymentInput`, etc.).
- Keep procedure outputs equal to the current repository return types so `queries.ts` can be reused.
- Preserve invariants currently enforced in the repository: share summing, duplicate checks, archive rules, payment/change calculations.

### 7.4 Server wiring

1. Keep `apps/server/src/index.ts` mounting `/trpc`.
2. Increase or scope CORS:
   - dev: `http://localhost:1420`
   - prod: Tauri origin (`http://tauri.localhost` on Windows, `tauri://localhost` on macOS/Linux)
   - Consider allowing comma-separated values in `CORS_ORIGIN`.
3. Ensure `trpcServer` context exposes `db` from `services.ts`.
4. Add an `/health` (or reuse `/`) response that includes readiness, for sidecar polling.

Acceptance:
- Integration test: create boarder → create bill type → create bill with splits → record payment → dashboard totals match expected values, all against a temp libsql file.
- `bun check-types` passes.
- Existing calculations tests still pass.

---

## 8. Phase 3 — Scaffold and Build the Web Frontend

### 8.1 Stack

- React 19 + TypeScript + Vite.
- Tailwind CSS v4.
- `@tanstack/react-query` v5.
- `@trpc/tanstack-react-query` v11 (same version family as mobile).
- Routing: TanStack Router (or React Router) — choose one and record it here.
- Icons: `lucide-react`.
- UI: rebuild native screens with plain Tailwind plus a small local component set. HeroUI Native and `tailwind-variants` RN usage do not carry over directly.

### 8.2 Screens to port

From `apps/native/app`:

| Native route | Desktop route |
|---|---|
| `(tabs)/index.tsx` | `/` dashboard |
| `(tabs)/boarders.tsx` | `/boarders` |
| `(tabs)/settings.tsx` | `/settings` |
| `bills/new.tsx` | `/bills/new` |
| `bills/[id].tsx` | `/bills/:id` |
| `boarders/[id].tsx` | `/boarders/:id` |
| `bill-types.tsx` | `/bill-types` |
| `payments/new.tsx` | `/payments/new` (modal or route) |

Reuse `apps/native/src/features/treasurer/queries.ts` almost verbatim: replace direct repository imports with tRPC proxy calls while keeping the same hook names and query keys.

### 8.3 Client bootstrap

- `src/trpc.ts`: build `createTRPCContext` / `trpc` React hooks and an `httpBatchLink`.
- `src/env.ts`:
  - dev: `import.meta.env.VITE_SERVER_URL ?? "http://localhost:3000"`.
  - prod: `await invoke<string>("server_url")`.
- Replace `DatabaseBootstrap` with a `ServerBootstrap` gate that waits for health and then renders the router. Preserve the existing loading/error/retry UX.

Acceptance:
- All listed screens render and mutate data through tRPC in `bun tauri dev`.
- `bun check-types` and `bun build` pass for `apps/desktop`.

---

## 9. Phase 4 — Tauri Production Wiring

### 9.1 Sidecar

1. Add a `build:sidecar` script in `apps/desktop` that runs the server `compile` script and copies the binary with the Tauri target-triple suffix, e.g.:
   - `server-x86_64-pc-windows-msvc.exe`
   - `server-aarch64-apple-darwin`
   - `server-x86_64-unknown-linux-gnu`
2. Register in `src-tauri/tauri.conf.json`:
   ```json
   {
     "bundle": {
       "externalBin": ["binaries/server"]
     }
   }
   ```
3. Grant shell/sidecar permissions in `capabilities/default.json`.

### 9.2 Rust setup

In `src-tauri/src/main.rs`:
- Resolve the app data directory.
- Choose a free port bound to `127.0.0.1`.
- Spawn the sidecar with:
  - `DATABASE_URL=file:<path>/treasurer.db`
  - `CORS_ORIGIN=<webview origin>`
  - `NODE_ENV=production`
- Store the child handle for shutdown.
- Poll `GET /` until `OK`, with a bounded timeout and a clear failure dialog.
- Implement `#[tauri::command] fn server_url() -> String`.

### 9.3 Window/bundle config

- `build.devUrl`: `http://localhost:1420`.
- `build.frontendDist`: `../dist`.
- Product name: `Boarding House Treasurer`.
- Identifier: keep consistent with mobile or use `com.anonymous.mybettertapp.desktop`.
- Icons generated from existing assets.
- `.gitignore`: `apps/desktop/src-tauri/target/`, generated sidecar binaries.

Acceptance:
- `bun tauri build` produces a Windows installer.
- Installed app opens, spawns the sidecar, creates the DB in app-data, and completes a create-bill + record-payment flow.
- Closing the app terminates the sidecar process.

---

## 10. Data Migration & Compatibility

- Mobile and desktop start with independent databases.
- Reuse `apps/native/src/features/treasurer/backup.ts` as the interchange format:
  - Mobile export already exists.
  - Add a desktop import that accepts the same JSON and writes through server procedures or a dedicated import transaction.
- Do not attempt to read the mobile `treasurer.db` file directly; expo-sqlite paths and WAL state are not a supported coupling.

---

## 11. Testing Strategy

| Layer | Test |
|---|---|
| Pure logic | Move and run `calculations.test.ts`, `money.test.ts` in `packages/api`. |
| Routers | Integration tests against a temp libsql file: CRUD, splits, payments, change, archive, duplicate detection. |
| Frontend | Smoke tests for hooks/components; optional Playwright against Vite dev server. |
| Tauri | Manual smoke checklist: launch, health gate, create bill, record payment, restart persistence, clean shutdown. |
| CI | `turbo run check-types`, `turbo run build`, router integration tests. Rust `cargo check` if Rust is available in CI. |

Existing test command (native tests use Bun), verify with `bun test` scoped to the moved test files.

---

## 12. Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Server router port diverges from mobile repository | Two behaviors over time | Phase 1 extraction + 1:1 router mapping; integration tests. |
| CORS/origin mismatch in Tauri prod | App cannot reach server | Allow both `tauri://localhost` and `http://tauri.localhost`; verify on Windows first. |
| Sidecar binary not bundled / wrong target triple | Installed app fails | Build script that renames per target; assert in CI/release checklist. |
| Fixed port collisions | Server fails to start | Rust picks a free port and passes it to the frontend. |
| App-data DB path errors | Data loss/false empty state | Resolve path once in Rust, pass explicitly, log on startup. |
| Large native files (repository ~950 lines) ported by hand | Regressions | Port function-by-function, test each router as it lands. |
| Uniwind/HeroUI assumptions in screens | Visual/behavior drift | Treat desktop as a fresh UI implementation sharing only hooks and logic. |
| Rust toolchain absent on contributors' machines | Cannot build desktop | Document prerequisites in README; keep `check-types`/web build working without Rust. |

---

## 13. Decisions

1. **Native stays offline-first.** `apps/native` keeps direct `expo-sqlite` access; only the desktop app talks to the tRPC server. Both share the domain logic in `packages/api/src/treasurer`.
2. **Manual backup migration for v1.** Moving between Expo and desktop is an explicit JSON export/import; no automatic one-time migration is built.
3. **TanStack Router.** The desktop app uses TanStack Router with hash history, matching the generated route types and search-param validation.
4. **Windows-first v1.** macOS/Linux builds are possible from the same source, but only Windows is verified and claimed.
5. **Keep `libsql`.** The sidecar keeps the existing `@libsql/client` stack. Because a compiled Bun binary cannot read `.sql` files, schema creation is handled by the idempotent startup migrator in `packages/db/src/migrate.ts`.

Implementation notes:

- Varlock's runtime auto-loader crashes Bun 1.3.14 on Windows, so the server reads `process.env` with development defaults; Varlock remains the source for `.env.schema` and codegen.
- `bun build --compile --bytecode` rejects top-level `await`; the sidecar compile omits `--bytecode`.
- The webview health gate polls `/health` and shows a retry card, which covers the sidecar readiness check; Rust exposes only the chosen URL and kills the child on exit.
- libsql loads its N-API addon through a dynamic `require` that `bun build --compile` cannot embed. `build:sidecar` stages the host addon, Tauri bundles it under `native_modules`, and Rust copies it to `<app-data>/node_modules` before spawning the sidecar with that folder as its working directory.
- The Windows bundle targets MSI (`targets: ["msi"]`), which is the installer verified for v1.

---

## 14. Definition of Done (desktop v1)

- [x] Phase 1 shared logic extracted; native type-check passes. (Expo export could not be run: Varlock's Expo Metro integration crashes Bun 1.3.14 on Windows, independent of this change.)
- [x] Treasurer tRPC routers implemented with integration tests.
- [x] `apps/desktop` implements all listed screens against tRPC.
- [x] Tauri dev and production builds work on Windows (MSI installer produced).
- [x] Sidecar spawns, health-checks, and shuts down cleanly; verified in the release build.
- [x] Database persists in app-data; `treasurer.db` created at `%APPDATA%\com.boardinghouse.treasurer`.
- [x] Backup export/import round-trips through the shared JSON format and integration test.
- [x] `turbo run check-types` and `turbo run build` pass.
