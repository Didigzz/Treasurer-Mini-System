# Billing System Improvements — Specification

**Status:** Draft, ready for implementation
**Date:** 2026-11-15
**App:** Boarding House Treasurer 1.1.0 → 1.2.0
**Scope:** Calendar date-picker fix, "Select all" tenant selection, Undo Payment
**Request:** Billing system bug fixes and feature improvements (calendar picker, Select All, reverse accidental payment confirmation)

This document is the complete, implementation-ready specification. It records what the code currently does, the decisions made during the clarification interview, the exact UX behavior, the edge cases, and the acceptance checks. No code has been changed yet.

---

## 1. Summary of decisions

| Topic | Decision |
|---|---|
| Calendar bug | Fix the shared sheet-opening wiring, not just an Android-specific symptom. Verify on desktop **and** Android. |
| Calendar visual design | Keep the existing month-grid, calendar cells, header, time controls, colors, and layout. Internal Slint structure/touch handling may change. |
| Calendar entry points | All five: billing month, due date, payment date + time, dashboard month, change-returned date. Also payment-detail returned-date correction. |
| Calendar after picking | Return to the originating sheet (payment, return-change, payment detail). Page-level pickers close as they do today. |
| Select all default | No tenants selected when New Bill opens (current behavior). |
| Select all control | Tri-state checkbox in the "Split among" section header row. |
| Count display | Section header caption, the Select-all row, and the sticky action bar. |
| Eligibility | All active boarders already loaded by `active_boarders()`. Archived boarders never appear; no duplicate-bill filtering. |
| Custom amounts | Preserved across select/deselect/select-all; never overwritten by Select All. |
| Existing Void | Renamed to **Undo payment** and reused; one reversal mechanism, no second accounting path. |
| Undo reason | Not collected. Confirmation only. Audit comes from the preserved payment record + reversal timestamp. |
| Undo scope | Share row: only when the share is **Paid**. Payment rows and payment detail: any active payment, including **Partial**. |
| Multi-payment shares | Undo opens a payment picker; the treasurer explicitly chooses which payment to reverse. No hard block. |
| Undo guards | Block when change was already returned; block when the bill is archived; block already-undone payments. Warn when change is still outstanding. |
| Change still outstanding | Allow, but warn in the confirmation that the change obligation will be cleared. |
| Archived bill | Undo unavailable until the bill is restored. |
| Audit trail | Mark the original payment **Undone**; keep amount, paid date, note, and legacy reason. No activity-log table. |
| Legacy voided rows | Display "Undone" everywhere; retain and show their stored reason. |
| Confirmation style | Centered alert dialog (new component), not a bottom sheet. |
| Tests | Core tests for selection helpers, undo rules, and balance/status recalculation, plus desktop and Android manual checklists. |
| Version | Bump to 1.2.0 (`version_code` 3). |
| Schema / backup | No new tables or columns; no backup format change. `voided_at` / `void_reason` are reused. |

---

## 2. Context and current behavior

### 2.1 Architecture

- Rust + Slint phone-first app. UI: `apps/mobile/ui/*.slint`; wiring and state: `apps/mobile/src/app/*.rs`.
- Domain, SQLite storage, money math, backup: `crates/treasurer-core`.
- One window (`AppWindow`) with screen ids: 0 Dashboard, 1 Boarders, 3 History, 4 Settings, 10 Bill, 11 Boarder ledger, 12 New bill.
- All overlays are bottom sheets owned by the window, except the overpayment notice which is a full-window dialog.
- `sheet` is a private `SheetKind` property in `apps/mobile/ui/app.slint`, set only by the Slint function `open-sheet(kind, index)` and cleared by `close-sheet()`.
- Amounts are integer minor units. Currency formatting and dates go through `treasurer-core::money`.
- Design tokens live in `apps/mobile/ui/theme.slint`; shared controls in `components.slint`; layout primitives in `layout.slint`.

### 2.2 Calendar flow today

- `apps/mobile/src/app/calendar.rs::open_calendar(target)` populates calendar state: target kind, selected date, year/month, month-only flag, time, title, and the 42-cell day model. It does **not** change the Slint `sheet` state.
- Date fields call the Slint callback `open-calendar(target)`; the UI handlers are in `apps/mobile/ui/app.slint`:
  - Dashboard period → `open-calendar(4)`
  - New bill billing month → `open-calendar(3)`
  - New bill due date → `open-calendar(1)`
  - Payment date (inside the pay sheet) → `open-calendar(2)`
- `CalendarSheet` only renders inside `if root.sheet == SheetKind.calendar`.
- The only places that actually set `SheetKind.calendar` are the payment-detail returned-date correction and the return-change date row, both of which call `open-sheet(SheetKind.calendar, -1)` explicitly.

### 2.3 Root cause of the calendar bug

The calendar is **never shown** when a date field is tapped. The field handlers call `root.open-calendar(...)`, which is a callback into Rust; Rust updates calendar properties but nothing sets `root.sheet = SheetKind.calendar`. The `CalendarSheet` block therefore never mounts. This is not Android-only even though the report came from an Android device. The static code path explains "nothing opens at all".

The fix must make opening the calendar a UI-state transition, and must preserve the existing visuals.

### 2.4 Payment reversal today

- `Treasurer::void_payment(payment_id, reason)` requires a non-empty reason.
- It marks `voided_at` and `void_reason`, keeps the row, recomputes change for the remaining active payments, and commits in one transaction.
- Voided payments are excluded from paid totals/balances by `recompute_payment_changes` / `compute_share_state`; share status becomes Paid/Partial/Unpaid from the remaining active payments.
- The bill screen shows "Void", opens a sheet titled "Void this payment?" with a required reason, and the payment list marks rows "Voided".
- The payment detail sheet shows a red banner "Voided: {reason}".
- There is no activity log. History is a monthly aggregation of bills, not a transaction log.

### 2.5 New-bill selection today

- `App::pick_rows: Vec<Pick>` is loaded from `db.active_boarders()` when the New Bill screen opens.
- Each `Pick` has `boarder_id`, `name`, `selected`, `amount`.
- The screen renders a `Checkmark` per row and a `pick-toggle(i)` callback; there is no select-all.
- `create_bill` uses only selected picks; typed amounts become pinned overrides; blank amounts share the remainder.
- Archived boarders are not in the list at all.

---

## 3. Requirement 1 — Calendar date picker

### 3.1 Goal

Every date the treasurer can enter opens a usable month-grid calendar, accepts taps on days, saves the value in the correct field, and displays it immediately. The calendar must work from pages and from inside other sheets, on desktop and Android.

### 3.2 Required changes

1. Add a Slint function in `AppWindow` (name may vary, e.g. `open-calendar-sheet(target: int)`) that:
   - calls `root.open-sheet(SheetKind.calendar, -1)`, then
   - calls `root.open-calendar(target)`.
2. Replace every direct `root.open-calendar(...)` field handler with this function. This is the actual fix for "cannot be clicked or opened". Do not rely on Rust setting Slint UI properties.
3. Add a small return-target state so the calendar can restore the sheet it came from:
   - `private property <SheetKind> calendar-return-to: SheetKind.none;`
   - `private property <int> calendar-return-index: -1;`
   - When opening the calendar from the pay sheet, the return-change sheet, or the payment-detail returned-date row, record the current sheet and index before switching to the calendar.
   - When the calendar closes/confirms, either restore that sheet or close to the page.
4. Keep `CalendarSheet`'s look and behavior. Internal changes are allowed only where needed for reliable touch handling (e.g. layering, hit areas). Do not introduce new colors; use `Theme` tokens.
5. Ensure the calendar sheet (and the new undo dialog) is declared after the navigation and other sheets in the window's stacking order, so it is never covered by the nav pill or the page.

### 3.3 Entry-point matrix

| Entry point | Screen / sheet | Target | Mode | Commit behavior | After selection |
|---|---|---|---|---|---|
| Dashboard period | Dashboard page | 4 | Month-only | Applies immediately | Close sheet |
| Billing month | New bill page | 3 | Month-only | Applies immediately | Close sheet |
| Due date | New bill page | 1 | Date, clearable | Applies immediately | Close sheet |
| Payment date + time | Pay sheet | 2 | Date + time | Only on **Confirm** | Return to Pay sheet |
| Change returned date | Return-change sheet | 5 | Date | Applies immediately | Return to Return-change sheet |
| Correct returned date | Payment detail sheet | 5 | Date | Applies immediately and refreshes detail | Return to Payment detail |

The existing `detail-correcting` behavior may be folded into the general return-target state, but the payment-detail behavior must not regress.

### 3.4 Detailed behavior

- **Month-only pickers** (billing month, dashboard month): header chevrons step years; tapping a month applies it and closes; the year/month chips remain usable; no day grid is shown.
- **Date pickers without time** (due date, change returned date, corrected returned date): tapping a day selects it, writes the ISO date, updates the visible label, and closes/restores the origin according to the matrix. "Today" behaves the same way.
- **Due date** additionally has a Clear action that empties `bill_due` and `bill_due_label`.
- **Payment date + time**:
  - Tapping a day highlights it and updates the pending calendar date but does **not** write the payment fields yet.
  - Time is edited with the existing hour/minute steppers and AM/PM control; the time label updates live.
  - **Confirm** writes `payment_date`, `payment_time`, and `payment_date_label`, then returns to the pay sheet.
  - Cancel/dismiss leaves the previously stored payment date/time untouched.
  - The pay sheet must still show the typed amount after returning. Do not reset `payment-amount` when the calendar closes.
- **Change returned date** picker: writes `return_change_date`, updates `return_change_date_label`, and returns to the return-change sheet so the treasurer can finish pressing "Confirm returned".
- **Payment-detail corrected date**: writes via `update_change_returned_at`, refreshes the bill/detail, and reopens the payment detail.
- **Month/year navigation** keeps clamping to 2000–2100, clamps month-end days correctly, and preserves the selected day when possible.
- **Invalid or empty stored values** fall back to today, as today's Rust code already does.

### 3.5 Touch and overlap requirements

- Every day cell has a minimum 44px tap target and is enabled only within the supported year range.
- The sheet scrim dismisses on outside tap; the panel swallows taps; day cells, chevrons, month/year chips, time controls, Clear, Today, Cancel, and Confirm all receive taps.
- Opening a calendar must not be intercepted by a parent `ScrollView`, a covering `TouchArea`, the navigation, or a keyboard. If the software keyboard is open (e.g. after typing an amount), the sheet must appear above it or dismiss the keyboard.
- The calendar must render correctly with the Android navigation bar/safe area using the existing `bottom-inset`.
- Do not add a duplicate/native second date picker. There is one shared calendar component.

### 3.6 Calendar edge cases

- Day grid has 42 cells; leading/trailing days from adjacent months are shown dimmed.
- Month-only mode never shows a day grid.
- Tapping an adjacent-month day selects that date and moves the shown month.
- Feb 29 / 30 / 31 month-end clamping must not panic or silently change the month.
- The same calendar instance opens fresh each time with the currently stored value preselected.
- Opening from an archived bill is not possible for the New Bill screen; the payment-detail correction remains available where the underlying row allows it.
- Back/Escape closes the calendar and returns to the origin sheet, not all the way to the page.

### 3.7 Acceptance criteria — calendar

1. Tapping each of the five date fields opens the calendar sheet on desktop.
2. Tapping a day selects it, and the date is visible in the field/label after closing.
3. Billing month and dashboard month apply immediately and refresh the relevant screen.
4. Payment date + time requires Confirm; Cancel leaves the prior value unchanged.
5. After choosing a payment date, the pay sheet is visible again with the amount still typed.
6. After choosing a change-returned date, the return-change sheet is visible again with the chosen date.
7. Due-date Clear empties the field; Today selects today.
8. No dead taps anywhere in the sheet; no overlap with the nav, keyboard, or scrim.
9. Same checks pass on the Android APK, not just desktop.
10. Existing calendar-related core tests (`calendar_weeks`, period/date formatting) still pass.

---

## 4. Requirement 2 — Select all when creating bills

### 4.1 Goal

The treasurer can select every eligible tenant with one tap, then deselect exceptions, with an always-visible selected count and a correct checkbox state.

### 4.2 Placement and appearance

In the New Bill screen's **"Split among"** section:

- Above the tenant `ListCard`, add a select-all header row:
  - tri-state checkbox (same 24px rounded style as the existing row `Checkmark`),
  - label "Select all",
  - right-aligned count "17 of 20 selected".
- The existing `SectionHeader` caption is prefixed with the same count, e.g.
  "17 of 20 selected. Leave amounts empty to split equally. A typed amount is fixed; the rest are split equally…"
- The sticky action bar shows the count on the left and the existing primary "Create bill" button on the right. On narrow widths (< 360px) the count may shorten to "17 selected"; it must not overlap or elide the button.
- The count uses existing typography tokens (`text-caption` / `text-sub`, `text-muted`).
- The checkbox is a real tri-state control, not just a checked/unchecked toggle:
  - all selected → checked (accent fill + check icon),
  - some selected → indeterminate (accent border + horizontal dash),
  - none selected → unchecked.
- Do not hide the count when zero: "0 of 20 selected".

### 4.3 Interaction and state transitions

`n` = number of eligible active boarders in `pick_rows`.

| Current state | Tap on Select all | Result |
|---|---|---|
| None selected (0/n) | Select all | All `n` selected |
| Partially selected (1..n-1) | Select all | All `n` selected |
| All selected (n/n) | Deselect all | None selected |

- Toggling an individual tenant updates the tri-state immediately.
- Select all only changes the `selected` flag; it never changes a tenant's typed custom amount.
- Deselecting a tenant leaves its typed amount stored in memory (so reselecting restores it). The amount of a deselected tenant is ignored at creation.
- Opening New Bill always starts with none selected (current behavior).
- With zero active boarders, the existing empty state is shown and the select-all row is hidden.
- With one active boarder, the control still works (0/1 ↔ 1/1); no special casing needed.
- The selected count is based on active rows, never on archived or excluded tenants. Archived tenants are not loaded into the form at all, and creation-time eligibility is unchanged (all active boarders, regardless of other bills for that type/month).

### 4.4 Data flow and Rust changes

- Keep `Pick { boarder_id, name, selected, amount }` as-is; add:
  - `select_all(&mut self)` (or equivalent) that applies the tri-state transition to all `pick_rows`,
  - `selected_pick_count() -> (usize, usize)` for UI state,
  - rendering of the select-all state/count into new `AppWindow` properties.
- Slint needs new properties, e.g. `picks-selected-count`, `picks-total-count`, `picks-all-selected`, `picks-some-selected`, and callbacks `select-all-picks()`.
- `create_bill` is unchanged: it filters `pick.selected`, applies typed overrides, validates at least one selection, and keeps the existing split rules.
- The checkbox state must be derived from the model, not stored separately, so it cannot drift.

### 4.5 Testable helpers

Extract the pure selection logic into `crates/treasurer-core/src/selection.rs` (new module, exported from `lib.rs`), with tests:

```rust
pub enum SelectionState { None, Some, All }

pub fn selection_state(selected: &[bool]) -> SelectionState;
// Empty slice => None.
// All true (and non-empty) => All.
// Otherwise => Some.

pub fn toggle_select_all(selected: &[bool]) -> Vec<bool>;
// All true => all false; otherwise => all true (preserves length and order).
```

The mobile layer uses these helpers so the tri-state and select-all behavior are covered by core tests even though the UI is not.

Optional helper: `selected_count(selected: &[bool]) -> usize`.

### 4.6 Accessibility

- Select-all is exposed as a checkbox with label "Select all tenants".
- The accessible label includes the count and partial state, e.g. "Select all tenants, 17 of 20 selected, partially selected".
- The visual dash for indeterminate is paired with the count text, so state does not rely on color alone.
- Individual tenant rows keep their existing "selected/not selected" accessible label.

### 4.7 Acceptance criteria — Select all

1. 20 active tenants → tap Select all → "20 of 20 selected", checkbox checked, all rows selected.
2. Deselect 3 → "17 of 20 selected", checkbox indeterminate, 17 rows selected.
3. Tap Select all from the partial state → back to 20/20.
4. Tap Select all from 20/20 → 0/20 and unchecked.
5. Type a custom amount for a tenant, deselect, reselect → the amount is still there.
6. Select all never overwrites or clears any typed amount.
7. Create bill with 17 selected submits 17 shares and ignores the deselected tenants and their amounts.
8. Archived tenants never appear and are never selected.
9. Counts appear in the section header, the select-all row, and the action bar, and agree at every state.
10. The control is usable with one tap on both desktop and Android, including at the 320px minimum width.

---

## 5. Requirement 3 — Undo Payment

### 5.1 Goal

After a payment is confirmed, the treasurer can reverse an accidental confirmation with a clear confirmation step. The original record is preserved, balances and statuses are recalculated, and the reversal is visible for audit. This replaces the existing "Void" flow, which is the same operation under a clearer name.

### 5.2 Product decision

- Rename **Void** to **Undo payment** everywhere in the UI.
- Reuse the existing soft-delete mechanism (`voided_at`), not a delete and not a new refund/credit transaction.
- Do not collect a reason; the request's dialog is Cancel / Confirm Undo only.
- Do not build a separate activity log. The payment row itself is the audit record: preserved amount, paid date/time, note, original creation time, reversal timestamp, and any legacy reason.
- This is an accidental-confirmation correction, not a refund. Do not add refund accounting, refund payouts, or "refund" wording.

### 5.3 Entry points

| Entry point | When offered | Target payment |
|---|---|---|
| Bill screen → tenant/share row | Share status is **Paid** | If one active payment: that payment. If several: open the payment picker. |
| Bill screen → payment row | Any active payment (Paid or Partial) | That specific payment |
| Boarder ledger → payment row | Any active payment (Paid or Partial) | That specific payment |
| Payment detail sheet | Any active payment (Paid or Partial) | The open payment |
| Archived bill | Not offered (blocked) | — |

- Share-row action label: **Undo payment** (compact secondary/danger-soft button in the row's action area).
- Payment-row action: replace the existing "Void" button with **Undo payment**.
- Payment-detail action: a new **Undo payment** button above "Close", styled as a destructive secondary action.
- Do not offer undo from the dashboard balance list in this release.

### 5.4 Guard rules

Undo is rejected by the domain layer (not just hidden in the UI) when:

1. The payment does not exist.
2. The payment is already undone.
3. The payment's change was already marked returned.
4. The bill that owns the share is archived.

Undo is **allowed** when:

- The payment is partial (share status Partial).
- The payment has change still outstanding (not returned) — allowed with a warning; undoing clears that change obligation.
- Other active payments exist on the share — the treasurer must select the target explicitly through the picker, but it is not blocked.

There is no time-window, role, or "same day" restriction. The app has one treasurer and no accounting period lock.

Error copy:

- Already undone: "This payment was already undone."
- Change returned: "This payment can't be undone because its change was already returned."
- Archived bill: "Restore this bill before undoing a payment."
- Missing/stale row: "That payment is no longer available. Go back and reopen the bill."
- Not found in DB: "Payment not found."

### 5.5 Confirmation dialog

A centered alert dialog, above sheets and navigation, with a scrim. Do not use the existing bottom Sheet for this confirmation.

- Title: **Undo Payment Confirmation**
- Body: **Are you sure you want to reverse this payment confirmation? The tenant's payment status will return to Unpaid.**
- Context line: the tenant name, bill type and period, and the payment amount/date.
- If other active payments remain on the share, append:
  "Other payments remain on this share, so the status will be recalculated."
- If the payment has change still outstanding, show a warning line:
  "This payment has {amount} of change to return. Undoing it clears that change obligation."
- Buttons:
  - **Cancel** — secondary, closes the dialog and changes nothing. Initial focus.
  - **Confirm Undo** — danger, performs the reversal.
- Escape / Android back cancels.
- While the action runs, the buttons may disable to prevent double submits.
- On success: close the dialog/picker, refresh the bill, payment list, ledger, dashboard, history, and open payment detail, then show "Payment undone."
- On failure: keep the dialog open and show the error inside it (or in the standard banner), without changing any data.

Recommended copy for the audit line in the dialog:
"The original payment record stays in history marked Undone."

### 5.6 Multi-payment picker

When Undo is invoked from a share row whose share has more than one active payment:

- Open a sheet titled **Undo which payment?**
- Subtitle: "Choose the payment to reverse for {tenant}."
- List active payments for that share, newest first, each showing paid date/time and amount, plus a "Newest" tag on the newest.
- Selecting a row opens the centered confirmation dialog for that payment.
- Cancel returns to the bill without changes.
- Use existing `Sheet`, `ListCard`, `ListRow`/`SheetOption` components and theme tokens.

When the share has exactly one active payment, skip the picker and open the dialog directly.

### 5.7 Reversal mechanics

Domain/repository:

- Replace `void_payment(payment_id, reason)` with `undo_payment(payment_id) -> Result<()>` (or keep a compatibility wrapper for backups/tests).
- Transactionally:
  1. Load the payment and its share/bill.
  2. Validate the guards above.
  3. Set `voided_at = now`; leave `void_reason` untouched (NULL for new undos; legacy text retained).
  4. Recompute change for all remaining active payments on the share using `recompute_payment_changes` and persist any changed `change_minor`.
  5. Commit.
- Do not modify `amount_minor`, `paid_at`, `note`, `created_at`, `change_returned_at`, or `change_return_note` on the undone row. The record stays exactly as entered, plus the reversal timestamp.
- Do not delete the row and do not create a negative "refund" payment.
- No schema migration; `voided_at` remains the reversal marker. `void_reason` remains for legacy data and backup compatibility.

UI/read model:

- `is_active()` continues to mean "not undone".
- Payment rows show a neutral **Undone** status pill and muted amounts.
- The bill payment subtitle changes " · voided: {reason}" to " · undone" (or " · undone: {reason}" for legacy rows).
- The ledger payment subtitle changes " · voided" to " · undone".
- The payment detail banner changes from "Voided: {reason}" to:
  - "Undone" (new records), or
  - "Undone — {reason}" (legacy records with a stored reason).
- Add an "Undone on" detail row using `voided_at`, formatted with the existing date/time helpers.
- Hide/disable Undo on an undone payment; hide Return change for undone rows.
- Share status, expected/collected/outstanding, change-to-return, dashboard totals, boarder balances, and history monthly totals all refresh from the same recomputation; no manual arithmetic in the UI.

### 5.8 Audit requirements

- The original payment remains in the bill's payment list and the boarder ledger after undo.
- It is excluded from collected totals and from the share's paid amount.
- The reversal timestamp (`voided_at`), original paid timestamp, amount, note, and legacy reason remain readable in the UI and survive export/restore.
- History bills' monthly totals reflect the undo because they are computed from active payments.
- The app does not claim an activity log; the spec deliberately uses the payment record as the audit trail.

### 5.9 Edge cases

- **Paid share, one payment** → undo → Unpaid; change obligation gone.
- **Paid share, several payments** → picker → undo one → Partial or Unpaid depending on the remainder; remaining payments' change recomputed.
- **Partial share** → undo from payment row/detail → Unpaid or still Partial; allowed.
- **Overpayment with change still out** → confirmation warns; undo clears the pending change.
- **Change already returned** → blocked with a clear error; do not silently create a negative balance.
- **Archived bill** → Undo hidden/disabled; domain rejects with "Restore this bill before undoing a payment."
- **Undo twice** → second attempt is rejected as already undone.
- **Additional charge** → the share balance includes it; undo recalculates from the remaining payments; no special handling beyond the standard recompute.
- **Payment detail open while undo happens** → detail refreshes and shows the Undone banner/row.
- **Bill reached from a boarder ledger** → undo refreshes the ledger; Back still returns to the ledger.
- **Legacy voided payment restored from backup** → shown as Undone, reason retained; its `voided_at` drives the audit line.
- **Restore flow** → backup validation continues to accept `void_reason`/`voided_at` as before.

### 5.10 Acceptance criteria — Undo

1. Mark a tenant Paid, then Undo from the share row. Dialog appears with the specified title/body/buttons.
2. Cancel changes nothing; Confirm Undo flips the share to Unpaid and removes the amount from collected totals.
3. The payment remains in the list, marked Undone, with amount and paid date intact.
4. The payment detail shows Undone and the reversal date; the stored reason (legacy) is still visible.
5. Dashboard, boarder ledger, bill summary, and History totals update immediately.
6. A partial payment can be undone from its payment row/detail.
7. A share with two payments opens the picker; choosing one reverses only that one; the status is recalculated.
8. A payment with change already returned cannot be undone and shows the guard message.
9. A payment with pending change can be undone after a warning; the change obligation disappears.
10. An archived bill offers no working Undo; the domain rejects attempts.
11. Undo requires no typed reason and cannot be triggered without confirmation.
12. Undo survives backup/export/restore and remains visibly Undone.

---

## 6. Cross-cutting requirements

### 6.1 Design system

- Use only `Theme` tokens for colors, spacing, radii, typography, and motion. No hard-coded hex values in screens.
- Keep the existing light/dark palettes, warm ledger surfaces, deep-green accent, and semantic tones.
- Status labels are always paired with text, not color alone.
- Preserve the existing page structure, cards, bottom sheets, and navigation.
- New controls must look native to the current design: 24px tri-state checkbox, 44px minimum touch targets, `radius-md`/`radius-lg`, existing button kinds.

### 6.2 Platform and responsiveness

- Desktop preview and Android APK must both work.
- Minimum window width 320px; content centered with the existing `Responsive` gutter on wide screens.
- Counts and new actions must wrap/elide gracefully on narrow phones.
- Safe-area insets and on-screen keyboard behavior must remain correct, especially for the pay sheet → calendar flow.

### 6.3 Data integrity

- No destructive deletes for payments.
- All multi-row writes in one transaction.
- No new tables or columns; no backup format/version change.
- Money remains integer minor units; no floats introduced in calculations.
- Existing split, charge, change, archive, and restore behavior must not regress.

### 6.4 Accessibility

- Tri-state select-all exposes checked/indeterminate state through its accessible label.
- Undo button labels include the target, e.g. "Undo payment for Maria Santos, ₱1,000.00, 15 Nov 2026".
- Confirmation dialog is labeled, traps focus visually, and defaults to Cancel.
- Calendar day cells keep their existing accessible labels and 44px targets.

### 6.5 Documentation and version

- Bump `apps/mobile/manifest.yaml`: `version_name: 1.2.0`, `version_code: 3`.
- Update the Settings version string in `apps/mobile/src/app/mod.rs` from "1.1.0" to "1.2.0".
- Update README feature bullets: replace "void a payment with a required reason" with Undo payment; add Select all and the calendar fix/undo notes where appropriate.
- Do not rewrite the README structure.

### 6.6 Out of scope

- Refunds as a separate accounting action.
- A general activity/audit log screen or table.
- Undo from the dashboard Paid list.
- Time-window or role-based restrictions.
- Cloud sync, accounts, multi-house support.
- Redesigning the calendar visuals or replacing it with a native picker.
- Changing split math, additional charges, change-return accounting, backup format, or duplicate-bill rules.
- Bulk undo of multiple payments.

---

## 7. Test plan

### 7.1 Automated (Rust core)

Run: `cargo test -p treasurer-core`

Selection (`crates/treasurer-core/src/selection.rs`):

- Empty selection → `None`.
- All false → `None`.
- Mixed → `Some`.
- All true → `All`.
- `toggle_select_all` from none/partial → all true; from all → all false; length/order preserved.
- `selected_count` matches.

Undo (repository tests alongside the existing payments/bills tests):

- Undoing the only payment flips the share to Unpaid; payment remains with `voided_at` set; summary `applied_minor` excludes it.
- Undoing one of two payments leaves the share Partial; remaining payment's change is recomputed.
- Undoing an overpayment with pending change clears the change obligation and its `change_minor`.
- Guard: payment with `change_returned_at` set cannot be undone.
- Guard: archived bill cannot be undone.
- Guard: second undo of the same payment fails.
- Legacy `void_reason` is preserved and survives a backup round trip.
- Backup validation/restore still accepts undone/voided rows.

### 7.2 Compile check

- `cargo check -p treasurer-mobile` (compiles the Slint UI and Rust wiring).
- Optionally `cargo build -p treasurer-mobile` for the desktop preview.

### 7.3 Desktop manual script

1. Dashboard: open month picker, choose another month, verify the dashboard refreshes and the calendar closes.
2. New bill: open billing month, choose a month; open due date, choose a date, verify the label; open again, Clear, verify empty.
3. Record a payment: type an amount, open the payment date, choose a day, adjust time, Confirm; verify the pay sheet is back, the amount is still there, and the date/time label is correct.
4. Return change: open the date picker, choose a date; verify the return-change sheet is back with the date, then confirm.
5. Payment detail: correct the returned date; verify the calendar returns to the detail and the date updates.
6. New bill with 20 active boarders: verify 0/20; Select all → 20/20; deselect 3 → 17/20 indeterminate; reselect via Select all; type custom amounts and toggle rows.
7. Record payment, open the payment detail, Undo, Cancel, then Undo → Confirm; verify status, totals, and the Undone row.
8. Repeat undo for a partial payment and for a share with two payments (picker).
9. Return change on a payment, then attempt Undo; verify the guard message.
10. Archive the bill, attempt Undo; verify it is unavailable; restore and repeat.

### 7.4 Android APK checklist (required, since the report came from a device)

Build:

```bash
export PATH="$LOCALAPPDATA/Android/Sdk/ndk/27.1.12297006/toolchains/llvm/prebuilt/windows-x86_64/bin:$PATH"
(cd apps/mobile && x build --platform android --arch arm64 --format apk)
adb install -r target/x/debug/android/treasurer-mobile.apk
```

Then on the device:

- Open every date field; confirm the calendar appears above the keyboard/nav.
- Tap days, month/year chips, Today, Clear, Confirm; confirm each works.
- Confirm the payment amount survives the calendar round trip.
- Exercise Select all with 20+ tenants and individual deselection; confirm touch targets and counts.
- Exercise Undo from share row, payment row, and payment detail; confirm the centered dialog and both buttons.
- Confirm guard messages for returned change and archived bills.
- Confirm the app does not crash, freeze, or lose data after undo.
- Record the device/Android version in the implementation notes.

If no device is available, the implementation is not considered fully verified; report that limitation explicitly rather than claiming an Android pass.

### 7.5 Definition of done

- All acceptance criteria in sections 3.7, 4.7, and 5.10 pass.
- `cargo test -p treasurer-core` passes.
- `cargo check -p treasurer-mobile` passes.
- Desktop manual script passes.
- Android checklist passes on a real device (or is explicitly reported as unverified).
- Version is 1.2.0 in manifest and Settings.
- No unrelated files, features, or schema changes are included.

---

## 8. Implementation plan (suggested order)

1. **Calendar wiring**
   - Add `open-calendar-sheet(target)` and return-target state in `apps/mobile/ui/app.slint`.
   - Replace the four direct field handlers; keep the payment-detail/return-change explicit paths working.
   - Keep the calendar visuals; adjust layering/hit areas only as needed.
2. **Payment/return-change return flow**
   - Restore the originating sheet after date selection; stop resetting `payment-amount`.
   - Verify Cancel leaves prior values untouched.
3. **Selection helpers**
   - Add `crates/treasurer-core/src/selection.rs`, export it, and add unit tests.
4. **Select-all UI**
   - Extend the row `Checkmark` to support indeterminate.
   - Add the select-all header row, count properties, callbacks, and the action-bar count.
   - Add `select_all` / count methods to `App`; keep `create_bill` unchanged.
5. **Undo domain**
   - Add `undo_payment` with guards and transactional recompute; retire the mandatory reason.
   - Add repository tests.
6. **Undo UI**
   - Replace Void labels with Undo payment on payment rows, share rows, and payment detail.
   - Add the multi-payment picker sheet.
   - Add the centered `UndoPaymentDialog` component and error/warning states.
   - Wire callbacks and refresh paths (bill, ledger, dashboard, history, open detail).
7. **Legacy labels**
   - Sweep "Voided"/"voided" UI strings in `bills.rs`, `boarders.rs`, `bill.slint`, and `app.slint` to "Undone"/"undone"; keep stored `void_reason` visible.
8. **Version and docs**
   - Bump manifest and Settings version; update README bullets.
9. **Verify**
   - Run core tests and the mobile compile; run the desktop script; build/install the APK and run the device checklist.

---

## 9. Resolved interview decisions (audit of choices)

- Calendar failure observed on Android only; it opens nothing at all; only the calendar sheet is affected (other sheets work); all five date fields must work; visuals stay, internals may change; cross-platform fix; Android checklist is the acceptance proof.
- Select all starts with none selected; all active tenants are eligible; typed custom amounts are preserved and editable after selecting all; counts in header caption, select-all row, and action bar; checkbox in the section header.
- One reversal mechanism: rename Void → Undo payment, no reason field, centered dialog; audit on the payment record; legacy records read Undone.
- Undo from share row (Paid only), payment rows, and payment detail; partial payments allowed from payment lists; multi-payment shares use a picker; pending change warns but is allowed; change already returned and archived bills block.
- Version 1.2.0; core tests for selection and undo; desktop + Android verification.

## 10. Assumptions and open questions

Assumptions (no further input needed):

- "Paid" means `ShareStatus::Paid` (balance zero); "Partial" means balance > 0 with active payments.
- The existing saved reason field is no longer requested but is never discarded; new undos leave it empty.
- The original payment's `paid_at` and amount remain unchanged; `voided_at` is the reversal timestamp.
- The undo action and labels are English, matching the rest of the UI.
- The calendar return-to-origin behavior applies only when the calendar was opened from a sheet; page-level pickers still close on selection.
- No new dependencies.

Open questions (none blocking; decide during implementation if needed):

- Whether the action bar count should read "17 of 20 selected" or "17 selected" at widths between 320–360px (spec allows either, as long as it fits).
- Whether the multi-payment picker lists newest first with a "Newest" tag or all rows equally (spec recommends the tag).
- Whether legacy "Voided" wording should remain in exported JSON keys (yes: keep `void_reason`/`voided_at` for compatibility).
