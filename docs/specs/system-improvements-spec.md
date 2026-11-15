# Calendar Pickers, Post-Creation Bill Management, Overpayment Dialog, and Change Tracking — Feature Spec

**Status:** Draft, ready for implementation
**Request short name:** system-improvements
**Platform:** `apps/mobile` (Rust + Slint; desktop preview and Android), domain/storage in `crates/treasurer-core`
**Scope:** The four requested features, expressed against the app that actually exists: no reservations module, no separate tenant-record dates. "Tenant" is a **boarder**, "bill" is a monthly shared bill.

---

## 1. Summary

1. **Calendar date pickers.** Every field where the treasurer enters a date stops being free text and opens a month-grid calendar sheet drawn in the app's own design language. Applies to bill due dates, payment date + time, the billing month on New Bill, and month switching on the Dashboard, plus the change-returned date. Typing dates is removed.
2. **Bill editing and exclusion move fully after creation.** New Bill is only for composing a bill. On an existing bill, a **Manage shares** sheet offers Edit, Exclude from this bill, and Add boarder. Excluding a boarder keeps the bill total and re-splits it equally among the remaining boarders.
3. **Overpayment dialog.** Recording a payment that creates change shows a non-auto-dismissing, centred dialog: "Overpayment Received!" with the amount paid and the excess. X or an outside tap dismisses it. The payment is committed before the dialog is shown and is never rolled back.
4. **Change-return tracking.** Change is a first-class state on each payment: **Change to Return** until confirmed, then **Change Returned**, with the returned amount and date recorded. A payment detail sheet shows Total Bill / Amount Received / Change Returned / Net Payment / Payment Status / Change Status. A bill can be fully paid while change is still outstanding.

No schema change is required: `payments.change_minor`, `payments.change_returned_at`, and `payments.change_return_note` already exist and change is already recomputed per payment. This is UI plus two new repository operations (`exclude_share`, `add_share_to_bill`) and a corrected-date update.

---

## 2. Request restated against this codebase

### 2.1 Calendar date pickers

> "Add a calendar/date picker to every field that requires the user to enter a date."

Today dates are typed:

| Field | Current control | Storage |
| --- | --- | --- |
| Bill due date (New Bill) | `TextField`, placeholder `YYYY-MM-DD`, validated in `create_bill` | `bills.due_date` ISO `YYYY-MM-DD`, optional |
| Payment date (Record payment sheet) | `TextField`, hint "Leave empty for today, or enter YYYY-MM-DD." | `payments.paid_at` epoch ms, set to **noon local** of the chosen day |
| Billing month (New Bill) | Two numeric `TextField`s, Month + Year | `bills.period_year`, `bills.period_month` |
| Dashboard month | `‹` / `›` buttons only, no direct choice | `App.year` / `App.month` |
| Change returned date | Not editable; always "now" when marking returned | `payments.change_returned_at` epoch ms |
| Last backup (Settings) | Read-only formatted string | `settings.last_backup_at` ISO timestamp |

### 2.2 Bill editing and exclusion

> "Move the Manual Edit and Exclude Tenant options to the stage after a bill has been created."

- Bill detail already has a manual edit path: tap a share row → **Edit share** sheet (share amount + additional charge, with live split preview).
- There is **no** exclude action anywhere in the UI. `update_bill` can drop shares but is unreachable.
- New Bill lets you choose which boarders are on the bill and optionally type per-boarder amounts.

### 2.3 Overpayment popup

Currently an overpayment is silent apart from a banner: "Payment of ₱655.00 recorded." and the change appears in the share row and Payments list. Nothing tells the treasurer that change is owed at the moment of payment.

### 2.4 Change return tracking

Already modelled and already per-payment:

- `PaymentRecord.amount_minor`, `.change_minor`, `.change_returned_at`, `.change_return_note`.
- `recompute_payment_changes` assigns the overpayment portion to the payment that crosses the amount due.
- `compute_share_state` yields `change_outstanding_minor`, `change_returned_minor`, `status`.
- `mark_change_returned(payment_id, returned_at, note)` exists; UI wires it to a one-tap "Mark change returned" button.
- Dashboard already has a **Change** filter and a "Change due" metric; bill rows already say "… change to return".

What is missing: explicit **Change to Return** / **Change Returned** status labels, a confirmation step with a chosen date, a per-payment transaction breakdown, and the ability to correct the returned date.

---

## 3. Verified current state

| Area | Fact | File |
| --- | --- | --- |
| Design tokens | Warm-paper surfaces, deep-green accent, semantic tones, 4pt spacing, radii, motion durations; light/dark via `Theme.dark` | `apps/mobile/ui/theme.slint` |
| Shared widgets | `Card`, `TapCard`, `AppButton`, `IconButton`, `TextField`, `SelectField`, `Toggle`, `Segmented`, `StatusPill`, `ProgressBar`, `Banner`, `Sheet`, `SheetOption`, `EmptyState` | `apps/mobile/ui/components.slint` |
| Icons | `Icons.calendar` and `Icons.clock` already exist; no new icons needed | `apps/mobile/ui/icons.slint` |
| Shell | `AppWindow` holds all screen state and window properties; `SheetKind` enum; sheets rendered as direct window children so they cover nav; `go-back()` closes sheet → screen → app | `apps/mobile/ui/app.slint` |
| Screens | 0 Dashboard, 1 Boarders, 3 History, 4 Settings, 10 Bill, 11 Ledger, 12 New bill | `apps/mobile/src/app/mod.rs`, `ui/app.slint` |
| Payment record path | `pay_share` parses amount + date, calls `db.record_payment`, refreshes detail/dashboard, then `notify(...)` | `apps/mobile/src/app/bills.rs` |
| Payment date parsing | `parse_payment_date` → empty means now, otherwise `%Y-%m-%d` at 12:00 local | `apps/mobile/src/app/bills.rs` |
| Repository payments | `record_payment`, `void_payment`, `mark_change_returned` (bails if voided/already returned/no change) | `crates/treasurer-core/src/db/repository/payments.rs` |
| Repository bills | `create_bill`, `update_bill` (updates in place, deletes removed shares), `save_share_edit` (locked bills refuse amount changes), `set_bill_archived`, `update_bill_meta` | `crates/treasurer-core/src/db/repository/bills.rs` |
| Locking | `BillWithDetails.is_locked` = any active payment on any share | `crates/treasurer-core/src/db/repository/loading.rs` |
| Split math | `equal_split`, `split_with_overrides`, `resplit_with_pinned`, `validate_additional_charge`; leftover minor units go to the earliest boarders | `crates/treasurer-core/src/calculations.rs` |
| Money/date helpers | `format_date`, `format_datetime`, `format_date_only`, `to_iso_date`, `today_iso_date`, `month_key`, `parse_month_key`, `shift_period` | `crates/treasurer-core/src/money.rs` |
| Ledger | Reuses `root.shares` / `root.payments` and the shared `PaymentItem` row | `ui/screens/ledger.slint`, `ui/screens/bill.slint` |
| Backup | JSON v1 with strict validation; shares must sum to bill total; payments carry `changeMinor` / `changeReturnedAt` | `crates/treasurer-core/src/backup.rs` |
| Conventional spec style | Worked examples, "Current state (verified)", interview decisions table, data model, UI spec, open items | `docs/specs/bill-custom-split-spec.md` |

---

## 4. Interview decisions (source of truth)

| # | Decision | Answer |
| --- | --- | --- |
| 1 | Calendar widget type | **Month-grid bottom sheet**, same sheet shell used everywhere |
| 2 | Which fields get pickers | **Every date the treasurer enters**, including bill period, Dashboard month, and the change-returned date |
| 3 | Typed dates | **Removed** — date fields become read-only rows that open the picker |
| 4 | Payment time | **Date + time**, stored in `paid_at` |
| 5 | Payment time UI | **Combined**: calendar in the sheet, with a Time section (hour/minute wheels + AM/PM) expanded in the same sheet |
| 6 | Excluding a tenant | **Per-bill only.** The boarder stays active on every other bill and stays selectable on future bills |
| 7 | Exclude money effect | **Remove the share, keep the bill total, re-split equally among the remaining boarders** |
| 8 | Exclude entry point | **Bill-level "Manage shares" sheet** listing every boarder with Edit and Exclude |
| 9 | New Bill custom amounts | **Kept.** Per-boarder amounts at creation are split setup, not post-hoc editing |
| 10 | Exclude + already paid | Not a normal workflow. Defensive rule: refuse while payments exist (already implied by the bill lock) |
| 11 | Add boarder after creation | **Yes** — same sheet, re-splits the same bill total across the new set |
| 12 | Overpayment popup trigger | **Every payment that creates change**, even if the share already had change outstanding |
| 13 | Popup actions | Message + **"Return change now"** primary and **"Later"** secondary, plus X |
| 14 | Popup dismissal | X, outside tap, Back/Escape. **Never automatic** |
| 15 | Change status visibility | **Payment rows + share rows** (pill on rows, explicit line on shares) |
| 16 | Transaction breakdown | **Read-only payment detail sheet**, opened from any payment row (bill and ledger) |
| 17 | Confirm change returned | **Confirmation sheet** showing the amount + "Returned on" date picker |
| 18 | Calendar week start | **Sunday** |
| 19 | Selectable years | **2000–2100** (matches existing validation) |
| 20 | Payment detail reach | **Everywhere payments are listed**: bill screen and boarder ledger |

---

## 5. Feature 1 — Calendar date pickers

### 5.1 New shared components (`ui/components.slint`)

Three components, all built only from existing tokens/patterns, no new dependency (no calendar crate):

```slint
export enum DateTarget { none, bill-due, payment-date, bill-period, dashboard-period, change-returned }

export component CalendarCell {
    in property <int> day;
    in property <bool> in-month;
    in property <bool> selected;
    in property <bool> today;
    in property <bool> enabled: true;
    callback picked();
}
```

- Cell: 44 × 44 px (Theme.touch), radius `Theme.radius-sm`, day label `Theme.text-body`.
  - selected → background `Theme.accent`, text `Theme.on-accent`, weight semibold.
  - today (not selected) → 1 px `Theme.accent` border, text `Theme.accent`.
  - outside month → text `Theme.text-faint`, still tappable (selecting it pages the month).
  - disabled (outside 2000–2100) → opacity 0.35, no touch.
- Weekday header row: `S M T W T F S`, `Theme.text-caption`, `Theme.text-muted`, centered, 44 px per column.
- Grid is 6 rows × 7 columns always (fixed height so the sheet does not jump between months).

```slint
export component CalendarSheet {
    in property <string> title;            // "Due date", "Payment date", "Returned on", …
    in property <DateTarget> target;
    in property <int> year;
    in property <int> month;               // 1–12
    in property <string> selected-iso;     // "2026-11-30" or ""
    in property <[CalendarCell]> days;      // 42 entries from Rust
    in property <bool> month-only: false;  // billing period / dashboard month
    in property <bool> allow-clear: false; // optional dates
    in property <bool> include-time: false;
    in-out property <int> hour: 12;        // 1–12
    in-out property <int> minute: 0;       // 0–59
    in-out property <bool> pm: false;
    callback prev-month();
    callback next-month();
    callback zoom();                       // month-year title → year/month chooser
    callback pick(string iso-date);
    callback today();
    callback clear();
    callback close();
    callback confirm();
    callback time-changed(int hour, int minute, bool pm);
}
```

Layout inside the sheet panel (existing `Sheet` shell, `bottom-inset: root.bottom-inset`):

```
[ drag handle ]
[ Due date                        (X) ]          title-bold + IconButton close
[ ‹   November 2026   › ]                        chevrons 44px, title tappable
[ S   M   T   W   T   F   S ]
[ day grid 6×7 ]
[ Time       2:35 PM          › ]                only when include-time
[ Clear ]        [ Today ]        [ Done ]        Clear only when allow-clear
```

- `month-only: true` replaces the day grid with a 3 × 4 grid of month names (`January`…`December`, 48 px chips), year stepper in the header. Used for the billing month and the Dashboard month.
- Fixed `Sheet` panel width already caps at 560 px, centred; on phones `Responsive.compact` shrinks cell and label spacing (`Theme.space-2`), grid stays 7 columns.
- All colours/typography come from `Theme` so dark mode needs no extra work. Light/dark parity must be checked on both.
- Accessibility: grid container `accessible-role: groupbox`, label = title; each cell `accessible-role: button`, label "November 30, 2026, selected"; header chevrons labelled "Previous month"/"Next month"; month chips `accessible-role: radio-button`.

### 5.2 Interaction

1. Tapping a date row (a `SelectField`-style read-only field) opens the calendar at the current stored value, or at today when empty.
2. **Day tap** selects and, for date-only fields, closes the sheet immediately (one tap to choose).
3. For the combined payment sheet (`include-time: true`), a day tap selects but keeps the sheet open; the **Done** button commits date + time. Time stays default at the current clock time when the field was empty.
4. Header chevrons step one month, never more than 2000-01 ↔ 2100-12 (chevrons disable at the edges).
5. Tapping "November 2026" toggles the **month/year chooser** inside the same sheet: a year strip (12 years per page, `‹ 2016–2027 ›`) then the 12-month grid. This is how a treasurer jumps years without 100+ taps.
6. **Today** jumps to today's month and selects today.
7. **Clear** empties an optional date (bill due date). Absent for required/settled dates (payment date, change-returned date).
8. No typing anywhere: the fields are read-only rows (`combobox` accessible role, visible `Icons.calendar`, or `Icons.clock` for the time line).

### 5.3 Field inventory

| Field | Target | Sheet mode | Display when set | Clear |
| --- | --- | --- | --- | --- |
| New Bill → Due date | `bill-due` | day grid, 2000–2100 | `Nov 30, 2026` | yes (optional in storage) |
| New Bill → Billing month | `bill-period` | `month-only` | `November 2026` | no (required) |
| Record payment → Payment date | `payment-date` | day grid + time | `Nov 15, 2026 · 2:35 PM` | no |
| Dashboard month switcher | `dashboard-period` | `month-only` | existing period label | no |
| Confirm change returned → Returned on | `change-returned` | day grid | `Nov 15, 2026` | no |
| Payment detail → Returned on (already returned) | `change-returned` | day grid, edits an existing date | `Nov 15, 2026` | no |
| Settings → Last backup | — | — | existing formatted date | not editable (see §11) |

New Bill's Month and Year `TextField`s are replaced by a single **Billing month** row. The "Due date (optional)" `TextField` becomes a **Due date** row with placeholder "No due date". The payment sheet's "Payment date" `TextField` (and its hint) becomes a **Payment date** row whose default value is "now", reset each time the pay sheet opens.

### 5.4 Supporting Rust helpers

In `crates/treasurer-core/src/money.rs` (pure, unit-testable, `chrono` already a dependency):

```rust
pub const MIN_YEAR: i32 = 2000;
pub const MAX_YEAR: i32 = 2100;

/// 42 cells starting on the Sunday on or before the 1st of the month.
pub fn calendar_weeks(year: i32, month: i32) -> Vec<(NaiveDate, bool /* in_month */)>;

/// Local date + wall-clock time to epoch millis. Falls back to `single()`,
/// then `earliest()`, then noon when the local time is ambiguous/skipped (DST).
pub fn local_datetime_ms(date: NaiveDate, hour: u32, minute: u32) -> i64;

/// Splits an epoch into (date, hour12, minute, pm) using the local zone.
pub fn split_local_datetime(epoch_ms: i64) -> (NaiveDate, u32, u32, bool);

pub fn add_months(year: i32, month: i32, delta: i32) -> (i32, i32); // clamps to MIN_YEAR..=MAX_YEAR
pub fn iso_date_to_epoch_noon(iso: &str) -> Option<i64>;             // replaces parse_payment_date
```

- `format_date_only`, `format_datetime`, `to_iso_date`, `today_iso_date` are reused for display, so no new formatting vocabulary leaks into the UI.
- `parse_payment_date` in `apps/mobile/src/app/bills.rs` is deleted; all payment dates now come from the picker.

### 5.5 Slint state and plumbing (`ui/app.slint` + `src/app/*.rs`)

New window properties:

```slint
in-out property <DateTarget> calendar-target: DateTarget.none;
in property <string> calendar-title: "";
in property <int> calendar-year: 2026;
in property <int> calendar-month: 11;
in property <string> calendar-selected: "";
in property <bool> calendar-month-only: false;
in property <bool> calendar-allow-clear: false;
in property <bool> calendar-include-time: false;
in property <int> calendar-hour: 12;
in property <int> calendar-minute: 0;
in property <bool> calendar-pm: false;
in property <[CalendarCell]> calendar-days: [];

callback open-calendar(DateTarget);
callback calendar-prev-month();
callback calendar-next-month();
callback calendar-zoom();
callback calendar-pick(string);
callback calendar-today();
callback calendar-clear();
callback calendar-confirm();
callback calendar-time-changed(int, int, bool);
```

- One new `SheetKind` value: `calendar`. It renders `CalendarSheet` as a direct window child like the other sheets, so it covers the nav and responds to Back.
- Rust owns the pending value: `App.calendar_target`, `calendar_date: NaiveDate`, `calendar_hour/min/pm`, and a scratch slot until confirm. Opening the sheet computes `calendar_days` via `calendar_weeks` and fills the title/labels.
- `calendar-pick` → day-only targets write straight to the corresponding window property:

| Target | On confirm/pick |
| --- | --- |
| `bill-due` | `bill-due = iso` |
| `payment-date` | `payment-date = "YYYY-MM-DD"`, `payment-time` = "HH:MM" (24h string, new window property) |
| `bill-period` | `bill-year` / `bill-month` |
| `dashboard-period` | `App.year` / `App.month`, then `refresh_dashboard()` |
| `change-returned` | `change-return-date` (new window property, used by the confirm sheet) or the stored date reported back to Rust for a correction |

- Because `payment-date` now carries a real time, `CreatePaymentInput.paid_at` is built from `local_datetime_ms(date, hour, minute)`. Empty payment date still means "now" only for safety; the picker always shows a value.
- Date displays inside the bill summary, payment rows and history continue to use `format_datetime`, so a 2:35 PM payment reads "Nov 15, 2026 at 2:35 PM" with no UI change.

### 5.6 Edge cases

- Optional due date cleared → `bills.due_date = NULL`; subtitle simply omits "· due …". Existing overdue logic (`money::is_overdue`) already tolerates `None`.
- Due date may be in the past or future — no restriction, consistent with the current text field.
- Selecting a leading/trailing cell from the adjacent month both selects that date **and** pages the header to that month.
- Year bounds: chevrons disable at 2000-01 and 2100-12; month/year zoom stops at the same bounds.
- DST: `local_datetime_ms` falls back to `earliest()`, then noon, so no payment can land outside a day.
- Rotating the device / resizing the window reflows the grid but never clears the pending selection (state lives in Rust).
- Payment sheet reopening resets date/time to now, matching today's `root.payment-date = ""` reset.

---

## 6. Feature 2 — Bill management after creation

### 6.1 New Bill form (`ui/screens/new-bill.slint`)

Stays as the composition step; no edit or exclude affordance exists on it.

- Unchanged: bill type, total amount, billing month (now a picker), due date (now a picker), note, boarder selection, per-boarder custom amount fields.
- Copy change under **Split among**: "Leave amounts empty to split equally. A typed amount is fixed; the rest are split equally. You can edit or exclude boarders after the bill is created."
- `create_bill` in `apps/mobile/src/app/bills.rs` keeps its current split logic (`split_with_overrides`) and its validation messages.
- Rationale for keeping typed amounts: they configure the initial split (a rent bill where one room is larger); editing and exclusion are the post-creation operations the request is about.

### 6.2 Bill screen entry point (`ui/screens/bill.slint`)

- The **Shares** `SectionHeader` gains `action-text: "Manage"`, `action-icon: Icons.sliders`, wired to `callback manage-shares()`.
- Tapping a share row still opens the Edit share sheet (unchanged shortcut).
- The **Manage shares** sheet (`SheetKind.manage-shares`, direct child of `AppWindow`) lists every share with its current amount and three actions per row plus one global action:

```
Sheet  title: "Manage shares"
       subtitle: "<bill title> · Total ₱900.00"
[ Banner, warning ]  "This bill is archived. Changes apply immediately and are kept if you restore it."   (archived only)
[ Banner, error ]    sheet error (lock/validation)
ListCard
  Maria Santos        ₱450.00   [ Edit ]  [ Exclude ]
  Juan Dela Cruz      ₱450.00   [ Edit ]  [ Exclude ]
[ AppButton  "Add boarder"  kind: secondary  icon: plus ]
Text: "Excluding a boarder keeps the bill total and splits it equally among the remaining boarders. It only affects this bill."
```

- **Edit** closes this sheet and opens the existing `SheetKind.edit-share` for that share (amount + additional charge + live split preview; locked rules unchanged).
- **Exclude** shows an inline confirmation inside the same sheet (subtitle becomes "Exclude <name>?") with `Exclude from this bill` (danger) and `Keep on bill` (secondary), plus a one-line consequence: "Their ₱450.00 share is split equally among the remaining 2 boarders. The bill total stays ₱900.00."
- **Add boarder** opens a picker of active boarders not already on the bill (reuse `SheetOption` rows). Selecting one re-splits immediately; no further confirmation because it is easily reversible by exclusion.
- Whole-row styling, radii, and type reuse `ListCard`/`ListRow`/`AppButton` so the sheet looks like the rest of the app in light and dark mode.

### 6.3 Exclusion math

Exclusion never changes `bills.total_minor`. Base shares are re-split equally across the remaining boarders, using the same centavos rule as `equal_split`:

1. `remaining` = the bill's shares minus the excluded one, in existing bill order.
2. `base_total` = `bills.total_minor`.
3. `step` = 1 when `allow_centavos`, else 100 (`MINOR_UNITS_PER_UNIT`).
4. `base = (base_total / (n * step)) * step`; `remainder_units = (base_total − base * n) / step`.
5. The first `remainder_units` boarders in bill order get one extra `step`, exactly like `equal_split`.
6. Each remaining share keeps its `additional_charge_minor`; the **excluded share's charge disappears with the share**.
7. The excluded share row is deleted (payments are impossible — see §6.5 — so no orphan payment rows exist).

Worked example, centavos on, bill total ₱900:

| Boarder | Before base | Before charge | After exclusion of Ana | After charge | Owed |
| --- | --- | --- | --- | --- | --- |
| Maria | 300.00 | 65.00 | **450.00** | 65.00 | **515.00** |
| Juan | 300.00 | 0 | **450.00** | 0 | **450.00** |
| Ana | 300.00 | 0 | *(excluded)* | – | – |
| Bill total | **900.00** | | **900.00** | | Expected **965.00** |

Whole-peso example (centavos off), total ₱1,000, four boarders, exclude one: remaining base ₱1,000 ÷ 3 = ₱333 each, ₱1 leftover → earliest remaining boarder gets ₱334. The sum is always exactly ₱1,000.

### 6.4 Add-boarder math

Same routine over the enlarged set: the new boarder joins at the end of bill order and the bill total is re-split equally across everyone; existing charges are preserved. Example: ₱900 with Maria ₱450 / Juan ₱450 → add Ana → ₱300 / ₱300 / ₱300 (Maria's ₱65 charge would keep her owed at ₱365).

### 6.5 Rules, locking, and messages

| Situation | Behavior |
| --- | --- |
| Bill has any active payment (`is_locked`) | Exclude **and** Add are refused: "Share amounts are locked because payments have been recorded. Void the payments first to change who is on this bill." Edit share keeps today's rule (charges editable, amounts frozen) |
| Only one share would remain | Refuse: "A bill needs at least one boarder. Archive the bill instead." |
| Excluded share has an active payment | Refuse: "Void this boarder's payment before excluding them." (defensive; the lock rule above fires first) |
| Boarder already on the bill (Add) | Refuse: "That boarder is already on this bill." |
| Archived boarder (Add) | Refuse: "Archived boarders cannot be added to a bill." |
| Archived bill | Allowed, with the existing warning banner |
| Voided payments only | Counts as unlocked; exclusion and add are allowed |

- Exclusion is **per-bill**: `boarders.status` is untouched, so the boarder appears on every other bill and can be selected on future bills.
- Consequence to state in the UI copy and the release note: the excluded boarder's **ledger no longer lists this bill**, because the share row is deleted. This is intended: the boarder is exempt for that month.

### 6.6 Persistence (`crates/treasurer-core/src/db/repository/bills.rs`)

```rust
/// Removes one boarder from a bill and re-splits the bill total equally
/// among the remaining shares. Bill total is unchanged.
pub fn exclude_share(&self, bill_id: &str, boarder_id: &str) -> Result<()>;

/// Adds an active boarder with no share on this bill and re-splits the bill
/// total equally across everyone. Existing additional charges are preserved.
pub fn add_share_to_bill(&self, bill_id: &str, boarder_id: &str) -> Result<()>;
```

Both:

1. Load `bill_detail`; bail "Bill not found." when absent.
2. Read `allow_centavos` from `app_settings`.
3. Enforce §6.5 (lock, minimum count, duplicates, archived boarder).
4. Compute the new base split with the `equal_split` algorithm (step-aware, leftover to earliest).
5. In **one transaction**: delete the share row / insert the new share (`additional_charge_minor = 0`, `boarder_name_snapshot` from the boarder), `UPDATE bill_shares SET amount_due_minor` for every changed row, and `UPDATE bills SET updated_at`.
6. Never touch `payments`; a payment row can only exist for a share that the lock rule already prevents removing.

The app layer runs the same checks first so the sheet can show inline errors; the repository stays the source of truth. The existing `save_share_edit` is unchanged and is what the Edit funnel calls.

---

## 7. Feature 3 — Overpayment dialog

### 7.1 Trigger and sequencing

`pay_share` in `apps/mobile/src/app/bills.rs` changes order deliberately so failure cannot look like success:

1. Parse amount (unchanged) and date/time from the picker.
2. Compute a **preview** with `preview_payment(share_due, existing_payments, amount, paid_at)`.
3. Call `db.record_payment(...)` — the payment is committed here.
4. Refresh bill detail, dashboard, and history.
5. If the preview (or the freshly recomputed payment) has `change_from_this_payment_minor > 0`, set the dialog properties and show the overpayment dialog; otherwise `notify("Payment of ₱… recorded.")` as today.
6. When the dialog shows, the pay sheet closes first (`SheetKind.pay → none`), then the dialog opens as its own overlay.
7. The success banner is suppressed while the dialog is visible (the dialog is the confirmation); after dismissal no banner is required, because the share row already shows the new balance/change.

The dialog appears for **every** payment that creates change, including a second overpayment on the same share. It also appears when the share was already fully paid (a payment on a ₱0 balance is 100% change).

### 7.2 Visual specification (`ui/components.slint` → `OverpaymentDialog`)

```
        scrim: Theme.scrim (full window, swallows taps)
        ┌──────────────────────────────────────┐   max-width 360px, radius-xl,
        │ (icon)                           (X) │   surface, border, shadow,
        │ Overpayment Received!                │   padding space-5
        │ Maria Santos · Nov 2026 Water        │   title-lg bold / sub muted
        │ The tenant paid ₱655.00, which is    │
        │ ₱155.00 more than the required bill  │   text-body, wrapped
        │ of ₱500.00.                          │
        │ [ Return change now ]                │   AppButton primary, full width
        │ [ Later ]                            │   AppButton secondary, full width
        └──────────────────────────────────────┘
```

- Icon: 44 px circle, `Theme.warning-soft` background, `Icons.alert` in `Theme.warning`; title and body in `Theme.text` / `Theme.text-muted`.
- Copy is fixed template, formatted with `money::format_money`:
  - Title: `Overpayment Received!`
  - Body: `The tenant paid {received}, which is {change} more than the required bill of {due}.`
  - `{due}` is the **tenant's share due** (base share + additional charge), not the whole bill total. The subtitle names the boarder and bill so a ₱500 share of a ₱1,500 bill is unambiguous.
- "Return change now" → closes the dialog and opens the change-returned confirmation sheet (§8.2) for the just-recorded payment.
- "Later" and X → dismiss only.
- Entrance: fade scrim + 8 px rise of the card, `Theme.medium` / `ease-out`; no bounce, no auto-dismiss timer.
- The dialog is a direct child of `AppWindow` after the sheets so it covers everything, including the nav.

### 7.3 Dismissal rules

| Input | Result |
| --- | --- |
| X button | Dismiss |
| Tap anywhere on the scrim | Dismiss |
| Android Back / desktop Escape | Dismiss (explicit input, not automatic) |
| Any caret on the card body | Does nothing (card swallows taps) |
| Timer | None — it stays until the treasurer dismisses it |
| Navigating/recording another payment | Only possible after dismissal; the overlay blocks the UI |

`go-back()` gains the dialog as its highest-priority layer, before the sheet check.

### 7.4 State

```slint
in property <bool> overpay-shown: false;
in property <string> overpay-amount;      // "₱655.00"
in property <string> overpay-change;      // "₱155.00"
in property <string> overpay-due;         // "₱500.00"
in property <string> overpay-subtitle;    // "Maria Santos · Nov 2026 Water"
callback overpay-return-change();
callback overpay-dismiss();
```

Rust stores the payment ID behind the dialog so "Return change now" can open §8.2 for exactly that payment (`App.overpay_payment_id`). The dialog must not reopen on a later screen refresh — it is driven only by the payment action.

---

## 8. Feature 4 — Change-return tracking

### 8.1 Status model (derived, no schema change)

Per active payment, with `computed_change_minor` from `recompute_payment_changes`:

| Condition | Label | Tone |
| --- | --- | --- |
| `computed_change_minor == 0` | *(no change row)* | — |
| `> 0` and `change_returned_at.is_none()` | **Change to Return** | warning |
| `> 0` and `change_returned_at.is_some()` | **Change Returned** | success |
| payment voided | Voided (existing) | neutral |

Share level, from `ShareState`:

- `change_outstanding_minor > 0` → line `Change to Return {amount}` (warning).
- else `change_returned_minor > 0` → line `Change Returned {amount}` (muted/success).
- `has_returned_change` already exists for the mixed case (some returned, some outstanding) — label it `Change Returned {returned} · {outstanding} to return`.

Existing invariants stay: `paid_minor` counts the full received amount, `applied_minor = min(paid, due)`, `balance_minor = max(0, due − paid)`, so a bill is **fully paid** while change is outstanding, and the bill status (Paid/Settled) does not wait for change.

### 8.2 Confirm "Change returned"

New `SheetKind.return-change`, opened from:

- the payment row's **Change returned** action (replaces today's one-tap "Mark change returned"),
- the share row's change line (same sheet, newest outstanding payment for that share),
- the overpayment dialog's "Return change now".

Content:

```
Sheet  title: "Return change?"
       subtitle: "<boarder> · <amount> to return"
[ field ] Amount to return        ₱155.00              (read-only row)
[ field ] Returned on             Nov 15, 2026   ›     (calendar picker, default today)
[ field ] Note (optional)         "handed to Maria"
[ AppButton  "Confirm returned"  icon: check ]
[ AppButton  "Cancel"  secondary ]
```

- Confirm → `db.mark_change_returned(payment_id, local_datetime_ms(date, 12, 0), note)` (date-only fact at noon, matching the payment-date convention), then refresh bill detail / dashboard / ledger and `notify("Change of ₱155.00 marked as returned.")`.
- The returned amount is not typed — it is the computed change, so the record cannot drift from the math.
- Already-returned payments show `Change Returned` with no action.
- Corrections: the payment detail sheet lets the treasurer re-pick the returned date, calling a new `update_change_returned_at(payment_id, at)` that only touches `change_returned_at` (bails when the payment is voided or has no change).
- No partial change returns: change is returned per payment, whole amount. (Request keeps the three figures separate; partial change would need a fourth table and is out of scope.)

### 8.3 Payment detail sheet

New `SheetKind.payment-detail`, opened by tapping any payment row on the bill screen **and** the boarder ledger. Read-only, no save button:

```
Sheet  title: "Payment"
       subtitle: "<boarder> · <bill type> · <period>"
[ Banner ]  "Voided: <reason>"                        (voided only)
[ rows ]  Total Bill            ₱500.00               (tenant's share due)
          Amount Received       ₱655.00
          Change Returned       ₱155.00               ("—" when none; "Change to Return ₱…" chip when outstanding)
          Net Payment           ₱500.00               (received − change)
          Payment Status        Fully Paid            (share status: Fully Paid / Partial / Unpaid)
          Change Status         Change Returned       (Leave blank / Change to Return / Change Returned)
          Recorded              Nov 15, 2026 at 2:35 PM
          Returned on           Nov 15, 2026   ›      (only when returned; tappable to correct)
[ AppButton  "Close"  secondary ]
```

- Layout: label left (`Theme.text-sub`, muted), value right (`Theme.text-body`, semibold), 44 px rows on `Theme.divider`, inside a `Card` — the same rhythm as `SummaryMetrics`.
- Status values use `StatusPill` with `Theme.status-tone` semantics; "Change to Return" is warning, "Change Returned" is success, "Fully Paid" is success.
- `Payment Status` reflects the **share's** current status, because "fully paid" is a share-level fact shared by several payments. `Change Status` is per payment.
- Values come from a new `PaymentDetail` window model built in Rust from the selected `PaymentWithComputedChange` plus its share state and bill context; no new core query is needed beyond `share_payment_context` / `bill_detail`.

### 8.4 Required final transaction table

For the request's example (bill ₱500, received ₱655, change ₱155), after confirming the return the payment detail reads exactly:

| Description | Amount |
| --- | ---: |
| Total Bill | ₱500.00 |
| Amount Received | ₱655.00 |
| Change Returned | ₱155.00 |
| Net Payment | ₱500.00 |
| Payment Status | Fully Paid |
| Change Status | Change Returned |

Before the confirm, the same sheet shows `Change to Return ₱155.00` and `Net Payment ₱500.00`, with the return action offered. Records remain separate: `amount_minor = 65500`, `change_minor = 15500`, `change_returned_at = <date>`, and everything derived (`applied_minor`, `balance_minor`) is computed, never overwritten.

### 8.5 Aggregates (already wired, keep green)

- Dashboard "Change due" metric and the **Change** filters keep reading `change_outstanding_minor`.
- Bill summary `change` metric keeps summing outstanding change.
- Boarder balances and ledger keep showing change outstanding.
- History totals keep including change outstanding; no new column or label changes there.

---

## 9. Persistence summary

| Change | Needed? |
| --- | --- |
| Schema migration | **No.** `change_minor`, `change_returned_at`, `change_return_note` already exist; exclusion deletes a share row using existing columns |
| New repository methods | `exclude_share`, `add_share_to_bill`, `update_change_returned_at` |
| Changed repository methods | none (existing signatures preserved) |
| Backup format | unchanged, `version: 1`; excluded shares are simply absent, added shares export normally, change fields already round-trip |
| New core helpers | `calendar_weeks`, `local_datetime_ms`, `split_local_datetime`, `add_months`, `iso_date_to_epoch_noon` in `money.rs` |
| New Slint models | `CalendarCell` (components file) and a `PaymentDetail` struct in `models.slint` |

---

## 10. File-by-file change plan

| File | Work |
| --- | --- |
| `crates/treasurer-core/src/money.rs` | Calendar/date-time helpers and year bounds |
| `crates/treasurer-core/src/db/repository/bills.rs` | `exclude_share`, `add_share_to_bill` |
| `crates/treasurer-core/src/db/repository/payments.rs` | `update_change_returned_at` |
| `crates/treasurer-core/tests/calculations.rs` or new `tests/bill_management.rs` | Re-split-after-exclusion/add math, lock refusals, last-share refusal |
| `crates/treasurer-core/tests/storage.rs` | Repository round trip for exclude/add/return-date correction |
| `crates/treasurer-core/tests/money.rs` | Date helper round trips, DST fallback, calendar grid shape |
| `apps/mobile/ui/components.slint` | `CalendarCell`, `CalendarSheet`, `OverpaymentDialog`, shared read-only date field styling |
| `apps/mobile/ui/icons.slint` | No additions expected (`calendar`, `clock`, `sliders`, `check`, `alert` already exist) |
| `apps/mobile/ui/models.slint` | `PaymentDetail` struct (if it is exposed as a model rather than window properties) |
| `apps/mobile/ui/app.slint` | `SheetKind.calendar`, `.manage-shares`, `.return-change`, `.payment-detail`; dialog layer; new properties/callbacks; replace typed date fields; payment-date row; close-sheet ordering for the dialog |
| `apps/mobile/ui/screens/new-bill.slint` | Read-only due-date row + billing-month row, updated helper copy |
| `apps/mobile/ui/screens/bill.slint` | Manage action, change-status line/pill, tap-to-open payment detail, `return-change` triggers sheet instead of instant action |
| `apps/mobile/ui/screens/ledger.slint` | Payment rows open the payment detail sheet |
| `apps/mobile/ui/screens/settings.slint` | Last-backup row unchanged (display only); no picker |
| `apps/mobile/src/app/bills.rs` | Date/time plumbing, overpayment dialog state, manage-shares handlers, payment detail builder, return-change sheet logic |
| `apps/mobile/src/app/dashboard.rs` | Month picker handler |
| `apps/mobile/src/app/mod.rs` | New `App` fields (`calendar_*`, `overpay_payment_id`, `detail_payment_id`), dialog-aware `go-back` ordering |
| `README.md` | Feature bullets: calendar pickers, post-creation share management, overpayment notice, change return status |
| `docs/specs/system-improvements-spec.md` | This spec |

---

## 11. Edge cases and invariants

1. **Base shares always equal `bills.total_minor`.** Exclusion, addition, and edits preserve this; charges are additive and unbounded.
2. **No payment is ever orphaned.** Shares are deleted only when the bill has no active payments.
3. **Changing `allow_centavos` mid-life** keeps stored amounts untouched (existing limitation); re-splits reject fractional amounts under whole-peso mode with the existing "Whole pesos only…" message.
4. **Excluding the last boarder** is refused rather than leaving an empty bill.
5. **Archived bill + exclusion**: allowed with the warning; restoring shows the new split.
6. **Payment detail for a voided payment** shows the void reason and zero change status; no return action.
7. **Overpayment dialog while bill refresh fails**: the dialog originates from the committed payment, so it still shows; the refresh error banner appears after dismissal.
8. **Double tap on Record payment / Confirm returned**: the sheet closes on success; the repository re-checks state and rejects duplicates ("already marked returned").
9. **Scrim taps** dismiss the dialog/sheets but must not activate the row underneath — the overlay owns the tap.
10. **`paid_at` ordering** is unchanged: `recompute_payment_changes` sorts by `paid_at`, then `created_at`, then id, so choosing an earlier date/time on a later entry can move change attribution. This behavior is intentional and preserved; the dialog copy always names the payment's own computed change.
11. **Last backup date is display-only.** It records a file operation, not a treasurer-entered date; making it editable would falsify the backup record. This is the single exception to "every date anywhere" (see open questions).
12. **Timezone**: all dates are local-zone; stored `due_date` remains a plain `YYYY-MM-DD` string, payments remain epoch ms.

---

## 12. Testing plan

- **Core unit tests** (`cargo test -p treasurer-core`):
  - `exclude_share`: total unchanged, remaining equal split (with centavos on and off, incl. leftover step), charge preserved, excluded charge dropped, refusals for lock/last share/unknown boarder.
  - `add_share_to_bill`: re-split across old + new, duplicate/archived refusals, lock refusal.
  - `mark_change_returned` with a chosen date and note; `update_change_returned_at` correction and its refusals.
  - Money helpers: `calendar_weeks` returns 42 cells starting Sunday and covers month edges; `local_datetime_ms`/`split_local_datetime` round trip; `add_months` clamps to 2000–2100.
  - Existing change tests (`overpayment_creates_change`, `bills_settle_when_every_share_is_covered_even_with_change`, etc.) must stay green unmodified.
- **Desktop UI pass** (`cargo run -p treasurer-mobile --bin treasurer`): open each of the six picker entry points, choose a day in another month, clear a due date, pick a time, exclude and add a boarder, overpay and dismiss via X / outside tap / Escape, then return the change and reopen the payment detail. Verify light + dark and a narrow (320 px) window.
- **Android sanity**: build/install per `docs/installation.md`; confirm sheet + dialog cover the pill nav, Back dismisses the dialog, and the keyboard never covers the payment fields.
- **Backup round trip**: export after an exclusion and an added boarder, restore, confirm totals and change fields survive.

---

## 13. Non-goals

- No reservations module, no tenant move-in/lease dates, no cloud sync.
- No partial change returns, no change ledger table, no payment splitting across shares.
- No undo for an exclusion beyond adding the boarder back.
- No changes to the money model, split rounding rules, backup format, or lower-level SQL beyond the listed methods.
- No new third-party dependency; the calendar is drawn in Slint.

---

## 14. Open questions

1. Last backup date: keep display-only (recommended, §11.11) or allow re-picking it for full literal parity with "every date anywhere"?
2. Should excluding a boarder also appear in that boarder's ledger as an "exempt this month" line (would require keeping the row with `amount_due_minor = 0` instead of deleting it)?
3. Minute granularity on the time wheel: every minute (0–59, recommended for accuracy) or 5-minute steps for fewer taps?
4. Should the Manage shares sheet show a "Reset to equal split" action for boards with several manual edits?
5. Should the overpayment dialog also offer a "View payment" shortcut in addition to "Return change now" and "Later"?
