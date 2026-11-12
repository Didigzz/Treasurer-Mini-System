# Boarding House Treasurer

A phone-first, offline record-keeping app for one boarding-house treasurer, written in
Rust with [Slint](https://slint.dev) and built only through Cargo.

The treasurer can keep a boarder list, manage bill types, split monthly shared bills
equally or with custom amounts, record payments and change to return, follow balances per
month and per boarder, and export/restore everything as a JSON file.

All records live **only on the device** in a local SQLite database (`treasurer.db`). There
are no accounts, cloud sync, or analytics.

See [installation.md](installation.md) for building and installing the Android APK.

## Features

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

## Commands

```bash
cargo run -p treasurer-mobile --bin treasurer   # Preview the UI on the desktop
cargo test -p treasurer-core                    # Run the core tests
(cd apps/mobile && x build --platform android --arch arm64 --format apk)   # Debug APK
```

The Android build uses xbuild; there is no Gradle project. The tests cover money parsing,
equal/custom splits, payment and change calculations, SQLite storage, dashboard and
history queries, and a JSON backup export/restore round trip.

## Project Structure

```
apps/
  mobile/                  # Rust/Slint phone app (Cargo + xbuild)
crates/
  treasurer-core/          # Rust domain, SQLite storage, backups, tests
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
- Bill money fields lock after the first active payment; corrections are made by voiding a
  payment and recording a new one.
