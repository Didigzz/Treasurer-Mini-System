# Paid Share Payment Type and Cash/Online Filter — Specification

**Status:** Draft, ready for implementation
**Date:** 2026-11-16
**App:** Boarding House Treasurer 1.2.0 → 1.3.0
**Scope:** Show Cash/Online on paid shares, add a Cash/Online filter with per-method collected totals, disallow partial payments
**Request:** "in paid. the status cash or online payment should be seen there, also add a filter for cash or online payment"

This document records what the code does today, the decisions made during the clarification interview, the exact UX behavior, the edge cases, and the acceptance checks. No code has been changed yet.

---

## 1. Summary of decisions

| Topic | Decision |
|---|---|
| Where the type is shown | Bill screen **Shares** list only. The status pill of a settled share reads `Paid – Cash` or `Paid – Online`. |
| Ledger and dashboard | Unchanged. The boarder ledger's bill rows and the dashboard "Boarders this month" list keep their plain `Paid` pill. |
| Mixed payments | The share pill uses the **latest active payment's** type. A share paid ₱500 cash then ₱500 online reads `Paid – Online`. |
| Partial shares | Partial payments are no longer allowed (see §4). Existing partial shares keep their `Partial` label until completed. |
| Filter target | The bill screen **Shares** list, the same list the existing name search and All/Unpaid/Paid filter already narrow. |
| Filter control | A **second `Segmented` row** — `All / Cash / Online` — directly below the existing `All / Unpaid / Paid` row. Both apply together with the name search (AND). |
| Filter matching | Only shares with at least one **active** payment can match Cash or Online; the match uses the same latest-active-payment type as the pill. |
| Unpaid / charge-only shares | Do not match Cash or Online; they disappear under both type filters. They remain visible under `All`. |
| Empty state | Reuse the existing "No boarders match" card; Clear filter resets name, status, and type filters. |
| Summary totals | The bill-wide summary metrics never change when filtering. Selecting Cash or Online additionally shows the money actually **collected** by that method on this bill (§5.2.1); no outstanding figure is shown there. |
| Filter reset | Reset per bill. Opening a bill starts with `All` status, `All` type, and an empty name search. |
| Payment amount field | Pay sheet opens **prefilled with the remaining balance** and stays editable upward. |
| Minimum payment | The remaining balance. Anything above is allowed and uses the existing change-to-return / overpayment flow. |
| Existing partial shares | Allowed to finish: the next payment must cover the whole remaining balance. No forced undo, no migration. |
| Enforcement | Domain (`record_payment`) **and** UI. No path may record a partial payment. |
| Schema / backup | No new tables or columns and no backup format change. `payment_type` already exists; old payments/backups read as cash. |
| Version | Bump to 1.3.0 (`version_code` 4) in manifest and Settings. |
| Docs | Update README feature bullets for full payments, paid-share method label, and the type filter. |
| Tests | Core tests for the latest-payment-type helper, the type filter predicate, and the partial-payment rejection; update existing tests that record partial payments. |

---

## 2. Context and current behavior

### 2.1 Architecture

- Rust + Slint phone-first app. UI: `apps/mobile/ui/*.slint`; state and wiring: `apps/mobile/src/app/*.rs`.
- Domain, SQLite storage, money math, backup: `crates/treasurer-core`.
- One window (`AppWindow`). Bill screen is screen id 10; boarder ledger is 11.
- Amounts are integer minor units, formatted through `treasurer-core::money`.
- Design tokens live in `theme.slint`; shared controls (`Segmented`, `StatusPill`, `SectionHeader`, `EmptyState`, `ListCard`) in `components.slint` / `layout.slint`.

### 2.2 Payment types today

- `crates/treasurer-core/src/models.rs` already defines:
  - `PaymentType { Cash, Online }` with `as_str()`, `from_db()` (anything that is not `online` reads as Cash), `label()` (`"Cash"` / `"Online Payment"`), and `status_label()` (`"Paid – Cash"` / `"Paid – Online"`).
  - `PaymentRecord.payment_type` and `PaymentRecord::status_label()`, which returns the type label for active payments and `"Undone"` for undone ones.
- The payment sheet has a `Segmented` payment type control (`app.slint`, options `["Cash", "Online Payment"]`, bound to `payment-type-index`).
- `payments.payment_type` is stored (`DEFAULT 'cash'`), loaded, exported, and restored. Old backups without the field read as cash.
- Payment rows already display `payment.payment-type` and `payment.status`, so `Paid – Cash` / `Paid – Online` is visible on individual payment rows and in the payment detail sheet.

### 2.3 What is missing

- `ShareRow.status` comes from `ShareStatus::label()` — `"Unpaid"`, `"Partial"`, `"Paid"` — so a settled share never shows how it was paid. The bill's Shares list, the boarder ledger's bill list, and the dashboard boarder list all show a plain `Paid` pill.
- `ShareState` aggregates money and payments but exposes no "type of the latest active payment".
- The bill's Shares filter is name text (`share-filter`) plus a 3-way status mode (`share_filter_mode`: 0 All, 1 Unpaid, 2 Paid).
- `record_payment` only rejects amounts `<= 0`; a payment below the share balance is accepted and produces `ShareStatus::Partial`.

### 2.4 Partial payments today

The app explicitly supports partial payments: `ShareStatus::Partial`, README bullets, `compute_share_state`, and tests all cover them. The clarification interview decided to remove that capability going forward, while leaving already-partial data completable.

---

## 3. Requirement 1 — Show Cash/Online on paid shares

### 3.1 Goal

On the bill screen, a boarder whose share is fully paid shows how the money arrived, not just that it arrived.

### 3.2 Display rule

For each row in the bill's **Shares** list:

| Share state | Pill text |
|---|---|
| Paid, latest active payment is Cash | `Paid – Cash` |
| Paid, latest active payment is Online | `Paid – Online` |
| Paid, no active payment at all (zero-value share) | `Paid` (fallback when no type exists) |
| Partial | `Partial` (unchanged) |
| Unpaid | `Unpaid` (unchanged) |

- Reuse the existing en-dash wording from `PaymentType::status_label()`; do not invent new copy.
- The pill keeps `Theme.status-tone(...)`; `theme.slint` already maps `Paid – Cash` / `Paid – Online` to `Tone.success`.
- The payment type is not shown anywhere else on the bill row; the payment rows keep their own type pills.
- Do not change the boarder ledger's bill list or the dashboard boarder list in this release.

### 3.3 Picking the type

- "Latest active payment" means the active (`voided_at == None`) payment with the greatest `paid_at`, then `created_at`, then `id` as a deterministic tiebreak — the same ordering already used by the undo picker in `bills.rs`.
- One shared helper must answer this so the pill and the filter can never disagree. Add to `crates/treasurer-core/src/calculations.rs` (exported):

```rust
/// Type of the newest active payment, or None when the share has no active payments.
pub fn latest_active_payment_type(payments: &[PaymentWithComputedChange]) -> Option<PaymentType>;
```

- The mobile layer calls this once per share while building `ShareEntry` and stores `latest_payment_type: Option<PaymentType>` on `ShareEntry`.
- Do not add a new column, and do not derive the type from `bill_summary` or a separate query.

### 3.4 Affected UI and wiring

- `apps/mobile/ui/models.slint` — `ShareRow` needs a boolean such as `settled: bool` (or the Rust side can keep emitting a status string). Recommended: keep `status` for display and add `settled` so the UI stops comparing label strings.
- `apps/mobile/src/app/bills.rs` — `ShareEntry` gains `latest_payment_type: Option<PaymentType>`; `render_shares()` builds the display status:
  - when `entry.status == ShareStatus::Paid` and a type exists, use `entry.latest_type.status_label()`;
  - otherwise use `entry.status.label()`.
- `apps/mobile/ui/screens/bill.slint` — the share-row `StatusPill` already renders `share.status`, so the label updates automatically. Two direct string comparisons must change:
  - the "Undo payment" button's `if share.status == "Paid"` → use the new `settled` flag (or `!share.has-balance`), otherwise Undo disappears once the pill reads `Paid – Cash`;
  - the row `label` / `accessible-label` strings keep using `share.status`, which now carries the method — no change needed beyond that.
- `apps/mobile/src/app/boarders.rs` also builds `ShareRow` for the ledger and must compile when `ShareRow` changes; it may pass the plain `status.label()` plus the new flag, because the ledger keeps its plain pill. If a shared `ShareRow` field is added, fill it everywhere.

### 3.5 Edge cases

- **Share paid by two payments of different types** → latest active payment wins; pill reads that type (`Paid – Online` in the interview example).
- **Latest payment undone** → it is excluded; the type falls back to the newest remaining active payment, and the pill follows it.
- **All payments undone** → status becomes `Unpaid`; no type pill.
- **Overpayment** → the share is Paid; the type is the overpayment's type; change is unaffected.
- **Additional charge** → no effect on the rule; balance already includes it.
- **Archived bill** → the type still shows; archived state is conveyed by the header pill and the archive section.
- **Zero-value share (base 0, no charge)** → status Paid with no payment; pill stays `Paid`.
- **Legacy payments / restored backups without `payment_type`** → cash (already handled by `from_db`).

### 3.6 Acceptance criteria — display

1. Pay a share fully in cash → its Shares pill reads `Paid – Cash`.
2. Pay a share fully online → its pill reads `Paid – Online`.
3. Pay ₱500 cash then ₱500 online on the same share → pill reads `Paid – Online`.
4. Undo the online payment → pill returns to `Paid – Cash`.
5. Undo the only payment → pill returns to `Unpaid`.
6. The "Undo payment" button is still offered on a settled share after the label change.
7. The ledger bill list and dashboard boarder list still show plain `Paid`.
8. Payment rows and the payment detail sheet keep showing `Paid – Cash` / `Paid – Online`.

---

## 4. Requirement 2 — Do not allow partial payments

### 4.1 Goal

A payment either settles the share or overpays it (with change to return). The treasurer can no longer leave a share in a new partial state by recording a smaller amount.

### 4.2 Rule

For any new payment on a share:

- `minimum = remaining balance = share.total_due_minor() - sum(active payment amounts)`, floored at 0.
- Reject when `amount_minor < minimum`.
- Allow `amount_minor == minimum` (settles the share).
- Allow `amount_minor > minimum` (overpayment; existing change/overpayment flow applies unchanged).
- A share that is already Paid (minimum 0) is not payable again from the normal Pay button; keep whatever guard already exists rather than adding a new "pay a settled share" path.

Because the minimum is always the *remaining* balance, an already-partial share can only be finished: its next payment must cover all that is left.

### 4.3 Enforcement

**Domain** — `Treasurer::record_payment` in `crates/treasurer-core/src/db/repository/payments.rs`:

- After loading the share, compute the current state from existing active payments (the repository already loads `payments_for_share`, so use those before inserting the draft).
- Reject a partial amount before opening the transaction, with an error such as:
  - `"Record the full remaining balance of ₱1,250.00 or more. Partial payments aren't allowed."`
  - or a stable phrasing reused verbatim by the UI; keep the amount formatted with the settings currency.
- Keep the existing `amount_minor <= 0` rejection.
- Do not add a schema constraint; the guard belongs in the repository so restores and tests behave the same.

**UI** — `apps/mobile/src/app/bills.rs` + `app.slint`:

- When opening the Pay sheet, prefill `payment-amount` with the share's `balance-input` and set `payment-used-full-balance = true` when the prefill equals the balance.
- Remove the now-redundant "Use full balance (₱…)" button? **No** — keep it, because the prefill can be edited away; it remains a one-tap reset. It stays consistent with the prefill.
- Show a hint under the amount field while the typed amount is below the balance, e.g. `"Record at least ₱1,250.00 — partial payments aren't allowed."`
- Disable "Record payment" while the amount is empty, unparseable, or below the balance. Add an `in property <bool> payment-can-record` (or equivalent) set from Rust, because Slint cannot compare formatted money strings.
- `pay_share` must use the same rule before calling the domain so the user gets the inline message rather than a banner round trip; the domain check remains the backstop.
- The overpayment notice ("change to return") is unchanged.

### 4.4 Existing data

- Partially paid shares already in the database remain `Partial` and remain payable.
- Their next payment must cover the remaining balance, so they can only move to Paid or Paid-with-change.
- No migration, no forced undo, no automatic adjustments.
- Undo on those legacy partial payments stays available on payment rows/detail exactly as today.

### 4.5 Copy and error states

| Situation | Where | Copy |
|---|---|---|
| Typed amount below balance | Pay sheet hint, red | "Record at least {balance} — partial payments aren't allowed." |
| Record button with too little | disabled + hint | (same hint) |
| Domain rejection (stale screen / backup test) | banner | "Record the full remaining balance of {balance} or more. Partial payments aren't allowed." |
| Empty amount | unchanged | button disabled |

### 4.6 Edge cases

- **Overpay** → allowed, change-to-return flow and the overpayment dialog unchanged.
- **Exact balance** → settles; no change.
- **Legacy partial share** → minimum is the remaining balance; completing it works.
- **Amount equals balance after "Use full balance"** → `used_full_balance` stays true as today.
- **Course-correcting a mistyped amount** → the amount field is editable until Record; no partial is ever written.
- **Undo then re-pay** → after undo the balance is restored; the new minimum is that restored balance.
- **Additional charge changes** → minimum recomputes from `total_due_minor()` (base + charge).
- **Existing tests that record partials** → update them to full amounts; add one explicit rejection test (see §7).

### 4.7 Acceptance criteria — no partials

1. A ₱1,000 share opens the Pay sheet prefilled with `1000.00`; Record is enabled.
2. Typing `500` shows the hint and disables Record; nothing is saved.
3. Calling `record_payment` directly with `500` returns the domain error and writes no row.
4. Typing `1200` records successfully and shows the change-to-return flow for ₱200.
5. A legacy partial share (₱400 of ₱1,000) can be completed with ₱600 but rejects ₱300.
6. Payment status stays `Paid`/`Unpaid` for new data; `Partial` only appears for pre-existing rows.
7. Overpayment, change return, and undo behavior are unchanged.

---

## 5. Requirement 3 — Cash/Online filter on the bill's Shares list

### 5.1 Goal

The treasurer can narrow the bill's share list to boarders whose newest payment was cash or online, combining that with the existing name search and status filter.

### 5.2 Placement and appearance

Inside the bill screen's **Shares** section, below the existing controls:

```
[ Find a boarder                    ]
[ All | Unpaid | Paid ]              (existing status segmented)
[ All | Cash   | Online ]            (new payment-type segmented)
Cash collected ₱5,000.00             (only while Cash or Online is selected)
```

- Use the existing `Segmented` component; no new control type.
- Options are exactly `["All", "Cash", "Online"]` so three options fit at 320px width.
- `label: "Filter shares by payment type"` for accessibility.
- The type row is shown whenever `share-total-count > 0`, the same condition as the status row.
- When Cash or Online is selected, a single-line **collected** readout appears directly under the type row (§5.2.1). It is hidden while `All` is selected.
- Keep the section header caption logic but treat an active type filter as "filtered": `{visible} of {total} boarders` whenever the name search, the status mode, or the type mode is non-default.
- Counts and layout must not overflow on narrow phones; no new theme tokens and no new colors.

### 5.2.1 Collected total for the selected type

Selecting a method shows how much money that method brought in for this bill:

- `Cash` selected → caption **"Cash collected"**, value formatted with the saved currency.
- `Online` selected → caption **"Online collected"**.
- `All` selected → the readout is hidden.

The value is the net money collected through that method on the open bill:

```
sum over active payments of that type of max(amount_minor - computed_change_minor, 0)
```

- Undone payments are excluded.
- Change — whether still owed or already returned — is netted out, so the figure is "collected", not "received". Overpayments contribute their net amount, not the amount tendered.
- The readout never shows an outstanding/remaining figure. It answers only "how much cash/online money did this bill take in".
- It is **method-level for the whole bill**, not limited to the rows still visible after filtering. Example: a share paid ₱500 cash then ₱500 online appears only under the `Online` filter, but its ₱500 still counts toward "Cash collected". This keeps the number a true per-method total, which is what the request asks for.
- The name search and the All/Unpaid/Paid status filter do not change the readout; only the type selection does.
- Adding the two method totals reproduces the bill's collected (applied) total, because each active payment's net amount is applied to its share: `cash collected + online collected = bill collected`.

Presentation:

- One line directly under the type `Segmented`: left-aligned caption in `Theme.text-sub` / `Theme.text-muted`, amount on the right in `Theme.weight-semibold` / `Theme.text`.
- It uses the section's existing spacing and is not a new card; it does not replace the summary metrics at the top of the bill.
- Amounts use `money::format_money` with the saved currency, so a currency change reformats it.
- The readout still renders when no rows match (`0 of N boarders`), e.g. "Online collected ₱0.00".
- Archived bills show the readout like any other bill.
- Accessibility: the readout's `accessible-label` contains both parts, e.g. "Cash collected, ₱5,000.00".

Core helper (exported, testable):

```rust
/// Net money collected through one payment method: active payments only,
/// minus each payment's computed change, never below zero.
pub fn collected_for_payment_type(
    payments: &[PaymentWithComputedChange],
    payment_type: PaymentType,
) -> i64;
```

Mobile wiring:

- `refresh_bill_detail()` aggregates `cash_collected_minor` and `online_collected_minor` across every share's state into new `App` fields, using the helper. Do this once per refresh, not per row or per scroll.
- A small render step (called from `render_shares()` and when the type mode changes) sets two window properties:
  - `share-type-collected-caption` — `"Cash collected"`, `"Online collected"`, or `""` for All;
  - `share-type-collected` — the formatted amount, or `""` for All.
- These values do not depend on `display_share_ids`, so filtering never accidentally recomputes or resets them.

### 5.3 Filter semantics

`type_mode`: 0 = All, 1 = Cash, 2 = Online.

A share is visible when **all** of these hold:

1. Name search is empty, or the display name contains the needle (current behavior).
2. Status mode matches balance (`1` → balance > 0, `2` → balance == 0, `0` → any).
3. Type mode matches the share's latest active payment type (`0` → any, `1` → `Some(Cash)`, `2` → `Some(Online)`).

Consequences, by decision:

- Shares with no active payment (Unpaid, charge-only, zero-value Paid) never match Cash or Online; they appear only under `All`.
- A share paid cash then online matches **Online** (and not Cash), because the pill and the filter share the same "latest active payment" helper.
- Mixed-type shares match exactly one type filter.
- Archived bills filter normally; archiving does not change payments.
- Filtering never changes the bill-wide summary metrics. The method collected readout (§5.2.1) is an additional, type-scoped figure, not a replacement for them.
- The readout follows only the type mode, never the name or status filters.

### 5.4 State, wiring, and reset

Rust (`apps/mobile/src/app/mod.rs`, `bills.rs`):

- New `App` field `share_type_filter: i32` (0..=2), initialized to 0.
- `set_share_type_filter(mode)` clamps to 0..=2, writes the window property, calls `render_shares()`.
- `render_shares()` adds the type predicate; it must use `ShareEntry.latest_payment_type`.
- `clear_detail_lists()` / the existing filter-reset path (currently clears `share_filter` and `share_filter_mode`) also resets `share_type_filter` and the window property.
- Opening a bill (`open_bill`) continues to reset all three filters (via the existing reset path).

Slint (`apps/mobile/ui/app.slint`, `bill.slint`):

- New `in-out property <int> share-type-filter: 0;`
- New `callback set-share-type-filter(int);` wired in the Rust `wire` function like `set-share-filter-mode`.
- Pass the property and callback through to the `Shares` section (`BillScreen` gains `share-type-filter` / `set-share-type-filter`).
- The "Clear filter" action in the empty state calls all three resets (name, status, type).

### 5.5 Empty state and results

- Reuse the existing card: "No boarders match" / "Try another name or filter." / "Clear filter".
- The collected readout stays visible even when the card replaces the list, so a type with money collected but no matching rows still shows its total.
- The current "Clear filter" only resets name + status; extend it to reset the type mode too.
- The list keeps its normal row content and the new `Paid – Cash` / `Paid – Online` pill; no separate type row or badge is added.
- The `0 of N boarders` count is driven by `share-visible-count` and already exists; only its trigger condition changes.

### 5.6 Edge cases

- **Filter Cash while every share is unpaid** → zero rows and the empty card; counts show `0 of N`.
- **Name + status + type all set** → intersection; clearing restores everything.
- **Type filter with a stale row model** → `display_share_ids` must stay index-aligned with the rendered rows so Pay/Edit/Undo still target the right share (the existing debug assertion covers this).
- **Pay a share while a type filter is active** → after refresh, the row may leave or enter the filtered list; the list and counts re-render; no filter reset.
- **Undo the latest payment while filtered to its type** → the row's type falls back to the previous payment; the row moves accordingly.
- **Archived bill** → filters remain usable.
- **Returning to the bill from the ledger** → filters are reset per bill, including the type filter and the collected readout.
- **Mixed share while filtered** → the row appears only under its newest payment's method, while that share's earlier payment of the other method still counts toward the other method's collected total.
- **Undo the only cash payment while filtered to Cash** → its money leaves the cash collected total and the row drops out of the filtered list.
- **Overpayment with change outstanding or returned** → only the net amount counts in the collected total, never the tendered amount.
- **Every share unpaid** → both method totals are ₱0.00; the readout shows the selected method's zero rather than disappearing.

### 5.7 Acceptance criteria — filter

1. Bill with cash-paid, online-paid, unpaid, and mixed shares → `All` shows every share.
2. Tap `Cash` → only shares whose newest active payment is cash remain, including any partially paid cash shares (legacy).
3. Tap `Online` → only shares whose newest active payment is online remain.
4. Mixed share (cash then online) appears under `Online` only.
5. Unpaid shares appear under `All` only.
6. Name search, status filter, and type filter combine; the header caption shows `{visible} of {total} boarders`.
7. "Clear filter" in the empty card resets name, status, and type.
8. Summary metrics above the list are identical with and without any filter.
9. Opening another bill starts at `All` / `All` with an empty search.
10. Pay, Edit, Undo, and Return change still act on the tapped row while filtered.
11. Select Cash → the readout reads "Cash collected" with the net total of every active cash payment on the bill.
12. Select Online → the readout reads "Online collected" with the net total of every active online payment.
13. `cash collected + online collected` equals the bill's collected total shown in the summary card.
14. A mixed share (cash then online) appears under Online, yet its cash amount is included in the Cash collected readout.
15. Undo an active payment → the matching method total drops by that payment's net amount; returning change does not change the net total.
16. Select All → the readout disappears; the summary metrics are identical in every mode.

---

## 6. Cross-cutting requirements

### 6.1 Data integrity and compatibility

- No new tables or columns; `payment_type` already persists, exports, and restores.
- Old backups (`payment_type` absent) read as cash.
- No backup `version` bump.
- Money stays integer minor units; no floats in the new comparisons.
- The latest-type helper and the filter predicate are pure functions in core, so they are testable without the UI.

### 6.2 Design system and platform

- Only `Theme` tokens; no hard-coded colors.
- `Segmented` already handles three equal options; verify it fits at 320px with the new labels.
- Desktop preview (`cargo run -p treasurer-mobile --bin treasurer`) and Android must both work; the type pill must not overflow the share row on narrow screens (elide behavior already exists on the name, not the pill — if the row tightens, let the name elide).
- 44px minimum touch target inherited from `Segmented`.
- The collected readout stays on one line; if the amount is long, the caption elides or wraps before it, never pushing the Shares list off-screen.

### 6.3 Accessibility

- The new control is announced as "Filter shares by payment type" with tabs All / Cash / Online and the selected state.
- The share pill's accessible label is the pill text (`Paid – Cash`, `Paid – Online`), so the method is not color-only.
- The Pay sheet's minimum hint is visible text, not a tooltip.

### 6.4 Documentation and version

- `apps/mobile/manifest.yaml`: `version_name: 1.3.0`, `version_code: 4`.
- `apps/mobile/src/app/mod.rs`: Settings string `"Boarding House Treasurer 1.2.0"` → `"1.3.0"`.
- `apps/mobile/Cargo.toml` still says `version_name = "1.1.0"`; align it to `1.3.0` while touching versions.
- README:
  - Payments bullet: state that a payment must cover the remaining balance (overpayment produces change), replacing "partial payments" wording.
  - Add that a settled share's pill shows `Paid – Cash` or `Paid – Online`.
  - Add the All / Cash / Online share filter next to the existing filter bullet.
- Do not rewrite the README structure.

### 6.5 Out of scope

- Cash/Online display on the dashboard, boarder ledger bill rows, history, or the bill summary header.
- Cash/Online filtering of the Payments section, the dashboard boarder list, history, or the ledger.
- A permanent side-by-side cash-vs-online breakdown card, or per-method collected totals on the dashboard, ledger, history, or the bill summary header. Only the single, type-scoped readout in §5.2.1 is in scope.
- A new payment type (e.g. bank transfer, GCash as a separate type) or free-text method notes.
- Migrating or retroactively correcting existing partial payments.
- Changing split math, additional charges, change-return accounting, undo rules, duplicate-bill rules, or the backup format.

---

## 7. Test plan

### 7.1 Automated (Rust core)

Run: `cargo test -p treasurer-core`

New unit tests (`calculations.rs`):

- `latest_active_payment_type`: empty → `None`; one cash → `Cash`; two payments, newest online → `Online`; newest undone → falls back to the remaining active payment; all undone → `None`; equal `paid_at` resolved by `created_at` deterministically.
- Type filter predicate (if added as a core helper, e.g. `matches_payment_type_filter(latest, mode)`): `All` matches everything; `Cash`/`Online` reject `None`.
- `collected_for_payment_type`: only cash → cash amount; only online → online amount; mixed payments split correctly; undone excluded; overpayment contributes `amount - change`; change returned or outstanding still nets out; no payments → 0; never negative.

Repository tests (`tests/bill_management.rs` / `tests/storage.rs`):

- `record_payment` rejects an amount below the remaining balance; no row is written.
- `record_payment` accepts exactly the remaining balance; the share becomes Paid.
- `record_payment` accepts an overpayment and sets change.
- A legacy partial share (insert one, or record before the rule in a fixture) accepts a payment for the remainder and rejects less.
- Existing tests that intentionally record partial payments (storage, bill_management, custom_split) must be updated to full amounts; keep their original intent (payment totals, change, undo) intact.

Mobile compile:

- `cargo check -p treasurer-mobile` (compiles the Slint UI and the Rust wiring).

### 7.2 Desktop manual script

1. Create a bill with 4 boarders; pay one fully in cash, one fully online, one with cash then a second full online payment (after the first payment the share is already paid, so use a large bill where the first payment would be partial — instead simulate mixed types by setting a charge, or verify via a legacy partial fixture), and leave one unpaid.

   Practical script: bill ₱1,000 × 4. Pay boarder A ₱1,000 cash. Pay boarder B ₱1,000 online. For a mixed share, record ₱1,000 cash, undo it, then record ₱1,000 online (different type, single payment); the label follows the remaining active payment.
2. Verify shares read `Paid – Cash` and `Paid – Online`; unpaid reads `Unpaid`.
3. Verify the "Undo payment" button still appears on both paid shares.
4. Open the Pay sheet for the unpaid boarder: the amount is prefilled with `1000.00` and Record is enabled.
5. Type `500`: hint appears, Record disabled. Type `1000`: hint clears. Type `1200`: overpayment notice appears with change.
6. Tap Cash filter: only the cash-paid share remains; header shows `1 of 4 boarders`; summary numbers unchanged.
7. Tap Online filter: only the online-paid share remains.
8. Combine with status `Paid` and a name fragment; confirm intersection and the empty card when nothing matches; Clear filter restores all.
9. Open another bill: all filters reset.
10. Archive the bill and repeat a filter: still works; the paid label is unchanged.
11. With Cash selected, verify "Cash collected" equals the net total of every active cash payment on the bill; repeat for Online; confirm the two add up to the summary's collected figure and that a mixed share's cash payment counts under Cash.

### 7.3 Android checklist (required)

Build and install per README, then:

- Confirm the new third segmented row is fully visible and tappable at phone width, and that it does not crowd the keyboard.
- Confirm `Paid – Cash` / `Paid – Online` pills fit the share row without clipping; names elide instead.
- Confirm the prefilled Pay amount is editable and the minimum hint/disabled state behaves.
- Confirm filtering by type with a scroll long enough to exercise the list and that Pay still targets the tapped row.
- If no device is available, report that limitation explicitly instead of claiming an Android pass.

### 7.4 Definition of done

- Acceptance criteria in §3.6, §4.7, and §5.7 pass.
- `cargo test -p treasurer-core` passes.
- `cargo check -p treasurer-mobile` passes.
- Desktop script passes; Android checklist passes or is reported unverified.
- Version is 1.3.0 in manifest, Cargo.toml, and Settings.
- No unrelated files, features, or schema changes are included.

---

## 8. Implementation plan (suggested order)

1. **Core helpers**
   - Add `latest_active_payment_type` to `calculations.rs` and export it.
   - Add tests for ordering, undo fallback, and empty cases.
2. **Paid-share label**
   - Add `latest_payment_type` to `ShareEntry`; populate it from `share.state.payments`.
   - Map Paid + type to `status_label()` when building `ShareRow`.
   - Add a `settled` flag (or similar) to `ShareRow`; switch the Undo button condition from the `"Paid"` string to that flag; fill the field in the ledger builder too.
3. **No partial payments**
   - Enforce the minimum in `record_payment` (domain) with the formatted error.
   - Prefill the Pay amount with the balance, set the used-full-balance flag when it matches, add the minimum hint, and gate the Record button through a Rust-driven `payment-can-record` property.
   - Update existing tests and add the rejection/acceptance tests.
4. **Cash/Online filter**
   - Add `share_type_filter` to `App`, new window property/callback, reset in the existing reset path, and the predicate in `render_shares`.
   - Add `collected_for_payment_type`, aggregate the two method totals in `refresh_bill_detail()`, and set the readout properties when the type mode or the bill refreshes.
   - Add the second `Segmented` row, the collected readout line, and thread props/callbacks through `BillScreen`.
   - Extend the caption condition and the Clear filter action.
5. **Cross-checks**
   - Confirm summary metrics remain untouched, `display_share_ids` stays aligned, and filters reset per bill.
6. **Docs and version**
   - Bump manifest/Cargo.toml/Settings to 1.3.0; update README bullets.
7. **Verify**
   - `cargo test -p treasurer-core`, `cargo check -p treasurer-mobile`, desktop script, Android checklist.

---

## 9. Resolved interview decisions

- Type appears only on the bill's Shares list; dashboard and ledger keep a plain `Paid`.
- The label is the latest active payment's type, rendered as `Paid – Cash` / `Paid – Online`.
- Partial payments are rejected outright. The rule is "remaining balance or more"; overpayment keeps the change-to-return flow.
- Existing partial shares keep working and can only be completed, never extended into more partials.
- The Pay sheet prefills the balance and keeps the amount editable.
- Enforcement lives in both the domain and the UI.
- The filter is a second `Segmented` row (`All / Cash / Online`) under the existing status row, applying to the same Shares list and combining with name and status.
- Only shares with an active payment can match Cash/Online; unpaid shares match only `All`.
- The empty state reuses the current card and its Clear filter resets all filters.
- Selecting Cash or Online also shows a collected total for that method (net of change, whole bill, not limited to visible rows). No outstanding figure is shown in that readout.
- Bill-wide summary totals stay unchanged by filtering; filters reset per bill.
- Release includes core tests, README updates, and a version bump to 1.3.0.

## 10. Assumptions and open questions

Assumptions (no further input needed):

- `Paid` means `ShareStatus::Paid` (balance zero); `Partial` means balance > 0 with active payments (legacy data only going forward).
- Undone payments never decide the displayed or filtered type.
- A zero-value share with no payment shows plain `Paid`.
- "Collected" in the method readout means net collected (`amount - computed change`), matching the bill summary's collected semantics; the amount tendered is never used.
- The type filter options are short labels (`Cash`, `Online`) even though the payment sheet says `Online Payment`; pills use `Paid – Online`.
- The payment sheet's existing "Online Payment" option label is unchanged.
- No new dependencies.

Open questions (non-blocking; decide during implementation):

- Whether the Pay sheet's "Use full balance" button should be removed now that the field is prefilled (spec keeps it as a reset).
- Whether the minimum-payment error copy should use the currency's formatted amount or a bare number on narrow screens.
- Whether `ShareRow` should carry the type separately (for future reuse) in addition to the enriched status string.
- Whether the method readout should instead be limited to the rows visible after the name and status filters; the spec keeps it method-level for the whole bill because that is the useful "how much cash vs online" number.
