# Boarding House Treasurer

A phone-first, offline record-keeping app for one boarding-house treasurer, written in
Rust with [Slint](https://slint.dev) and built only through Cargo.

The treasurer can keep a boarder list, manage bill types, split monthly shared bills
equally or with custom amounts, record payments and change to return, follow balances per
month and per boarder, and export/restore everything as a JSON file.

All records live **only on the device** in a local SQLite database (`treasurer.db`). There
are no accounts, cloud sync, or analytics.

See [docs/installation.md](docs/installation.md) for building and installing the Android APK.

## Features

- **Boarders** — add, rename, archive, restore, delete; boarders with bill history can
  only be archived so past bills are preserved.
- **Bill types** — seeded Water, Electricity, Rent, Wi-Fi, Other; add, rename, archive,
  restore. Old bills keep the name they were created with.
- **Monthly bills** — calendar-month picker, optional due date and note, duplicate
  warning, archive/unarchive.
- **Calendar pickers** — every date the treasurer enters (bill period, due date, payment
  date and time, dashboard month, change-return date) opens a month-grid calendar sheet
  instead of typed text, from pages and from inside the payment, return-change, and
  payment-detail sheets. The dashboard month is also tappable.
- **Select all tenants** — the New bill form's tri-state Select all control, with the
  selected count always visible and typed custom amounts preserved.
- **Equal or custom splits** — leftover centavos go to the first boarders in selection
  order; custom shares must add up to the bill total.
- **Share management after creation** — the bill's Manage shares sheet edits, excludes
  (keeping the bill total and re-splitting it equally among the remaining boarders), or
  adds a boarder to that one bill.
- **Payments** — a payment must cover the remaining balance (paying more produces change
  to return), date and time picker, editable date/time, live balance and change preview.
- **Paid by method** — a settled share's pill shows `Paid – Cash` or `Paid – Online`,
  taken from its newest active payment.
- **Share filters** — the bill's Shares list narrows by name, by All / Unpaid / Paid, and
  by All / Cash / Online. Selecting Cash or Online also shows how much that method
  collected on the bill, net of change.
- **Overpayment notice** — a payment that creates change opens a non-dismissing dialog
  naming the amount received and the excess, with a "Return change now" shortcut.
- **Change tracking** — explicit Change to Return / Change Returned status on payment and
  share rows, a chosen return date, and a read-only payment breakdown showing total bill,
  amount received, change returned, net payment, and both statuses. A settled bill can
  still have change outstanding.
- **Undo payment** — reverse an accidental payment confirmation after a centered
  confirmation dialog. The original record stays in history marked Undone; balances,
  statuses, and change recalculate from the remaining payments.
- **Backup** — export all data as JSON through the share sheet; restore replaces all
  local data after confirmation and validation.
- **Dashboard** — selected-month totals, bill progress, and boarder balances with
  All / Unpaid / Paid / Change due filters.

## Commands

```bash
cargo run -p treasurer-mobile --bin treasurer   # Preview the UI on the desktop
cargo test -p treasurer-core                    # Run the core tests
export PATH="$LOCALAPPDATA/Android/Sdk/ndk/27.1.12297006/toolchains/llvm/prebuilt/windows-x86_64/bin:$PATH"
(cd apps/mobile && x build --platform android --arch arm64 --format apk)   # Debug APK
```

The Android build uses xbuild; there is no Gradle project. The tests cover money parsing,
equal/custom splits, payment and change calculations, SQLite storage, dashboard and
history queries, and a JSON backup export/restore round trip.

## Project Structure

```
apps/
  mobile/                  # Rust/Slint phone app (Cargo + xbuild)
    src/app/               # One module per feature: dashboard, boarders, bills,
                           #   history, settings, backup
crates/
  treasurer-core/          # Rust domain, SQLite storage, backups, tests
docs/
  installation.md          # Building and installing the Android APK
  specs/                   # Design specs and mocks
```

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
- Bill money fields lock after the first active payment; corrections are made by undoing a
  payment and recording a new one.
