# Boarding House Treasurer — Implementation Spec

**Status:** Draft for implementation
**Date:** October 19, 2026
**Request:** A small, phone-first local record-keeping app for a boarding-house treasurer to track boarders, shared bills, shares, in-person payments, and change still to be returned.

---

## 1. Summary

Build a single-treasurer, single-boarding-house app inside the existing Better T Stack project. The app will:

- Keep a reusable list of boarders with active/archived status.
- Manage bill types (Water, Electricity, Rent, Wi-Fi, Other, plus custom types).
- Create one bill per bill type and calendar month.
- Split a bill equally or with custom per-boarder amounts.
- Snapshot boarder names and shares on each bill so history survives renames and archiving.
- Record in-person payments, including partial payments and overpayments.
- Track unpaid balances and change to return separately.
- Keep an append-only, readable payment history with void-with-reason corrections.
- Export and restore all data as a JSON file.
- Work entirely offline on the phone, with no accounts, cloud sync, or analytics.

The first version is deliberately narrow: reliable bill/payment record keeping, not accounting software.

---

## 2. Existing Project Inspection

### 2.1 Stack found

The repository is a Better T Stack monorepo created with:

```text
bun create better-t-stack@latest my-better-t-app \
  --frontend native-uniwind --backend hono --runtime bun \
  --database sqlite --orm drizzle --api trpc --auth none \
  --payments none --addons turborepo --examples none \
  --db-setup none --web-deploy none --server-deploy none \
  --git --package-manager bun --install
```

Relevant versions and pieces:

| Area | Current setup |
| --- | --- |
| Monorepo | Bun 1.3.14 workspaces + Turborepo 2.x |
| Mobile | Expo SDK 58, React Native 0.88, React 19.3, Expo Router |
| UI | HeroUI Native 1.0.10, Uniwind 1.12, Tailwind CSS 4, Reanimated |
| Forms/keyboard | No form library; `react-native-keyboard-controller` is available |
| Server | Hono 4.13 on Bun, tRPC 11, `@hono/trpc-server` |
| Database | SQLite/libSQL via `@libsql/client`, Drizzle ORM 1.0.0-rc.4 |
| Migrations | Drizzle Kit 1.0.0-rc.4; `packages/db/src/migrations` is empty |
| API client | TanStack Query v5 + `createTRPCOptionsProxy` |
| Env | Varlock, generated `src/env.ts` files |
| Auth | None |
| Tests | No test runner or test files configured |
| App data today | Server-side SQLite only; no on-device database |

Key files inspected:

- `apps/native/app/_layout.tsx` — root providers: QueryClient, GestureHandler, KeyboardProvider, theme, HeroUI.
- `apps/native/app/(drawer)/...` — generated drawer + tab demo screens.
- `apps/native/app/(drawer)/index.tsx` — tRPC health-check demo.
- `apps/native/utils/trpc.ts` — tRPC client pointed at `EXPO_PUBLIC_SERVER_URL`.
- `apps/server/src/index.ts` — Hono server with `/trpc/*` and `GET /`.
- `packages/api/src/routers/index.ts` — only `healthCheck` exists.
- `packages/db/src/schema/index.ts` — empty schema export.
- `packages/db/src/index.ts` — libSQL Drizzle client for the server.
- `packages/db/src/relations.ts` — `defineRelations(schema)`.
- `apps/native/package.json`, `apps/server/package.json`, `packages/db/package.json` — dependency and script inventory.
- `apps/native/app.json` — app name `my-better-t-app`, typed routes, React Compiler.
- `apps/native/babel.config.js`, `apps/native/metro.config.js` — Varlock, Uniwind, and Reanimated wrappers.

### 2.2 Important consequences

- The generated app has **no on-device persistence**. Its data path is phone → tRPC → server SQLite. That does not satisfy “data on my phone, works when I close and reopen the app” without running the server.
- `expo-sqlite` is not installed, but Drizzle already ships an `expo-sqlite` driver and migrator in the installed `drizzle-orm` package. On-device storage is a genuine missing capability, not a replacement of the stack.
- The UI kit already provides `Input`, `TextArea`, `Select`, `Checkbox`, `Switch`, `Dialog`, `Alert`, `Button`, `Card`, `Chip`, `Tabs`, `Surface`, `Spinner`, `Separator`, and toast primitives. No UI library should be added.
- There is no test runner. Bun’s built-in test runner can cover pure domain logic without adding a dependency.

### 2.3 Stack boundary

The implementation will:

- Keep Hono, tRPC, the server package, the libSQL client, and all existing scripts intact.
- Add on-device SQLite as the app’s real persistence layer.
- Stop using the tRPC health check on app startup. The core app must not require the server or a network connection.
- Reuse the existing Drizzle schema package for table definitions, but import only the schema subpath from the native app so Metro does not pull in `@libsql/client`.
- Not delete or reconfigure the server merely because the phone no longer calls it.

---

## 3. Decisions Locked In By The Interview

| Area | Decision |
| --- | --- |
| Data location | On the phone using `expo-sqlite` + existing Drizzle ORM. Offline-first. Server not required. |
| Billing period | Calendar month + year (`October 2026`). |
| Currency | Philippine peso (`PHP`, ₱) default, changeable in Settings. Two-decimal minor units. |
| Bill types | Seed Water, Electricity, Rent, Wi-Fi, Other; allow add, rename, archive, restore. |
| Boarder selection | All active boarders selected by default when creating a bill. |
| Corrections | Void the wrong payment with a required reason, then record a new payment. No in-place money edits. |
| Change tracking | Per payment: each overpayment creates its own change record and returned flag. |
| Bill settlement | Automatic when every share is covered. Unreturned change does not block settlement. |
| Bill editing | Money fields lock after the first active payment. Due date and note remain editable. |
| Partial payments | Supported; multiple payments accumulate. |
| Duplicate bills | Warn with details and require “Create anyway”; do not hard-block. |
| Dashboard | Summary + bill cards; tapping a card opens bill detail with the per-bill boarder list and payments. Also show a period-wide boarder balance list with filters. |
| Archiving a boarder with debt | Allowed with a warning; debt remains visible and payable. |
| Backup | Complete JSON file exported through the phone share sheet; restore by picking a file. |
| Restore behavior | Validate, show a summary, warn, then replace all current data in a transaction. |
| Platforms | Android and iOS. No web target for v1. |
| Period navigation | Open on the current month; previous/next arrows; month/year picker; remember last viewed month for the session. |
| Dashboard filters | Paid = balance zero; Unpaid = any balance (including partial); Change due = unreturned change, independent of paid status. |
| New payment while change is outstanding | Keep change separate, show a reminder, never auto-net. |
| Equal-split remainder | Give one extra minor unit to the first boarders in the order they were selected; show exact shares before saving. |
| Bill archiving | Hide from the default list/totals, keep all records, allow unarchive. |
| Navigation | Bottom tabs: Dashboard, Boarders, Settings; stack screens for detail/forms. |
| Payment date | Default to now, editable. |
| Dashboard totals | Selected calendar month only. |
| Custom share mismatch | Block saving until shares exactly equal the bill total. |
| Payment amount prefill | Prefill the remaining balance. |
| Due dates | Optional; show Overdue only when the date has passed and a balance remains. |
| Duplicate boarder names | Allow with a warning. |
| Change return | Mark the whole change for a payment returned at once; no partial change returns. |

---

## 4. Scope

### 4.1 In scope

1. Boarder list, add, rename, archive, restore.
2. Bill type list, seed, add, rename, archive, restore.
3. Bill creation with calendar period, total, optional due date, optional note, type, included boarders, and equal/custom split.
4. Bill snapshot of type name, boarder names, and shares.
5. Duplicate bill warning.
6. Dashboard for the selected month with totals, bill progress, and period-wide boarder balances.
7. Bill detail with per-bill boarder shares, payment progress, filters, and payment history.
8. Boarder detail with all-time bill/payment ledger.
9. Payment recording with partial payments, overpayments, and live balance/change preview.
10. Change-return recording per overpaying payment.
11. Void payment with required reason, confirmation, and retained history.
12. Currency setting with no conversion of existing amounts.
13. JSON backup export and restore.
14. Local-only storage, offline operation, documentation of data-loss limitations.
15. Unit tests for money, splits, payment allocation, totals, filters, and backup validation.
16. README updates.

### 4.2 Explicitly out of scope

- User accounts, login, app lock, or roles.
- Cloud sync, remote database, online backup, analytics, crash reporting.
- Multiple boarding houses or multiple treasurers.
- Tenant/boarder self-service.
- Online payments or payment gateways.
- SMS/email reminders or notifications.
- Automatic recurring bill generation.
- Receipt photos or attachments.
- Charts, onboarding tours, decorative animations.
- Web support.
- Multi-currency conversion; changing currency only changes display.
- Partial change returns.
- In-place payment amount edits.
- Permanent deletion of boarders, bills, payments, or bill types.
- Deleting all data from a Settings button (restore replaces; uninstall clears).
- Encryption of the local database or backup file.
- Import merging; restore always replaces.

---

## 5. Architecture

### 5.1 Data flow

```text
Expo UI (React Native)
  → TanStack Query hooks
  → treasurer repository functions (Drizzle queries + transactions)
  → expo-sqlite database (treasurer.db)
```

No tRPC call is involved in these flows. The existing server and `apps/native/utils/trpc.ts` may remain in the repo but are not imported by the treasurer screens.

### 5.2 Local database

- File name: `treasurer.db`.
- Open with `expo-sqlite` (`openDatabaseSync` is sufficient for the singleton).
- Wrap with `drizzle(sqlite, { relations })` from `drizzle-orm/expo-sqlite`.
- On open, enable:
  - `PRAGMA journal_mode = WAL;`
  - `PRAGMA foreign_keys = ON;`
- Run migrations at app startup before rendering any data-dependent screen.
- Seed default settings and bill types once, idempotently, after migrations.
- Export a singleton `db` from `apps/native/src/db/index.ts`.
- Do not import `@my-better-t-app/db` root from the native app; import `@my-better-t-app/db/schema` so the libSQL client is not bundled into the app.

### 5.3 Migrations

Use the existing Drizzle toolchain:

- Add a native Drizzle Kit config (for example `apps/native/drizzle.config.ts`) with:
  - `dialect: "sqlite"`
  - `driver: "expo"`
  - schema pointing at `packages/db/src/schema/index.ts`
  - output pointing at `apps/native/drizzle`
- Add a native script such as `db:generate` that runs Drizzle Kit with that config.
- Generate and commit the SQL migrations and the Drizzle v1 migration entrypoint.
- Add `babel-plugin-inline-import` as a native dev dependency and extend `babel.config.js` with the inline-import plugin for `.sql`, preserving the existing Varlock Babel plugin.
- Extend `metro.config.js` with `config.resolver.sourceExts.push("sql")`, preserving the Varlock, Uniwind, and Reanimated wrappers.
- Import the generated migrations entrypoint and run `useMigrations(db, migrations)` in a database bootstrap component.
- Show a blocking loading/splash state until migrations succeed. On failure, show a readable error screen with Retry; do not render the app with a partially migrated database.
- Do not rely on reading SQL files at runtime from the filesystem.

Drizzle 1.0 RC migration output may differ from older v0 layouts. The implementation must follow the generated entrypoint/layout rather than hand-writing a `journal.json` assumption. If the RC generator cannot produce an Expo bundle, fall back to a checked-in SQL bootstrap guarded by `PRAGMA user_version` while continuing to use Drizzle for all queries; document the fallback in the README. The fallback is a contingency, not the primary plan.

### 5.4 Router and navigation

Replace the generated drawer demo with bottom tabs:

```text
apps/native/app/
  _layout.tsx                  # providers, DB bootstrap, root Stack
  (tabs)/
    _layout.tsx                # bottom tabs: Dashboard, Boarders, Settings
    index.tsx                  # Dashboard
    boarders.tsx               # Active/Archived boarders
    settings.tsx               # Currency, bill types, backup/restore, storage info
  bills/
    new.tsx                    # Create bill; also edit before payments (?id=)
    [id].tsx                   # Bill detail
  boarders/
    [id].tsx                   # Boarder ledger and actions
  bill-types.tsx               # Bill type management
  payments/
    new.tsx                    # Payment modal (?shareId=...)
  +not-found.tsx
```

- Root `unstable_settings.anchor` changes from `(drawer)` to `(tabs)`.
- Remove or repurpose generated `(drawer)` routes and `modal.tsx`.
- Payment screen uses modal presentation; bill form uses a stack screen.
- Add/edit boarder and rename bill type can use HeroUI dialogs rather than extra routes.
- Header actions:
  - Dashboard: “Add bill” (+).
  - Boarders: “Add boarder” (+).
  - Settings: no primary add action.
- Keep the existing theme toggle available in Settings (and optionally a header button).
- Use Ionicons consistently with the existing app.

### 5.5 Dependencies to add

Install Expo-managed packages with `bunx expo install` from `apps/native` so versions match SDK 58:

- `expo-sqlite` (SDK-matched; documented as `~58.0.13` for SDK 58 at the time of writing).
- `expo-file-system` (direct dependency; already pulled transitively by Expo).
- `expo-sharing`.
- `expo-document-picker`.
- `expo-crypto` for `randomUUID()`.
- `@react-native-community/datetimepicker` for payment/due-date pickers.

Also add:

- `drizzle-orm` to `apps/native` at the same `1.0.0-rc.4` version already used by `packages/db`.
- `@my-better-t-app/db` as a workspace dependency of the native app.
- `babel-plugin-inline-import` as a native dev dependency.

Do not add a form library, state library, chart library, date library, or alternative UI kit. Use Zod (already in the stack) for form and backup validation. Use TanStack Query for async data/cache invalidation.

### 5.6 Module plan

```text
apps/native/src/db/
  index.ts                     # expo-sqlite + Drizzle singleton, PRAGMAs
  migrations.ts                # generated migration bundle import
  seed.ts                      # default settings + bill types
  bootstrap.tsx                # migration gate / loading / error UI

apps/native/src/features/treasurer/
  money.ts                     # parse, format, minor-unit helpers
  calculations.ts              # split, payment allocation, totals, filters
  types.ts                     # domain types and status unions
  schemas.ts                   # Zod input/backup schemas
  repository.ts                # transactional reads/writes
  queries.ts                   # React Query hooks and keys
  backup.ts                    # export/import, file/share/picker integration
  components/
    screen.tsx
    money-text.tsx
    status-badge.tsx
    filter-chips.tsx
    month-switcher.tsx
    summary-card.tsx
    empty-state.tsx
    boarder-picker.tsx
    bill-card.tsx
    share-row.tsx
    payment-row.tsx
    confirm-dialog.tsx
    form-field.tsx
```

Use the existing `@/` alias and kebab-case filenames. Keep domain calculations pure and free of React Native imports so they can be unit tested with Bun.

### 5.7 Query and mutation conventions

Suggested query keys:

```text
["treasurer", "settings"]
["treasurer", "bill-types"]
["treasurer", "boarders"]
["treasurer", "boarder", boarderId]
["treasurer", "dashboard", year, month]
["treasurer", "bill", billId]
```

- Use `useQuery` for reads and `useMutation` for writes.
- Invalidate all affected keys after a mutation. For example, recording a payment invalidates the bill, dashboard, boarder, and payment-history queries.
- Use database transactions for:
  - Creating/editing a bill and its shares.
  - Recording a payment and recomputing change allocation.
  - Voiding a payment and recomputing change allocation.
  - Marking change returned.
  - Restoring a backup.
- Disable submit buttons while a mutation is pending to avoid duplicate writes.
- Do not use optimistic updates for money mutations; wait for the transaction and refetch.
- Generate IDs with `expo-crypto` `randomUUID()`.
- Store timestamps as Unix epoch milliseconds (`INTEGER`).
- Store due dates as `YYYY-MM-DD` strings.

---

## 6. Data Model

All money is stored as integer minor units (centavos). All timestamps are epoch milliseconds. All IDs are text UUIDs (except stable seeded bill-type IDs).

### 6.1 `settings`

| Column | Type | Notes |
| --- | --- | --- |
| `key` | TEXT PK | e.g. `currency_code`, `last_backup_at` |
| `value` | TEXT NOT NULL | Stored as text |
| `updated_at` | INTEGER NOT NULL | Epoch ms |

Default keys:

- `currency_code = "PHP"`.
- `last_backup_at = ""` until the first export; store ISO timestamp after export.

### 6.2 `bill_types`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | TEXT PK | Seeded IDs are stable: `type-water`, `type-electricity`, `type-rent`, `type-wifi`, `type-other` |
| `name` | TEXT NOT NULL | Display name, 1–40 chars |
| `is_archived` | INTEGER NOT NULL DEFAULT 0 | 0/1 |
| `sort_order` | INTEGER NOT NULL DEFAULT 0 | Seed order; new types appended |
| `created_at` | INTEGER NOT NULL | |
| `updated_at` | INTEGER NOT NULL | |

No delete. Archiving hides a type from new bills but keeps historical bills readable.

### 6.3 `boarders`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | TEXT PK | UUID |
| `name` | TEXT NOT NULL | 1–80 chars, trimmed |
| `status` | TEXT NOT NULL DEFAULT `'active'` | `'active'` or `'archived'` |
| `archived_at` | INTEGER NULL | Set when archived, cleared on restore |
| `created_at` | INTEGER NOT NULL | |
| `updated_at` | INTEGER NOT NULL | |

No unique constraint on names. Active-name duplicates are allowed after a warning.

### 6.4 `bills`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | TEXT PK | UUID |
| `bill_type_id` | TEXT NOT NULL | FK → `bill_types.id`, restrict delete |
| `bill_type_name_snapshot` | TEXT NOT NULL | Name at bill creation; never rewritten by a type rename |
| `period_year` | INTEGER NOT NULL | 2000–2100 |
| `period_month` | INTEGER NOT NULL | 1–12 |
| `total_minor` | INTEGER NOT NULL | > 0, ≤ 9,999,999,999 |
| `due_date` | TEXT NULL | `YYYY-MM-DD` |
| `note` | TEXT NULL | ≤ 500 chars |
| `archived_at` | INTEGER NULL | Null = not archived |
| `created_at` | INTEGER NOT NULL | |
| `updated_at` | INTEGER NOT NULL | |

Indexes:

- `(period_year, period_month)`
- `(bill_type_id, period_year, period_month)`

No unique constraint for type + period, because duplicates are allowed after an explicit warning.

### 6.5 `bill_shares`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | TEXT PK | UUID |
| `bill_id` | TEXT NOT NULL | FK → `bills.id`, cascade delete |
| `boarder_id` | TEXT NOT NULL | FK → `boarders.id`, restrict delete |
| `boarder_name_snapshot` | TEXT NOT NULL | Name when the share was created; never rewritten by a rename |
| `amount_due_minor` | INTEGER NOT NULL | > 0, ≤ total |
| `created_at` | INTEGER NOT NULL | |

Constraints/indexes:

- Unique `(bill_id, boarder_id)`.
- Index on `boarder_id`.

Each boarder appears at most once per bill. The sum of shares must equal the bill total.

### 6.6 `payments`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | TEXT PK | UUID |
| `bill_share_id` | TEXT NOT NULL | FK → `bill_shares.id`, cascade delete |
| `amount_minor` | INTEGER NOT NULL | > 0, amount handed over |
| `paid_at` | INTEGER NOT NULL | Epoch ms; defaults to now, editable |
| `note` | TEXT NULL | ≤ 500 chars |
| `change_minor` | INTEGER NOT NULL DEFAULT 0 | Excess assigned to this payment |
| `change_returned_at` | INTEGER NULL | Null = change still outstanding |
| `change_return_note` | TEXT NULL | ≤ 200 chars |
| `voided_at` | INTEGER NULL | Null = active |
| `void_reason` | TEXT NULL | Required when voided, 1–200 chars |
| `created_at` | INTEGER NOT NULL | |

Indexes:

- `(bill_share_id)`
- `(paid_at)`

Payments are never hard-deleted by the app.

### 6.7 Statuses

These are computed, not stored (except `archived_at`):

- **Share unpaid:** `paidMinor = 0`, `balance > 0`.
- **Share partial:** `0 < paidMinor < amountDueMinor`.
- **Share paid:** `paidMinor >= amountDueMinor` (even if change is due).
- **Bill open:** at least one share has a positive balance.
- **Bill settled:** every share has `paidMinor >= amountDueMinor`.
- **Bill archived:** `archived_at != null`; archive label takes precedence over open/settled.
- **Change returned:** `change_returned_at != null` on a payment with `change_minor > 0`.
- **Change outstanding:** `change_minor > 0`, `change_returned_at == null`, payment active.

A bill can be settled while change is still outstanding. This is intentional and is shown as a separate banner/badge.

### 6.8 Snapshot and history rules

- A bill stores the bill type name at creation. Renaming a bill type never changes existing bill labels.
- A bill share stores the boarder name at share creation. Renaming a boarder never changes existing share labels.
- If the current name differs from the snapshot, show the snapshot as the primary name and a secondary line such as “Currently: Maria Santos”.
- Archiving a boarder never removes shares or payments. Their rows remain in bill detail and boarder history.
- Archived boarders cannot be selected for new bills, but payments can still be recorded against their existing shares.
- Archiving a bill hides it from default dashboard lists/totals, but its records remain in boarder history and in the bill list when “Show archived” is enabled.
- Voiding a payment keeps the row visible, marked “Voided” with reason and timestamp.

---

## 7. Money Rules

### 7.1 Representation

- All amounts are integers in minor units.
- PHP has 2 decimals; 1 peso = 100 centavos.
- Never use floating-point arithmetic for stored or computed money.
- Parse input by string manipulation, not `parseFloat` multiplication:
  - Trim whitespace.
  - Accept digits, an optional single decimal point, and optional thousands commas.
  - Accept at most 2 decimal places.
  - Reject negative values, zero where positive is required, letters, `NaN`, multiple decimal points, and amounts above the maximum.
  - Convert to minor units as `whole * 100 + cents` using integer arithmetic.
- Maximum per amount: `99,999,999.99` (9,999,999,999 minor units), well below `Number.MAX_SAFE_INTEGER`.

### 7.2 Display

- Use `Intl.NumberFormat` with the configured currency code, 2 fraction digits, and grouping.
- Default: `PHP` → `₱1,234.50` (narrow symbol where supported).
- Fallback if `Intl` currency formatting fails: `PHP 1,234.50`.
- Use `fontVariant: ["tabular-nums"]` for amounts where supported.
- Dates: `Oct 19, 2026`.
- Times: `3:45 PM`.
- Periods: `October 2026`.

### 7.3 Currency setting

- Editable in Settings.
- Curated list of two-decimal currencies: `PHP`, `USD`, `EUR`, `GBP`, `AUD`, `CAD`, `SGD`, `HKD`, `NZD`, `MYR`, `THB`, `AED`, `SAR`.
- Default `PHP`.
- Changing currency changes display only. It does not convert stored amounts.
- If any records exist, show a confirmation warning: “Amounts will not be converted. Existing records will simply be displayed in the new currency.”
- All screens react to the setting through the settings query.

### 7.4 Equal split

Given `totalMinor` and `n` selected active boarders:

1. Reject if `n === 0`.
2. Reject if `totalMinor < n` (“Total is too small to split among N boarders”).
3. `base = Math.floor(totalMinor / n)`.
4. `remainder = totalMinor % n`.
5. Sort selected boarders by stable order:
   - Name ascending, case-insensitive, using `localeCompare` with base sensitivity.
   - Tie-break by `created_at` ascending.
   - Tie-break by `id` ascending.
6. Give one extra minor unit to each of the first `remainder` boarders.
7. Each share is `base` or `base + 1`.
8. Show every final share before saving and a note such as “Remainder of ₱0.02 assigned to the first 2 selected boarders.”

Examples:

| Total | Boarders | Shares |
| --- | --- | --- |
| ₱1,000.00 | 3 | ₱333.34, ₱333.33, ₱333.33 |
| ₱100.00 | 3 | ₱33.34, ₱33.33, ₱33.33 |
| ₱0.02 | 2 | ₱0.01, ₱0.01 |
| ₱0.01 | 3 | Error: too small to split |

### 7.5 Custom split

- Starts with the equal-split amounts as a convenience.
- Each selected boarder must have an amount of at least ₱0.01.
- The sum of all custom shares must exactly equal the bill total.
- Show:
  - Bill total.
  - Assigned total.
  - Difference (`Assigned is ₱X short` / `Assigned is ₱X over`).
- Save is disabled until the difference is exactly zero.
- No silent auto-adjustment and no saving with an unexplained mismatch.

### 7.6 Payment allocation and change

A share can have multiple active payments. Payments are ordered by `paid_at ASC`, then `created_at ASC`, then `id ASC`.

For each active payment, in order:

```text
cumulativeBefore = sum of earlier active payment amounts for this share
cumulativeAfter  = cumulativeBefore + thisPaymentAmount
changeForThisPayment =
  max(0, cumulativeAfter - amountDue) - max(0, cumulativeBefore - amountDue)
```

Consequences:

- The payment that pushes the cumulative total over the share amount gets the excess.
- The sum of all payment `change_minor` values equals `max(0, totalPaid - amountDue)`.
- A payment before the share is fully covered gets 0 change.
- A payment made after the share is already covered gets 100% of its amount as change.

On any payment insert, void, or backdated insertion:

1. Recompute `change_minor` for all active payments of the affected share in chronological order.
2. Preserve each payment’s `change_returned_at` and note.
3. If a payment’s recomputed change is lower than the change already marked returned, show a visible “Needs review” warning on that payment. Do not silently erase the return record.

### 7.7 Per-share totals

```text
paidMinor        = sum(active payment amounts)
appliedMinor     = min(paidMinor, amountDueMinor)
balanceMinor     = max(0, amountDueMinor - paidMinor)
changeOutstanding = sum(change_minor of active payments where change_returned_at is null)
changeReturned    = sum(change_minor of active payments where change_returned_at is not null)
```

“Paid” status is based on `balanceMinor === 0`, not on change.

### 7.8 Bill and dashboard totals

For the selected period, excluding archived bills unless explicitly shown:

```text
expectedMinor        = sum(all shares.amount_due)
appliedMinor         = sum(all shares.min(paid, due))
outstandingMinor     = expectedMinor - appliedMinor
changeOutstanding    = sum(unreturned change across active payments)
cashReceivedMinor    = sum(active payment amounts)  // shown in bill detail if useful
progress             = expectedMinor === 0 ? 0 : appliedMinor / expectedMinor
```

Dashboard summary labels:

- **Expected:** total of all shares.
- **Received:** amount applied to bills (overpayments are not counted here).
- **Still owed:** expected − received.
- **Change to return:** separate card/badge when greater than zero.

This keeps the equation `Expected = Received + Still owed` true. Cash handed over can exceed Received because of change.

### 7.9 Worked examples

| Scenario | Result |
| --- | --- |
| Due ₱250; pay ₱250 | Paid in full; balance ₱0; change ₱0. |
| Due ₱250; pay ₱300 | Paid; balance ₱0; ₱50 change to return. |
| Due ₱250; pay ₱100 | Partial; ₱150 still owed. |
| Due ₱250; pay ₱100, then ₱100 | Partial; ₱50 still owed. |
| Due ₱250; pay ₱100, then ₱200 | Paid; ₱50 change on the second payment. |
| Due ₱250; pay ₱300, then ₱100 | Paid; ₱50 change on the first payment and ₱100 on the second; ₱150 total change. |
| Due ₱250; pay ₱250, then mark change returned (no change) | No change action available. |
| Due ₱250; pay ₱300; mark change returned; void the payment | Payment voided; totals recompute; the returned-change record remains visible and is flagged for review. |
| Due ₱250; payment recorded for 1 Nov for an October bill | The payment stays on the October bill; its date is 1 Nov. The bill period does not change. |

---

## 8. Screen Specifications

### 8.1 App bootstrap

- Keep the splash/loading screen visible while the database opens and migrations run.
- On success, seed defaults and show the Dashboard.
- On migration/open failure:
  - Show a full-screen error state with a short technical message.
  - Provide Retry.
  - Do not show partially loaded screens.
  - Do not automatically delete or recreate the database.

### 8.2 Dashboard (main tab)

**Header**

- App title: “Boarding House Treasurer”.
- Month switcher:
  - Left arrow, month/year label (e.g., “October 2026”), right arrow.
  - Tapping the label opens a month/year picker dialog.
  - Default: current month on app launch; remember the last viewed month only for the current session.
- “Add bill” action (+).

**Summary section**

- Expected, Received, Still owed for the selected month.
- Change to return, shown separately when > 0.
- A compact progress line such as “2 of 3 boarders paid” across the month’s shares.
- Archived bills are excluded from totals. A small “Show archived” toggle controls whether archived bill cards appear in the list; a caption states that archived bills are excluded from totals.

**Bill list**

- One card per bill for the selected month.
- Card content:
  - Bill type name (snapshot).
  - Period (redundant but useful).
  - Total.
  - Progress: “₱750.00 of ₱1,000.00 paid”.
  - Paid count: “2 of 3 paid”.
  - Status: Open / Settled / Archived.
  - Due date and Overdue badge when applicable.
  - Change badge if unreturned change exists.
- Tapping a card opens Bill Detail.
- Sort: open bills first, then settled, then archived; within each group by bill type seed/sort order, then name.
- Empty state: “No bills for October 2026 — create a bill.” with an “Add bill” button.
- If there are no bill types at all: “No active bill types — manage bill types.”

**Period-wide boarder balances**

- A second section, “Boarders this month”, aggregates each boarder’s shares across all non-archived bills in the period.
- Each row:
  - Boarder name; “(archived)” tag if archived.
  - Total due, applied, and remaining for the period.
  - Paid / Partial / Unpaid status.
  - Change-due badge when applicable.
- Row tap opens Boarder Detail.
- Filter chips: All, Unpaid, Paid, Change due, with counts.
  - Paid: all period shares have zero balance.
  - Unpaid: any period share has a positive balance (includes partials).
  - Change due: any unreturned change in the period, independent of paid status.
  - A boarder can appear under both Paid and Change due.
- Empty filter state: “No boarders match this filter.”

**Quick actions**

- “Add bill” primary action.
- “Record payment” is reached from a bill or boarder; no global payment shortcut in v1.

### 8.3 Bill Detail

**Header**

- Bill type name and period.
- Status chip: Open / Settled / Archived.
- Due date and Overdue badge.
- Note, if present.
- Change-to-return banner if any.

**Summary**

- Total expected.
- Received/applied.
- Still owed.
- Change outstanding.
- Paid count.

**Boarder shares**

- List of every included share:
  - Snapshot boarder name (`Currently: ...` when renamed).
  - Amount due.
  - Paid so far.
  - Balance or “Paid”.
  - Status: Paid / Partial / Unpaid.
  - Change-due badge or “Change returned”.
  - Archived tag if the boarder is archived.
  - “Record payment” action.
- Filter chips: All, Unpaid, Paid, Change due with counts.
- Partial shares appear under Unpaid.
- A fully paid share with unreturned change appears under Paid and Change due.
- Empty state if no shares (should not occur due to validation): “No boarders on this bill.”

**Payment history**

- Newest first across all shares.
- Each row shows:
  - Boarder snapshot name.
  - Amount received.
  - Date and time.
  - Note.
  - Change amount and returned status/time/note.
  - Voided state and reason.
  - Actions:
    - “Mark change returned” when change is outstanding and the payment is active.
    - “Void payment” when active.
- Voided rows remain visible, muted/struck, excluded from totals.
- Empty state: “No payments recorded yet.”

**Actions**

- Edit bill:
  - If no active payments: full edit (type, period, total, boarders, split, due date, note).
  - If any active payment exists: money/identity fields locked; only due date and note editable.
  - Archived bill: read-only except unarchive (and perhaps note/due date; keep it simple: read-only).
- Archive bill:
  - Confirmation.
  - If a balance remains, warn that the debt remains in boarder records and show the amount.
  - Sets `archived_at`.
- Unarchive bill.
- No delete.

### 8.4 Create / Edit Bill

**Flow**

1. Bill type:
   - Active types only.
   - No default selection; placeholder “Select bill type”.
   - Link to manage bill types.
2. Period:
   - Month/year picker.
   - Default to the Dashboard’s selected month.
   - Year range 2000–2100.
3. Total amount:
   - Decimal keypad.
   - Inline validation.
4. Due date (optional):
   - Native date picker; clear action.
5. Note (optional):
   - Text area, max 500 chars.
6. Include boarders:
   - All active boarders listed and preselected.
   - Checkbox per boarder.
   - “Select all” / “Clear all” convenience actions.
   - Archived boarders are not shown.
   - If no active boarders: empty state with “Add a boarder first”.
7. Split method:
   - Segmented control: Equal / Custom.
   - Equal:
     - Live preview of each share.
     - Remainder explanation.
     - Error if total is too small for the number selected.
   - Custom:
     - One amount input per selected boarder.
     - Initialized from the equal split.
     - Assigned total and difference shown.
     - Save disabled until exact.
8. Review:
   - Show type, period, total, due date, note, boarder count, split method, and each share.
   - Primary button: “Create bill” / “Save changes”.

**Validation**

- Bill type required.
- Period required and valid.
- Total > 0 and within max.
- At least one selected boarder.
- No duplicate boarder in the selection.
- Equal split feasible.
- Custom amounts each > 0 and sum exactly total.
- Bill type + period duplicate check:
  - Query all bills with the same type and period, including archived.
  - If any exist, show a dialog naming the existing bill(s), total, and status.
  - Buttons: “Cancel” and “Create anyway”.
  - No hard block.

**Save behavior**

- One transaction inserts the bill and all shares.
- Snapshot the bill type name and each boarder’s current name.
- Generate share rows in the stable display order.
- After success, navigate to Bill Detail and show a success toast.
- Cancel with unsaved changes asks for confirmation.

**Edit behavior**

- Only allowed when no active (non-voided) payments exist on any share.
- Updating a bill replaces its shares transactionally.
- If a boarder was renamed since the bill was created, existing share snapshots stay unchanged; newly added shares use the current name.
- If all payments are voided, full editing becomes possible again.
- After the first active payment, disable type, period, total, boarder selection, and split fields with an explanation: “Money fields are locked because payments have been recorded.”

### 8.5 Record Payment

Open from a share row in Bill Detail or Boarder Detail.

**Header**

- Boarder name (snapshot + current name if different).
- Bill type, period.
- Amount due, paid so far, remaining balance.
- Existing change due reminder, if any: “₱50 change still to return from an earlier payment.” Never auto-net.

**Fields**

- Amount received:
  - Prefill the remaining balance.
  - If the balance is zero, start blank and show “This share is already paid; any new amount will be change to return.”
  - Decimal keypad; inline validation.
- Payment date and time:
  - Default now.
  - Editable via native date/time picker.
  - Future date shows a non-blocking warning.
- Note (optional, max 500 chars).

**Live preview**

Show the prospective result before saving:

- Amount equals remaining → “Paid in full.”
- Amount greater than remaining → “Paid. ₱X change to return.”
- Amount less than remaining → “₱X still owed after this payment.”
- Also show cumulative paid and final balance.

**Save**

- Transaction inserts the payment, recomputes change allocation for the share, and updates relevant queries.
- Success toast: “Payment recorded.”
- Disable the button while pending.
- Multiple payments are allowed and each is a separate row.
- No in-place edit; corrections use Void.

### 8.6 Change Return

- Action appears only on active payments with `change_minor > 0` and no `change_returned_at`.
- Dialog text names the person and amount: “Mark ₱50.00 change returned to Maria Santos?”
- Optional returned date/time, default now, editable.
- Optional note, max 200 chars.
- Confirm action records the timestamp and note; no automatic return.
- After saving:
  - Row shows “Change returned on Oct 19, 2026, 3:45 PM”.
  - The change no longer counts as outstanding.
  - The payment remains in history.
- No partial change returns.
- No unmark action in v1. If a change return was recorded by mistake, the payment can still be voided with a reason; the return record remains visible and is flagged.

### 8.7 Void Payment

- Action on active payments.
- Confirmation dialog:
  - Shows amount, boarder, bill, and current change status.
  - Requires a reason (trimmed, 1–200 chars).
  - If change was already returned, show a strong warning: “This payment’s change was already marked returned. Voiding may leave an over-refund in the history; the return record will be kept for review.”
- On confirm:
  - Set `voided_at` and `void_reason`.
  - Recompute change for the share.
  - Keep the row visible and excluded from totals.
  - Show success toast.
- No unvoid. Record a new payment instead.
- The audit trail is the retained payment row plus void timestamp/reason.

### 8.8 Boarders Tab

- Two segmented tabs: Active and Archived, each with a count.
- Add boarder action (+).
- Active rows:
  - Name.
  - Optional secondary line: all-time outstanding balance if > 0.
  - Tap opens Boarder Detail.
- Archived rows:
  - Muted styling.
  - “Archived” label.
  - Optional secondary line: outstanding balance if > 0.
- Empty states:
  - Active: “No boarders yet — add your first boarder.”
  - Archived: “No archived boarders.”
- Add boarder dialog:
  - Name field only.
  - Trim and validate.
  - If an active boarder has the same name (case-insensitive), show a warning but allow saving.
- Rename:
  - From Boarder Detail.
  - Same validation and duplicate warning.
- Archive:
  - Confirmation.
  - If the boarder has any outstanding balance across all bills, warn with the total and state that the debt remains payable.
  - Allowed after confirmation.
- Restore:
  - Returns the boarder to active.
  - Warn if another active boarder has the same name.
- No delete.

### 8.9 Boarder Detail

- Header: current name, Active/Archived status, optional “Currently owes ₱X” summary.
- Actions: Edit name, Archive/Restore.
- Summary across all bills, including archived bills:
  - Expected.
  - Applied/paid.
  - Outstanding.
  - Change outstanding.
- Bills ledger:
  - Grouped or sorted by period descending.
  - Each row: bill type, period, share due, paid, balance/paid status, change, archived-bill tag.
  - “Record payment” action for shares with a balance; also allow extra payment when paid if the user explicitly wants to record change (optional, show warning).
- Payment history:
  - All payments across bills, newest first.
  - Same change/void actions as Bill Detail.
- Historical entries show snapshot names; the current name is shown in the header.

### 8.10 Bill Types Screen

- Reached from Settings.
- List active types first, then archived, with bill counts.
- Add:
  - Name field, 1–40 chars.
  - Block case-insensitive duplicates among active types.
  - New type appended to the sort order.
- Rename:
  - Block duplicates among active types.
  - Existing bill snapshots are not changed.
- Archive:
  - Allowed even if bills exist.
  - Existing bills remain readable.
  - Type no longer appears when creating a bill.
- Restore:
  - Blocked if an active type already has the same name; ask for a rename first.
- Empty state: “No bill types — add one to start creating bills.”
- No delete.
- "Other" is a normal editable type; use the optional bill note for details.

### 8.11 Settings

Sections:

1. **Currency**
   - Select from the curated list.
   - Show current code and example formatted amount.
   - Confirmation warning when records exist; no conversion.
2. **Bill types**
   - Row linking to Bill Types screen.
3. **Backup**
   - “Export backup” primary button.
   - “Last backup: …” or “No backup yet.”
   - “Restore backup” secondary/destructive button.
   - Explanation: full JSON backup, includes boarder names and payment records, unencrypted.
4. **Data storage**
   - “Stored only on this device in an app-private SQLite database.”
   - “Uninstalling the app or clearing app data permanently deletes all records. Export a backup regularly.”
   - “No accounts, cloud sync, or analytics.”
5. **Appearance**
   - Existing light/dark theme toggle.
6. **About**
   - App name and version from `app.json`.
   - Optional short line: “Local treasurer record keeping.”

No “delete all data” button in v1.

---

## 9. Backup and Restore

### 9.1 Export

- Use `expo-file-system` modern `File`/`Paths` API.
- Build a plain JSON object with a versioned envelope.
- Write pretty-printed JSON to a cache file named:
  `boarding-house-treasurer-backup-YYYY-MM-DD-HHmm.json`
- Use `expo-sharing` `shareAsync(uri, { mimeType: "application/json", dialogTitle: "Save boarding house backup" })`.
- If `Sharing.isAvailableAsync()` is false, show an error and do not claim success.
- On successful share initiation, update `last_backup_at` in Settings to the current ISO timestamp and show success.
- Exporting an empty database is allowed; the file contains empty arrays.
- Do not write backups automatically or to an online service.

### 9.2 Backup format

```json
{
  "format": "boarding-house-treasurer",
  "version": 1,
  "exportedAt": "2026-10-19T12:34:56.000Z",
  "data": {
    "settings": {
      "currencyCode": "PHP",
      "lastBackupAt": "2026-10-19T12:30:00.000Z"
    },
    "billTypes": [
      {
        "id": "type-water",
        "name": "Water",
        "isArchived": false,
        "sortOrder": 0,
        "createdAt": "2026-10-19T09:00:00.000Z",
        "updatedAt": "2026-10-19T09:00:00.000Z"
      }
    ],
    "boarders": [
      {
        "id": "uuid",
        "name": "Maria Santos",
        "status": "active",
        "archivedAt": null,
        "createdAt": "2026-10-19T09:00:00.000Z",
        "updatedAt": "2026-10-19T09:00:00.000Z"
      }
    ],
    "bills": [
      {
        "id": "uuid",
        "billTypeId": "type-water",
        "billTypeNameSnapshot": "Water",
        "periodYear": 2026,
        "periodMonth": 10,
        "totalMinor": 100000,
        "dueDate": "2026-10-31",
        "note": "October water",
        "archivedAt": null,
        "createdAt": "2026-10-19T09:00:00.000Z",
        "updatedAt": "2026-10-19T09:00:00.000Z"
      }
    ],
    "billShares": [
      {
        "id": "uuid",
        "billId": "uuid",
        "boarderId": "uuid",
        "boarderNameSnapshot": "Maria Santos",
        "amountDueMinor": 33334,
        "createdAt": "2026-10-19T09:00:00.000Z"
      }
    ],
    "payments": [
      {
        "id": "uuid",
        "billShareId": "uuid",
        "amountMinor": 30000,
        "paidAt": "2026-10-19T10:00:00.000Z",
        "note": "Cash",
        "changeMinor": 5000,
        "changeReturnedAt": null,
        "changeReturnNote": null,
        "voidedAt": null,
        "voidReason": null,
        "createdAt": "2026-10-19T10:00:00.000Z"
      }
    ]
  }
}
```

- Timestamps in the backup are ISO 8601 strings; convert to epoch ms on restore.
- `changeMinor` is included for readability but recomputed on restore from active payments.
- Include only the tables above. No server/tRPC data.

### 9.3 Import / restore

- Use `expo-document-picker` with:
  - `type: ["application/json", "text/plain"]` for provider compatibility.
  - `copyToCacheDirectory: true`.
- Read the selected file with the modern `File` API.
- Reject files larger than 10 MB.
- Parse JSON; on syntax error, show “This file is not valid JSON.”
- Validate with Zod:
  - Envelope format string.
  - Version exactly `1`; reject newer versions with a clear message.
  - Required arrays and fields.
  - String lengths, amount ranges, month range, status enums.
  - Unique IDs.
  - Referential integrity:
    - Every `billTypeId` exists.
    - Every `billId` exists for shares.
    - Every `billShareId` exists for payments.
    - Every share’s `amountDueMinor` is > 0.
    - Each bill’s shares sum exactly to `totalMinor`.
  - No duplicate `(billId, boarderId)`.
- Recompute change allocation for every share from active payments in chronological order.
  - Preserve returned timestamps/notes.
  - If recomputed change is 0 but the payment claims returned change, flag it in the summary; do not silently discard the record.
- Show a preview dialog:
  - Exported date.
  - Counts: boarders, bill types, bills, shares, payments.
  - Currency.
  - A warning: “Restoring replaces all current data on this device. This cannot be undone.”
  - Buttons: Cancel / “Replace all data”.
- On confirm:
  - Run one database transaction: delete all rows in dependency order, then insert settings, types, boarders, bills, shares, payments.
  - On any error, roll back and show “Restore failed. Your current data was not changed.”
  - On success, clear/invalidate all queries, close dialogs, navigate to Dashboard (current month), and show “Backup restored.”
- Do not merge.
- Do not import into a separate dataset.
- If the backup is empty, allow restore after the same confirmation; it clears the app data.

### 9.4 Storage limitation messaging

Show in Settings and README:

- Data is stored only in the app’s private SQLite database (`treasurer.db`).
- Uninstalling the app or clearing app data deletes everything.
- Backups are manual and unencrypted.
- There is no cloud sync or recovery service.
- The app does not encrypt its local database.
- Anyone with access to the unlocked phone can open the app; no app lock in v1.

---

## 10. Validation and Edge Cases

| Case | Required behavior |
| --- | --- |
| Empty or whitespace-only name | Reject with inline error; trim before saving. |
| Name too long | Reject over 80 chars (boarders) / 40 chars (bill types). |
| Duplicate active boarder name | Warn but allow after confirmation. |
| Duplicate active bill type name | Block; require a different name. |
| Zero/negative bill total | Reject; Save disabled. |
| Invalid amount text | Reject; show “Enter a valid amount.” |
| More than 2 decimal places | Reject. |
| Amount above max | Reject. |
| Bill with no selected boarders | Block; explain. |
| No active boarders exist | Empty state with a link to add one. |
| Equal split with remainder | Distribute one minor unit to first N in stable order; show all shares. |
| Total too small for equal split | Block with a clear message. |
| Custom shares mismatch | Show difference; block Save until exact. |
| Custom share zero/negative | Reject; each included boarder needs at least ₱0.01. |
| Duplicate bill type + month | Warn with existing bill details; “Create anyway” required. |
| Bill with no payments | Open; show “No payments recorded yet.” |
| Partial payment | Allowed; show cumulative paid and remaining. |
| Overpayment | Allowed; show “Paid” plus change due. |
| Multiple payments on one share | Allowed; each is a new row; totals accumulate. |
| Payment while change is outstanding | Allowed; keep old change separate; show reminder; no auto-net. |
| Payment after the share is fully paid | Allowed with a warning; the whole new amount becomes change. |
| Payment date in the future | Warn but allow. |
| Payment date in another month | Allowed; it stays attached to the original bill period. |
| Void a payment | Require reason and confirmation; keep row; recompute totals. |
| Void a payment whose change was returned | Strong warning; keep return record; flag review. |
| Re-record after void | New payment row; no unvoid. |
| Mark change returned twice | Action hidden after the first return. |
| Partial change return requested | Not supported; whole change only. |
| Archived boarder has unpaid share | Allow archive with warning; debt stays visible and payable. |
| Rename boarder after bills | Bill shares keep snapshot names; show current name as secondary. |
| Archive boarder after bills | Bills/payments unchanged; boarder excluded from new bills. |
| Restore archived boarder | Active again; not auto-added to bills. |
| Rename bill type after bills | Bills keep snapshot; show current type name as secondary. |
| Archive bill type after bills | Existing bills remain; type unavailable for new bills. |
| Archive bill with balance | Allow with warning; excluded from default totals; visible via Show archived and in boarder history. |
| Unarchive bill | Restores to default list/totals. |
| Edit bill after payment | Money fields and included boarders locked; due date/note editable. |
| Edit bill after all payments voided | Full edit allowed again. |
| Archived bill editing | Read-only except unarchive. |
| Bill detail with renamed/archived participants | Snapshot remains primary; current status/name shown as secondary. |
| Period with no bills | Empty state with Create bill action. |
| Filter has no matches | Filter-specific empty state. |
| Currency changed with existing records | Confirm; display changes only; no conversion. |
| Restore invalid file | No changes; clear error. |
| Restore older/newer version | Reject unsupported version; no changes. |
| Restore with referential errors | Reject; no changes. |
| Migration failure | Blocking error screen; Retry; no partial app. |
| App closed and reopened | All records persist in SQLite. |
| App uninstalled / data cleared | Records are lost; documented; manual backup is the recovery path. |
| Phone offline | All core features work; no network required. |

---

## 11. Visual and Interaction Design

### 11.1 Layout

- Phone-first single-column layout.
- On larger screens/tablets, center content with a maximum width (e.g., 720 px) rather than stretching forms.
- Bottom tabs with labels and Ionicons.
- Native stack headers for bill detail, boarder detail, bill form, and payment modal.
- Safe-area handling via the existing `Container` and `useSafeAreaInsets`.
- Scrollable forms with keyboard handling from `react-native-keyboard-controller`.
- Primary actions within thumb reach; bill cards and share rows are fully tappable.

### 11.2 Typography and amounts

- Page title: 24–28 pt semibold.
- Section heading: 16–18 pt semibold.
- Body: 14–16 pt.
- Captions/meta: 12–13 pt muted.
- Amounts: 18–24 pt semibold, tabular numerals.
- Use existing HeroUI Native `Text`, `Card`, `Chip`, `Surface`, and theme tokens; avoid one-off color values where a semantic token exists.

### 11.3 Status presentation

Never rely on color alone. Every status includes text and an icon:

| Status | Label | Icon suggestion | Semantic color |
| --- | --- | --- | --- |
| Paid | Paid | checkmark-circle | success |
| Paid in full (exact) | Paid in full | checkmark-circle | success |
| Partial | Partial | half / remove-circle | warning |
| Unpaid | Unpaid | alert-circle | danger |
| Open bill | Open | time-outline | neutral/accent |
| Settled bill | Settled | checkmark-done | success |
| Archived | Archived | archive-outline | muted |
| Change due | ₱X change | cash-outline | accent/warning |
| Overdue | Overdue | calendar-outline | danger |
| Voided | Voided | close-circle | muted/danger |

Use HeroUI semantic variants where possible. Ensure contrast in both light and dark themes.

### 11.4 Touch targets and accessibility

- Minimum 44×44 pt touch targets; primary buttons at least 48 pt tall.
- Icon-only buttons need accessibility labels.
- Filter chips expose selected state and counts to screen readers.
- Share/payment rows expose a combined label including name, amount, status, and actions.
- Support dynamic font scaling; avoid fixed-height text containers.
- Use `accessibilityRole` for buttons, headers, checkboxes, and tab controls.
- Amount inputs use decimal keypad and clear labels (“Amount received”, not just “Amount”).
- Do not disable zoom/dynamic type.
- Test light and dark mode.
- Keep text concise; no onboarding or tutorial overlays.

### 11.5 Loading, success, error, empty states

- DB bootstrap: full-screen splash/spinner with app name.
- Initial list queries: skeleton or spinner; local queries should resolve quickly.
- Mutation pending: spinner inside the pressed button; disable repeat taps.
- Success: toast or inline confirmation banner; avoid a blocking alert for routine success.
- Validation: inline field errors beneath the field; focus the first invalid field when practical.
- Query failure: error banner with Retry.
- Mutation failure: dialog or inline error with the action preserved; never show a success state.
- Empty states:
  - No boarders: “No boarders yet — add your first boarder.”
  - No bills: “No bills for October 2026 — create a bill.”
  - No payments: “No payments recorded yet.”
  - No change: “No change to return.”
  - No archived boarders: “No archived boarders.”
  - Empty filter: “No boarders match this filter.”
  - No bill types: “No active bill types — manage bill types.”
  - No backup: “No backup yet.”
- Confirmation dialogs:
  - Archive boarder with debt.
  - Archive bill with balance.
  - Duplicate bill creation.
  - Duplicate boarder name.
  - Void payment.
  - Restore backup.
  - Currency change with existing data.
  - Discard unsaved bill form.

### 11.6 Motion

- No decorative animation, charts, onboarding, or celebratory effects.
- Standard native stack/tab transitions only.
- Do not use animation to convey payment status.

---

## 12. Error Handling and Data Integrity

- Every multi-row write uses a SQLite transaction.
- Foreign keys are enabled at connection open.
- Amounts are validated before conversion to minor units.
- Share sums are validated against the bill total before insert.
- Payment change allocation is deterministic and recomputed after inserts/voids.
- Restore validates everything before touching existing rows and uses one transaction.
- Do not swallow database errors. Show a generic user message and log details only in development.
- Never log boarder names, amounts, notes, or backup contents in production.
- Disable mutation buttons while pending.
- Use generated UUIDs so retries cannot create duplicate rows from a double tap.
- If a query fails because the database is not ready, wait for bootstrap; do not render empty data as if it were real.

---

## 13. Privacy and Local-Only Behavior

- No accounts, login, analytics, cloud database, or online sync.
- No network requests for core app data.
- No personal information beyond a boarder’s name.
- Do not collect phone numbers, addresses, IDs, or payment account details.
- Data lives in the app-private SQLite file `treasurer.db`.
- The app does not add encryption. It relies on the device’s OS-level storage protection and lock screen.
- Backup JSON is unencrypted and contains names and payment records; warn before sharing and tell the user to save it somewhere trusted.
- Add `android.allowBackup: false` in the Expo app config to prevent Android Auto Backup from uploading the local database. Document that iOS device backups may include app data depending on system settings; the app itself adds no cloud backup.
- Remove the tRPC health check from the app’s startup path so an offline phone never shows an API error.
- Keep the server package untouched for template completeness, but do not present it as required to run the app.

---

## 14. Implementation Plan

Do not start implementation until this spec is approved. When implementation begins, use this order and verify each phase.

### Phase 1 — Schema, local DB, migrations, seed

- Add treasurer tables to `packages/db/src/schema/treasurer.ts`.
- Export them from `packages/db/src/schema/index.ts`.
- Update `packages/db/src/relations.ts` if required by the Drizzle RC relation API.
- Add native Drizzle Kit config and generated migrations.
- Configure Metro `.sql` source extension and Babel inline-import while preserving existing wrappers.
- Add `apps/native/src/db/index.ts`, `migrations.ts`, `seed.ts`, and `bootstrap.tsx`.
- Add dependencies and workspace package link.
- Verify fresh database creation, migration, seed, and persistence after app restart.

### Phase 2 — Domain logic and unit tests

- Implement `money.ts` (parse/format/minor units).
- Implement `calculations.ts`:
  - Equal split and remainder allocation.
  - Custom-share validation.
  - Payment ordering and change allocation.
  - Share/bill/dashboard aggregates.
  - Status and filter classification.
- Implement `schemas.ts` (Zod).
- Add Bun tests for all pure logic.
- Run `bun test` and `bun run check-types`.

### Phase 3 — Navigation shell and boarders

- Replace drawer demo routes with bottom tabs and root stack.
- Add Dashboard/Boarders/Settings placeholders.
- Implement boarder list, add, rename, archive, restore.
- Implement Boarder Detail ledger skeleton.
- Verify empty states and validation.

### Phase 4 — Bill types and bill creation

- Implement Bill Types screen.
- Implement bill form with type, period, total, due date, note, boarder selection, equal/custom split preview.
- Implement duplicate warning.
- Implement transactional create.
- Verify equal remainder cases and custom mismatch blocking.

### Phase 5 — Dashboard and bill detail

- Implement period switching, summary cards, bill cards, period-wide boarder balances, and filters.
- Implement Bill Detail with shares, statuses, filters, notes, due/overdue.
- Implement edit locking rules and archive/unarchive.
- Verify archived bills are excluded from totals and visible via Show archived.

### Phase 6 — Payments, change, void

- Implement payment entry screen with prefill and live preview.
- Implement transactional payment recording and change allocation.
- Implement change-return dialog.
- Implement void-with-reason and recomputation.
- Implement payment history in Bill Detail and Boarder Detail.
- Verify partial/overpayment/repeated/backdated/void cases.

### Phase 7 — Settings, backup, restore, docs

- Implement currency setting with confirmation.
- Implement JSON export via share sheet and restore via document picker.
- Implement validation, preview, transaction replace, and query reset.
- Add Android backup opt-out and app display name.
- Update `README.md` with run instructions, data location, backup steps, and limitations.
- Add in-app storage disclaimer.

### Phase 8 — Polish and verification

- Accessibility labels, dynamic type, dark mode.
- Loading/error/empty states.
- Run full checks and manual test matrix.
- Fix regressions.
- Summarize screens, workflows, data location, and limitations for the user.

---

## 15. Files Expected to Be Added or Changed

### Database / native persistence

- `packages/db/src/schema/treasurer.ts` (new)
- `packages/db/src/schema/index.ts` (export tables)
- `packages/db/src/relations.ts` (update if needed)
- `apps/native/drizzle.config.ts` (new)
- `apps/native/drizzle/**` (generated; commit)
- `apps/native/metro.config.js` (add `sql` source extension)
- `apps/native/babel.config.js` (add inline-import plugin)
- `apps/native/src/db/index.ts` (new)
- `apps/native/src/db/migrations.ts` (new generated entrypoint import)
- `apps/native/src/db/seed.ts` (new)
- `apps/native/src/db/bootstrap.tsx` (new)
- `apps/native/package.json` (deps + scripts)
- `bun.lock` (updated by install)

### Feature code

- `apps/native/src/features/treasurer/money.ts`
- `apps/native/src/features/treasurer/calculations.ts`
- `apps/native/src/features/treasurer/types.ts`
- `apps/native/src/features/treasurer/schemas.ts`
- `apps/native/src/features/treasurer/repository.ts`
- `apps/native/src/features/treasurer/queries.ts`
- `apps/native/src/features/treasurer/backup.ts`
- `apps/native/src/features/treasurer/components/**`

### Screens

- `apps/native/app/_layout.tsx` (bootstrap + tabs/stack)
- `apps/native/app/(tabs)/_layout.tsx`
- `apps/native/app/(tabs)/index.tsx`
- `apps/native/app/(tabs)/boarders.tsx`
- `apps/native/app/(tabs)/settings.tsx`
- `apps/native/app/bills/new.tsx`
- `apps/native/app/bills/[id].tsx`
- `apps/native/app/boarders/[id].tsx`
- `apps/native/app/bill-types.tsx`
- `apps/native/app/payments/new.tsx`
- `apps/native/app/+not-found.tsx` (keep/update)
- Remove or repurpose generated drawer/modal demo screens that conflict with the new navigation.

### Configuration / docs

- `apps/native/app.json` (display name; `android.allowBackup: false`)
- `README.md` (app usage, data storage, backup, limitations)
- `apps/native/src/features/treasurer/__tests__/*.test.ts` (new)
- Root `package.json` optional `test` script using Bun (`bun test`); no new runner dependency.

---

## 16. Testing and Verification Plan

### 16.1 Automated

- `bun install` after dependency changes; preserve `bun.lock`.
- `bun run check-types` across the monorepo.
- `bun test` (or `bun test apps/native/src/features/treasurer`) for pure domain tests.
- `bun run build` to confirm the server package still builds.
- From `apps/native`, run an Expo export for both native platforms, for example:
  `bunx expo export --platform android --platform ios`
  This verifies Metro bundling, including SQL migrations.
- If an Expo/React Native lint or doctor script exists, run it; do not add lint tooling just for this feature.

### 16.2 Unit test cases

**Money**

- Parse `250`, `250.5`, `250.50`, `1,234.56`.
- Reject `""`, `" "`, `-1`, `0` where positive required, `1.234`, `1,2,3`, `abc`, `NaN`, too-large values.
- Format PHP and a second currency.
- Convert minor units back without floating-point drift.

**Equal split**

- ₱1,000 / 3 → 33334, 33333, 33333.
- ₱100 / 3 → 3334, 3333, 3333.
- ₱0.02 / 2 → 1, 1.
- ₱0.01 / 3 → error.
- Stable order with duplicate names → created_at/id tie-breakers.
- Sum always equals total.

**Custom split**

- Exact sum valid.
- Short/over sum invalid.
- Zero/negative share invalid.

**Payments**

- Exact payment: no change.
- Overpayment: correct change on the crossing payment.
- Partial payment: correct balance.
- Two partials: correct cumulative state.
- Partial then overpay: change assigned to the second payment.
- Two overpayments: each gets its own change.
- Backdated payment: recomputation order.
- Void first payment: change recomputed; returned-change flag retained.
- Payment after fully paid: full amount becomes change.
- Multiple payments never overwrite prior rows.

**Totals / status / filters**

- Expected = Received + Outstanding.
- Paid share with change appears in both Paid and Change due.
- Partial appears in Unpaid, not Paid.
- Settled bill with unreturned change is still settled.
- Archived bills excluded from dashboard totals.
- Boarder period aggregates across multiple bills.

**Backup**

- Valid round trip preserves all records.
- Unsupported version rejected.
- Missing foreign key rejected.
- Share sum mismatch rejected.
- Malformed JSON rejected.
- Restore is atomic on failure.

### 16.3 Manual test matrix

Run on at least one Android emulator/device. iOS bundle export is required; iOS runtime testing is desirable but may be unavailable on a Windows host.

1. Fresh install → dashboard shows current month, empty states.
2. Add three boarders; rename one; archive one; restore it.
3. Create a Water bill for October 2026, ₱1,000, equal split across three; verify exact shares and remainder.
4. Attempt a duplicate Water/October bill; verify warning and “Create anyway”.
5. Create an Electricity bill for the same month; verify both coexist independently.
6. Try custom shares that are short/over; verify Save is blocked.
7. Record a full payment; verify Paid in full.
8. Record an overpayment; verify Paid + change; verify filter/change count.
9. Mark change returned; verify timestamp and removal from Change due.
10. Record a partial payment on another share; verify remaining balance and Unpaid filter.
11. Add a second partial payment; verify cumulative paid.
12. Void a payment with a reason; verify history row, totals, and status revert.
13. Backdate a payment across a month boundary; verify it stays on the original bill.
14. Archive a boarder with debt; verify warning, bill history, and ability to record payment.
15. Rename that boarder; verify bill snapshot and “Currently” label.
16. Archive a bill with a balance; verify default totals exclude it, Show archived reveals it, boarder history still includes it.
17. Change currency; verify confirmation and display-only change.
18. Export a backup; inspect file name/JSON validity.
19. Restore to the same app; verify counts/confirmation and identical data.
20. Restore an invalid/corrupt file; verify no data changes.
21. Force-quit and reopen; verify persistence.
22. Toggle dark mode; verify contrast and status labels.
23. Test with large text/dynamic type; verify no clipped amounts or unreachable buttons.
24. Test offline/airplane mode; verify all core flows work.

### 16.4 Reporting

- Report commands actually run and their results.
- Report any checks that could not run (for example, no iOS simulator) as limitations, not passes.
- Report any remaining backup/restore or platform limitations explicitly.

---

## 17. Acceptance Criteria

The implementation is accepted when all of the following are true:

1. Opening the app offline shows the Dashboard for the current calendar month without requiring the Hono/tRPC server.
2. Boarders can be added with a name only, renamed, archived, and restored; active and archived lists are separate.
3. Archiving a boarder does not delete or rewrite any bill share or payment.
4. Renaming a boarder does not change historical bill share names.
5. Bill types are seeded with Water, Electricity, Rent, Wi-Fi, and Other; types can be added, renamed, archived, and restored.
6. Renaming a bill type does not change historical bill labels.
7. Bills can be created for a type + calendar month with a positive total, optional due date, and optional note.
8. Only active boarders are selectable for new bills; all active boarders are selected by default.
9. Equal split handles centavo remainders deterministically and shows every final share before save.
10. Custom shares must exactly equal the bill total; Save is blocked otherwise.
11. A duplicate type + month warns with existing bill details and requires explicit confirmation.
12. Each bill stores snapshots of the type name, boarder names, and share amounts.
13. The Dashboard shows selected-month expected, received, still owed, and change to return, plus bill progress and a filtered period-wide boarder balance list.
14. Bill Detail shows each boarder’s due, paid, balance, status, change, and payment history.
15. Filters correctly classify Paid, Unpaid (including partial), and Change due.
16. A payment can be recorded with an amount, date/time, and optional note; the remaining balance is prefilled.
17. Partial payments accumulate without overwriting earlier payments.
18. An overpayment shows “Paid” and a separate change amount; the share does not appear as owing.
19. Change is tracked per payment and is not considered returned until the user marks it returned.
20. Marking change returned records a timestamp and optional note and removes it from outstanding change.
21. Payments can be voided with a required reason and confirmation; the row remains visible and totals recalculate.
22. Voided payments are excluded from totals; no payment is permanently deleted through the UI.
23. A bill settles automatically when every share is covered, even if change is outstanding.
24. Money fields lock after the first active payment; due date and note remain editable.
25. Archived bills are hidden from default lists/totals, remain readable via Show archived and boarder history, and can be unarchived.
26. Archived boarders with balances remain visible in bill/boarder history and can still receive payments.
27. Currency defaults to PHP, is changeable in Settings, and changing it does not convert stored amounts.
28. A complete JSON backup can be exported through the share sheet and restored through the document picker.
29. Restore validates the file, shows a summary, requires confirmation, replaces data atomically, and leaves data unchanged on failure.
30. Data persists across app restarts.
31. Settings clearly documents local-only storage and the risk of data loss on uninstall/clear data.
32. The app works on Android and iOS; no web support is added.
33. No accounts, cloud sync, analytics, network dependency, or extra PII is added.
34. Type checks pass; pure domain tests pass; the native bundle exports for Android and iOS; the server build remains unaffected.
35. Common actions are reachable with large touch targets and statuses use text + icons, not color alone.

---

## 18. Documentation Requirements

Update the root `README.md` with:

- What the app does.
- Confirmation that the mobile app uses on-device SQLite and does not require the server.
- How to install and run:
  - `bun install`
  - `bun run dev:native`
  - Platform commands (`android`, `ios`) and the development-build caveat if Expo Go is not on a matching SDK.
- Where data is stored:
  - App-private SQLite database `treasurer.db` via `expo-sqlite`.
- Backup and restore:
  - Settings → Export backup → share sheet.
  - Settings → Restore backup → pick JSON → confirm replacement.
  - Backup files are unencrypted and contain boarder names and payment records.
- Data-loss warning:
  - Uninstalling the app or clearing app data deletes records.
  - No cloud recovery; manual backups are required.
- Limitations:
  - Single device, single treasurer, no sync.
  - No web support.
  - No app lock/encryption.
  - No partial change returns.
  - No in-place payment editing.
- Checks:
  - `bun run check-types`
  - `bun test`
  - `bun run build`
  - `cd apps/native && bunx expo export --platform android --platform ios`

The existing template documentation may remain below the app-specific section, but the app section must come first and be accurate.

---

## 19. Assumptions

1. The boarding house is a single location with one treasurer.
2. All boarders pay in the same currency.
3. Each bill uses exactly one bill type.
4. Each included boarder has exactly one share per bill.
5. A payment applies to exactly one boarder’s share.
6. Overpayment/change is tracked per bill share; it is not automatically applied to another bill or another person.
7. Amounts use two-decimal minor units.
8. Billing periods are calendar months in the device’s local timezone.
9. Payment dates are recorded in local time but stored as epoch ms.
10. The app UI language is English.
11. Boarder names are labels, not unique identifiers.
12. Historical snapshots are intentionally immutable.
13. The app display name becomes “Boarding House Treasurer”; the existing slug, scheme, bundle identifiers, and package name remain unchanged to avoid unrelated native configuration churn.
14. The Hono/tRPC server remains in the repository but is not part of the core app runtime.
15. Android and iOS are the only supported targets for v1.

---

## 20. Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| Drizzle 1.0 RC Expo migration tooling differs from older docs | Follow generated output; add the Babel/Metro SQL configuration; verify a fresh app launch; fall back to a guarded SQL bootstrap only if necessary. |
| Large change to navigation from drawer to tabs | Implement the shell early and verify typed routes; remove conflicting demo routes. |
| Currency formatting differences on Hermes | Use `Intl.NumberFormat` with a safe fallback to `CODE 1,234.56`; test PHP and one other currency. |
| Money rounding errors | Integer minor units only; pure unit tests; no floating-point arithmetic. |
| Restore corrupts data | Validate fully before writes; single transaction; rollback on failure; show preview. |
| Local data loss on uninstall | In-app warning, README warning, manual JSON backup, optional last-backup timestamp. |
| Android cloud backup contradicts local-only intent | Set `android.allowBackup: false`; document iOS system-backup behavior. |
| Duplicate taps create duplicate payments | Pending-state disabling + generated UUIDs + transactional insert. |
| Change allocation becomes inconsistent after edits/voids | Deterministic chronological recomputation; retain return flags; flag review cases. |
| Archived participants make history confusing | Snapshot names as primary; show current name/status as secondary. |
| Expo Go SDK 58 mismatch on a physical device | Use a development build or SDK-matched emulator; document the caveat. |
| No iOS runtime available on Windows | Verify Android manually and export the iOS bundle; report iOS runtime testing as a limitation if it cannot be performed. |
| Scope creep into accounting features | Explicit out-of-scope list; defer charts, reconciliation, recurring bills, and exports beyond JSON. |

---

## 21. Definition of Done

- All acceptance criteria are met.
- The spec’s screens and workflows are implemented in the existing Better T Stack project.
- `bun run check-types` passes.
- Domain tests pass.
- Native Android and iOS bundles export successfully.
- The server build still passes.
- README and in-app storage/backup limitations are documented.
- No accounts, cloud sync, analytics, or unnecessary dependencies were added.
- The final summary tells the user how to run the app, where data is stored, and what limitations remain.
