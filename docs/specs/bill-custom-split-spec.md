# Bill Name Filter, Custom Split, and Additional Charges — Feature Spec

**Status:** Draft, ready for implementation
**Request short name:** bill-custom-split
**Platform:** `apps/mobile` (Slint + Rust, desktop and Android; `treasurer-core` for domain/storage)
**Scope:** Bill detail screen only, plus the New Bill form's split behavior, persistence, backups, and the derived totals that read share amounts.

---

## 1. Summary

Inside an existing bill:

1. **Find a boarder by name.** A name filter plus an **All / Unpaid / Paid** status chip sits above the Shares list and narrows the visible shares live as the landlord types.
2. **Edit a share by tapping the boarder's name.** A sheet opens with the boarder's **Share amount** and an **Additional charge** field. Editing the share pins that boarder's amount and redistributes the remainder **equally among all other boarders** (the example: 300 split three ways, type 50 for one, the other two become 125 and 125).
3. **Add an additional charge to one boarder.** The charge is optional, manual, and separate from the bill total (300 stays 300). It is added to that boarder's balance: 100 share + 65 charge = 165 owed. The charge is not split among anyone else.

The same redistribution rule also applies on the **New Bill** form while composing a bill, so the create flow and the edit flow agree.

Additional charges flow into balances, outstanding, progress, dashboard totals, history totals, and boarder ledgers. Only the bill's stored **Total** stays at the original bill amount.

---

## 2. Request, restated with worked examples

### 2.1 Name filter

> "when in inside the bill there should be a name filter, so that I can find the name easily"

- Bill → Shares list → type part of a name → only matching boarders remain.
- The filter also has status chips, so "unpaid boarders whose name contains `ma`" is one interaction.
- The Payments section and the Archive button are not filtered.

### 2.2 Custom split (redistribution)

> "the bill has 300, and there is 3 tenant the normal is equal distribution 100, 100, 100 but if I manually input the 1 tenant like 50, the remaining 50 will go to the 2 tenant and then there bill will become 125,125."

Worked example, centavos on:

| Boarder | Before | Landlord types | After |
| --- | --- | --- | --- |
| Maria | 100.00 | **50.00** | 50.00 (pinned) |
| Juan | 100.00 | – | 125.00 |
| Ana | 100.00 | – | 125.00 |
| **Bill total** | 300.00 | | **300.00** |

### 2.3 Additional charge

> "there is 300, and 3 tenants 100,100,100, i can add additional charges to the 1 tenant, click the tenant name 100, and then ... add additional charges ... the landlord can write any amount like 65, in result the bill for that tenant is 164 [165] ... the 300 does not count in this because it is a additional charges"

Worked example:

| Boarder | Base share | Additional charge | Owed |
| --- | --- | --- | --- |
| Maria | 100.00 | **65.00** | **165.00** |
| Juan | 100.00 | 0 | 100.00 |
| Ana | 100.00 | 0 | 100.00 |
| Sum of base shares | 300.00 | | |
| **Bill Total** (subtitle) | **300.00** | | |
| **Bill Expected** (metric) | | | **365.00** |

Purpose: late-payment fee or any manual surcharge. No fee schedule, no automatic calculation, no reason required.

---

## 3. Current state (verified)

| Area | Current behavior |
| --- | --- |
| Bill screen | `apps/mobile/ui/screens/bill.slint`. Summary card, Shares list (`ListRow`, `tappable: false`, Pay button per row), Payments list, Archive button. |
| Bill app logic | `apps/mobile/src/app/bills.rs`: `refresh_bill_detail`, `open_bill`, `pay_share`, `void_payment`, `return_change`, `archive_bill`. `share_ids` holds share IDs parallel to the `shares` model, so **filtering must keep that index mapping consistent**. |
| Screens/sheets shell | `apps/mobile/ui/app.slint`: screen ids, window properties, `SheetKind` enum, bottom sheets (`pay`, `void-payment`, `currency`, `bill-type`, `boarder-actions`, `rename`, `restore`). Sheets cover the nav and are dismissed by scrim tap / hardware Back. |
| Shared widgets | `apps/mobile/ui/components.slint`: `TextField`, `Segmented` (equal-width chips), `Sheet`, `Banner`, `EmptyState`, `AppButton`, etc. `TextField` has **no `enabled`/read-only prop** today. |
| Icons | `apps/mobile/ui/icons.slint`: stroke paths only; has `chevron-right`, `pencil`, `cash`, `alert`. **No search icon** and no edit glyph. |
| Bill rows model | `apps/mobile/ui/models.slint`: `ShareRow { name, due, paid, balance, balance-input, change, status, has-balance, has-change }`. |
| Storage | `crates/treasurer-core/src/db/mod.rs`: `bill_shares (id, bill_id, boarder_id, boarder_name_snapshot, amount_due_minor, created_at)`. `initialize` runs `ensure_schema` (which **adopts** an existing `bills` table and skips DDL) then `seed`. **There is no migration mechanism for columns.** |
| Bill creation | `create_bill` validates `sum(shares) == total_minor`, inserts shares. New Bill form (`new-bill.slint` + `create_bill` in `bills.rs`) requires custom amounts to add up exactly; blanks become 0. |
| Locking | `BillWithDetails.is_locked` = any active payment. `update_bill` refuses amount changes then. No edit UI exists. |
| Summaries | `compute_share_state(amount_due_minor, payments)` drives paid/balance/change/status. `compute_bill_summary` sums `share.share.amount_due_minor` into `expected_minor`. `compute_bill_summary` drives dashboard, history, and boarder summaries through repository code. |
| Backup | `crates/treasurer-core/src/backup.rs`, `version == 1` (strict). Shares export/import `amountDueMinor`. Validation requires base shares to sum to `bill.totalMinor`. |
| Tests | `crates/treasurer-core/tests/{storage,calculations,money}.rs`; construct `BillShare` literals directly, so the new field touches test helpers. |
| Not the target | `apps/native` (Expo) contains no app code; `apps/desktop` and `apps/server` do not host the bill screen. |

---

## 4. Interview decisions (source of truth)

| # | Decision | Answer |
| --- | --- | --- |
| 1 | Filter scope | **Shares list only**; Payments list stays complete |
| 2 | Filter controls | **Name text + status chip** combined as one filter row above Shares |
| 3 | Chips | **All / Unpaid / Paid** (`Unpaid` = any balance; `Paid` = zero balance) |
| 4 | Filter reset | **Reset on leaving the bill**; empty name and `All` each time a bill opens |
| 5 | Edit entry point | **Tap the boarder's name row** on the Shares list |
| 6 | Edit UI shape | **One sheet, two fields**: Share amount + Additional charge, then Save |
| 7 | Redistribution | Edited amount is **pinned**; **all other boarders reset to equal split** of the remainder, even if one was previously custom |
| 8 | Same rule on New Bill | **Yes**, typing a custom amount before creating redistributes the rest |
| 9 | Charge effect | **Balance includes it**; bill Total stays 300, bill Expected becomes 365 |
| 10 | Aggregates | **Include everywhere**: dashboard, bill expected/outstanding, progress, history, boarder balances/ledgers |
| 11 | Charge structure | **Single amount**, no label, no reason; `0` removes it |
| 12 | Bill summary display | Subtitle keeps **Total 300**; the **Expected** metric shows 365; no new Charges metric |
| 13 | Share row display | **Separate charge line**: keep "100 due", add "+ 65 additional charge"; balance reads 165 |
| 14 | Locked bills (payments exist) | **Share amount frozen, charges still editable** |
| 15 | Archived bills | **Everything editable, with a warning** in the sheet |
| 16 | Settled bill + charge | **Bill reopens** (status returns to Open, outstanding rises) |
| 17 | Validation | **Inline error, block Save** (over total, cannot split, whole pesos only, invalid charge) |
| 18 | Edit sheet feedback | **Live preview list** of every boarder's recomputed amount before Save |
| 19 | Centavos rounding | Reuse the existing `allow_centavos` rule; leftover minor units go to the earliest non-pinned shares |

---

## 5. Data model

### 5.1 Schema change

`bill_shares` gains one nullable-free column:

```sql
additional_charge_minor INTEGER NOT NULL DEFAULT 0
```

- Fresh databases: add the column to `SCHEMA_SQL` in `crates/treasurer-core/src/db/mod.rs`.
- Existing databases: `ensure_schema` returns early when `bills` exists, so add an idempotent column check that runs on **every** `initialize`, after `ensure_schema` and before `seed`:

```rust
fn ensure_columns(conn: &Connection) -> Result<()> {
    // PRAGMA table_info(bill_shares); ALTER TABLE ... ADD COLUMN if missing.
}
```

- The migration must be safe on a database that has no `bill_shares` table yet (skip if absent), and safe to run repeatedly.
- No index needed.
- `additional_charge_minor >= 0` is enforced in app/core validation, not by SQLite `CHECK` (SQLite cannot add a CHECK constraint via `ALTER TABLE`).

### 5.2 Rust model

`crates/treasurer-core/src/models.rs`:

```rust
pub struct BillShare {
    // existing fields ...
    pub amount_due_minor: i64,        // base split share only, still sums to bills.total_minor
    pub additional_charge_minor: i64, // 0 when none
    pub created_at: i64,
}

impl BillShare {
    /// What the boarder actually owes: base share + manual charge.
    pub fn total_due_minor(&self) -> i64 {
        self.amount_due_minor + self.additional_charge_minor
    }
}
```

**Invariant:** `Σ amount_due_minor == bills.total_minor` (unchanged); `Σ total_due_minor == bill Expected` and may exceed the total.

### 5.3 Row mapping and queries

- `map_share` (`db/repository/mod.rs`) reads `additional_charge_minor` as column index 6.
- Every `SELECT ... FROM bill_shares` used by `map_share` must include the column in the same order: `all_shares` (`loading.rs`), backup export (`backup.rs`). Search for `amount_due_minor, created_at` to catch all sites.
- `create_bill` inserts `0` explicitly.
- `update_bill` currently deletes and re-inserts shares. It must:
  - Compare the sum of **base** amounts to `total_minor` (unchanged rule).
  - Carry each existing boarder's `additional_charge_minor` into the re-insert, and preserve share `id`/`created_at` (or at minimum preserve charges; `update_bill` is not reachable from the UI today but is covered by a test).
- `import_backup` inserts `additional_charge_minor`.

---

## 6. Share math

### 6.1 Effective due

Every balance/status/change computation uses `BillShare::total_due_minor()`:

- `compute_share_state(total_due_minor, payments)` → `paid_minor`, `applied_minor = min(paid, due)`, `balance_minor = max(0, due - paid)`, `status`, `change_outstanding_minor`.
- Payment preview and change allocation (`recompute_payment_changes`) treat 165 as the amount due, so a 200 payment yields 35 change and a 100 payment on a 165 due leaves 65.
- `compute_bill_summary`:
  - `expected_minor = Σ total_due_minor` → 365.
  - `applied_minor`, `outstanding_minor`, `progress`, share counts unchanged in formula.
  - `status`: a charge on a settled bill makes `settled == false`, so status becomes **Open** (unless archived, which wins).
- `Bill.total_minor` remains the 300 shown in the subtitle and stored in the `bills` table.

Call sites to change (audit all of them):

| File | What changes |
| --- | --- |
| `db/repository/loading.rs` | `shares_with_states` passes `share.total_due_minor()` |
| `db/repository/payments.rs` | preview, record, void, and change computations use total due |
| `db/repository/reports.rs` | `BoarderPeriodSummaryInput.amount_due_minor` and `LedgerSummary.expected_minor` use total due |
| `db/repository/boarders.rs` | boarder outstanding/change sums use total due |
| `calculations.rs` | `compute_bill_summary` expected sum uses total due; `preview_payment` inputs follow the caller |
| `apps/mobile/src/app/*.rs` | any place formatting `amount_due_minor` as the owed amount (bill rows, ledger rows) uses total due; the New Bill form still shows base amounts |

Because dashboard/history/boarder summaries read through these functions, charges propagate automatically once the call sites are updated. Existing test expectations that assume expected == total keep passing when all charges are 0.

### 6.2 Pinned resplit

New pure function in `crates/treasurer-core/src/calculations.rs`:

```rust
pub fn resplit_with_pinned(
    total_minor: i64,
    boarders: &[SplitBoarder],   // every share on the bill, in existing bill order
    pinned_id: &str,
    pinned_amount_minor: i64,
    allow_centavos: bool,
) -> SplitOutcome
```

Rules, in order:

1. `boarders` is non-empty and contains `pinned_id`, otherwise error "That share is no longer available."
2. `pinned_amount_minor >= 0`, otherwise "Amounts cannot be negative."
3. `pinned_amount_minor % step == 0`, otherwise "Whole pesos only. Turn on centavos to use decimals." (`step = 100` when centavos are off, matching `split_step`.)
4. If there is **one** share: pinned must equal `total_minor`, otherwise error "The only share must equal the bill total."
5. If there are **two or more** shares: `pinned_amount_minor < total_minor`, otherwise "Leave room for the other boarders."
6. `remainder = total_minor - pinned_amount_minor`; split it among the other boarders using the same algorithm as `equal_split`:
   - `base = (remainder / (pool_len * step)) * step`;
   - `remainder_units = (remainder - base * pool_len) / step`;
   - the first `remainder_units` boarders **in existing bill order (excluding the pinned boarder)** get one extra `step`.
7. If `remainder < pool_len * step`, error "The remaining amount can't be split among the other boarders."
8. Output shares in the same order as `boarders`; the pinned row keeps `pinned_amount_minor`.
9. Charges are **not** passed in and are untouched by the split.

Implementation may reuse `split_with_overrides` internally (`overrides = {pinned_id: amount}`) if that keeps behavior identical; the extra rules 4–5 still need explicit checks so that clear messages win over the generic over/short errors.

### 6.3 Charge validation

```rust
pub fn validate_additional_charge(charge_minor: i64, allow_centavos: bool) -> Option<String>
```

- `< 0` → "Additional charges cannot be negative."
- `> MAX_AMOUNT_MINOR` → "That amount is too large."
- `% step != 0` when centavos are off → "Whole pesos only. Turn on centavos to use decimals."
- `0` is valid and means "no charge".

The charge is never limited by the bill total.

### 6.4 New Bill form

`apps/mobile/src/app/bills.rs::create_bill`:

- Replace the `any_custom` → "blanks become 0, require exact total" branch with `split_with_overrides(total_minor, selected_boarders, overrides, allow_centavos)`.
- `overrides` contains only selected boarders whose amount field is non-empty (parsed). Blank fields are auto-split.
- If every selected boarder has a typed amount, the pool is empty, so the existing requirement that typed amounts add up to the total still applies (message from `split_with_overrides`: "The assigned shares are short of the bill total." / "Custom amounts are over the bill total.").
- Update the helper text under "Split among" to: "Leave amounts empty to split equally. A typed amount is fixed; the rest are split equally."
- Previewing live recomputation on the create form is **not** required (no live list there); only the saved result changes.

---

## 7. Persistence API

New repository methods in `crates/treasurer-core/src/db/repository/bills.rs`:

```rust
/// Saves one share-edit operation: optional new base split for the whole bill
/// plus the edited boarder's additional charge. Atomic.
pub fn save_share_edit(
    &self,
    bill_id: &str,
    split: &[SplitResult],          // boarder_id -> base amount_minor, all shares
    edited_boarder_id: &str,
    charge_minor: i64,
) -> Result<()>;
```

Behavior:

1. Load the bill and its current shares; bail "Bill not found." if absent.
2. Validate `charge_minor` with `validate_additional_charge`; bail on error.
3. Validate `split` is non-empty, has unique boarder IDs, covers exactly the bill's existing share set, is non-negative, and `Σ` equals `bill.total_minor`. If not, bail "The assigned shares must equal the bill total."
4. Detect `changed_amounts = any share base differs from the stored value`.
   - If `changed_amounts && is_locked` (any active payment) → bail "Share amounts are locked because payments have been recorded. You can still change additional charges."
   - Archived bills are allowed; no archived check.
5. In one transaction:
   - For each changed row: `UPDATE bill_shares SET amount_due_minor = ? WHERE bill_id = ? AND boarder_id = ?` (never delete/re-insert, so payment FKs and IDs survive).
   - `UPDATE bill_shares SET additional_charge_minor = ? WHERE bill_id = ? AND boarder_id = ?` for the edited boarder.
   - `UPDATE bills SET updated_at = ? WHERE id = ?` when anything changed.
6. Commit. No-op updates are allowed.

The app layer performs the same validation **before** calling this so it can show inline errors; the repository repeats it as the source of truth.

---

## 8. UI specification

### 8.1 Shares section: filter row

Placement, in `BillScreen`, between the `SectionHeader` and the `ListCard`:

```
SectionHeader "Shares"
Caption: "Tap a name to edit the split"        (was: "Tap Pay to record money received")
┌────────────────────────────────────────────────────────┐
│ [ Find a boarder                          ]            │  TextField, no label
│ [ All ][ Unpaid ][ Paid ]                              │  Segmented, existing component
└────────────────────────────────────────────────────────┘
ListCard with filtered rows
```

- The filter row is rendered only when the bill has at least one share (`share-total-count > 0`).
- `TextField`: placeholder "Find a boarder", `input-type: text`, no label, full width (`horizontal-stretch: 1`), bound to window `share-filter`.
- `Segmented`: options `["All", "Unpaid", "Paid"]`, `selected: share-filter-mode`, label "Filter shares by status", full width, directly under the field with `Theme.space-2` spacing.
- Header caption while filtering (name non-empty or mode ≠ All): `"N of M boarders"`. Otherwise `"M boarders"`.
- Matching: trim the text, lowercase both sides, `contains`. Empty text matches everything. `Unpaid` = `balance > 0`; `Paid` = `balance == 0`.
- No debounce; filter on `edited` and chip change.
- Accessibility: field label "Filter boarders by name"; `Segmented` label names the group.
- When the filter yields zero rows, show an `EmptyState` inside a `Card`:
  - icon `Icons.people`, title "No boarders match", body "Try another name or status.", action-text "Clear filter", action clears the text and sets mode `All`.
- The filter does **not** hide or alter Payments, the summary card, or the Archive section.
- Reset on `open_bill`, on `set_screen(10)`, and whenever `detail_bill` is cleared (`clear_detail_lists`). It survives a payment refresh while the bill stays open.

### 8.2 Shares list row

Each row keeps its current layout plus:

- The entire left content area (name / status / balance / paid-of-due / charge line) is tappable and opens the edit sheet for that share. The Pay button keeps its own tap target and must **not** trigger the edit sheet.
- Implementation note: `ListRow.tappable` covers the full row and would swallow/duplicate Pay taps. Set `tappable: false` here and wrap the left `VerticalLayout` in a `TouchArea`-backed `Rectangle` that provides the pressed background on the left region only.
- Add a small affordance: a 16 px `Icons.chevron-right`, `Theme.text-faint`, at the end of the name line (after the status pill and spacer). It exists already in `icons.slint`.
- Text content, top to bottom:
  1. `share.name` + `StatusPill` + spacer + chevron
  2. `balance + " left to pay"` (danger) or `"Fully paid"` (success), **balance includes the charge**
  3. `paid + " paid of " + due` where **due is the base share** (`100.00`), so the split and the charge remain legible
  4. If `has-charge`: `"+ " + charge + " additional charge"` in `Theme.warning`, `Theme.text-sub`
  5. If `has-change`: `change + " change to return"` (unchanged)
- Accessibility label: `name, status, balance <balance> left to pay, includes <charge> additional charge, opens share editor`.
- Row `min-height` grows naturally when the charge line is present.

### 8.3 Edit share sheet

New `SheetKind.edit-share`, placed in `app.slint` like the other sheets.

Content:

```
Sheet
  title: "Edit share"
  subtitle: "<name> · Total <bill total formatted>"
  [Banner] if share-edit-error != ""
  [Banner] warning if the bill is archived
  TextField "Share amount"     (numeric keypad, prefilled base share input value)
  TextField "Additional charge" (numeric keypad, prefilled charge input value or empty)
    hint: "Optional, added on top of this share. It is not part of the bill total."
  Section "Split preview"
    Card/ListCard: one row per boarder — name, amount (share + charge),
                   pinned row tinted/labeled "edited"
    Footer text: "Shares <sum of bases> · Charges <sum of charges> · Expected <total expected>"
  AppButton "Save share" (primary, disabled while error or required field empty)
  AppButton "Cancel" (secondary)
```

Field rules:

- Share amount is prefilled with `money::minor_to_input_value(base_due)` (existing helper) and is **disabled** when `share-edit-locked` is true (payments exist). Disabled state shows the hint: "Share amounts are locked because payments have been recorded. Additional charges can still be changed."
- Additional charge is prefilled with `minor_to_input_value(charge)` or empty when 0; always editable, including on locked and archived bills.
- File-level `TextField` gains `enabled: true` (defaults for every current use). When false: 45 % opacity, no focus, no tap, `accessible-enabled: false`.
- Both fields use `InputType.number`.
- Editing either field calls `preview-share-split(amount_text, charge_text)` on every keystroke. Rust recomputes and pushes the preview model + validation message.
- Archived bills show a warning banner: "This bill is archived. Changes apply immediately and appear on the bill; they are kept if you restore it." No confirmation step.
- Save:
  1. Reject if `share-edit-error` is non-empty.
  2. Call `save-share` with the edited share resolved by **stored share ID at sheet open**, not by the current filtered index.
  3. On success: close the sheet, refresh bill detail (reapply filter), refresh dashboard/history/boarders, and notify:
     - "Share updated." when the base split changed;
     - "Additional charge updated." when only the charge changed.
  4. On repository error: keep the sheet open and show the error in its banner (existing pattern: Rust sets `message` with `message_error`, sheet shows `sheet-error`).
- Hardware Back / scrim tap dismisses without saving.

Live preview mechanics:

- Preview rows reflect the **in-progress** values: the typed share amount replaces the pinned row's base; all other bases are recomputed for display only; charge is added per row.
- If input is invalid (empty share amount, over total, cannot split, decimals when centavos are off, invalid charge), show the specific inline error and stop updating the numeric rows (keep the last valid preview or show dashes).
- Centavos rounding follows §6.2; the preview is exactly what Save will persist.
- The preview is display-only; nothing is persisted until Save.

### 8.4 Summary card

- Subtitle stays `Total <bill.total_minor formatted>` (300), followed by due date and note as today.
- `Expected` metric = 365 (sum of base + charges).
- `Collected` and `Change due` unchanged in definition; `Outstanding` rises by the charges.
- Progress uses expected 365, so adding a charge lowers progress until it is paid.
- No new metric, no "Charges" column.

### 8.5 Pay sheet

No structural change. Because the balance includes the charge:

- The Pay sheet subtitle reads `<name> · 165.00 left to pay`.
- "Use full balance" prefills 165.00.
- Overpayment change is computed against 165.

### 8.6 Payments list

Unchanged, except that `share.display_name()` and amounts already reflect the charged share. Payments do not display charge lines independently.

---

## 9. App layer design (`apps/mobile/src/app`)

### 9.1 State

Add to `App`:

```rust
/// Full, unfiltered shares of the open bill, rebuilt on every detail refresh.
detail_shares: Vec<ShareEntry>,      // id, boarder_id, name, base_due, charge, paid, status flags
share_filter: String,
share_filter_mode: i32,              // 0 All, 1 Unpaid, 2 Paid
/// Share ID of the row being edited; captured when the sheet opens.
edit_share_id: Option<String>,
edit_share_base: i64,
```

`ShareEntry` mirrors the formatted + raw values needed by the row, preview, and edit sheet; it avoids reparsing display strings.

### 9.2 Rendering pipeline

`refresh_bill_detail` fills `detail_shares` from `bill_detail` (ordered as returned by `all_shares`, i.e. existing bill order), then calls `render_shares()`:

1. Compute visible entries by applying the name + status filter.
2. Build `Vec<ShareRow>` with the new fields and set the `shares` model.
3. Set `display_share_ids: Vec<String>` to the **visible** share IDs, used by `pay_share`.
4. Set `share-total-count` and `share-visible-count` window properties.
5. When filtering yields nothing, set an empty model; Slint shows the empty state.

`pay_share(index)` resolves `display_share_ids[index]`.
`edit-share(index)` resolves the same, stores `edit_share_id`, prefills the window properties, builds the preview, and opens the sheet.
`save_share()` uses `edit_share_id` and `detail_shares`, calls `db.save_share_edit`, then refreshes.

`render_shares` is called after: detail refresh, filter text change, filter mode change, and a successful share save.

### 9.3 New window properties and callbacks (`app.slint`)

Properties:

```text
in-out <string> share-filter: ""
in-out <int>    share-filter-mode: 0
in      <int>   share-total-count: 0
in      <int>   share-visible-count: 0
in      <[ShareSplitPreviewRow]> share-preview: []
in      <string> detail-total: ""              // formatted bill.total_minor for the sheet subtitle
in-out <string> share-edit-amount: ""
in-out <string> share-edit-charge: ""
in      <string> share-edit-name: ""
in      <string> share-edit-error: ""
in      <bool>   share-edit-locked: false
in      <bool>   share-edit-archived: false
in      <bool>   share-edit-can-save: false
```

New model in `models.slint`:

```slint
export struct ShareSplitPreviewRow {
    name: string,
    amount: string,     // final share + charge
    pinned: bool,
}
```

Callbacks:

```text
callback set-share-filter(string);
callback set-share-filter-mode(int);
callback edit-share(int);                  // visible share index
callback preview-share-split(string, string); // amount text, charge text
callback save-share();
```

Wire them in `bills.rs::wire`, app.slint forwards to screens.

### 9.4 Filter reset

`open_bill` and `clear_detail_lists` set `share_filter = ""`, `share_filter_mode = 0`, clear `detail_shares`, and reset the window filter properties. `set_screen(10)` already reopens via `open_bill`; no extra hook needed unless the bill is re-entered from the ledger (same path).

---

## 10. Backup changes

`crates/treasurer-core/src/backup.rs`:

- `BackupBillShare` gains:

```rust
#[serde(default)]
pub additional_charge_minor: i64,
```

  `#[serde(default)]` keeps **version 1** backups (without the field) importable as 0, and avoids a version bump. `format`/`version` validation stays `version == 1`.
- Export SQL selects `additional_charge_minor`; the struct literal includes it.
- Import insert includes the column.
- `validate_backup`: charge must be `0..=MAX_AMOUNT_MINOR`; the existing "shares must equal bill total" check stays on **base** amounts only, so charged bills still validate.
- `backup_counts` is unchanged.

---

## 11. Edge cases

| Case | Expected |
| --- | --- |
| Name filter matched nothing | Empty state with "Clear filter"; Payments untouched |
| Name typed with different case / extra spaces | Trim + case-insensitive substring match |
| Filter active, Pay tapped | Pays the correct **visible** share (index mapping preserved) |
| Filter active, payment recorded | Detail refreshes and the filter is reapplied; a now-paid boarder may drop out of "Unpaid" |
| Filter active, leave and reopen the bill | Filter is reset to empty + All |
| Tap a boarder's name | Edit sheet opens with that share and a live preview |
| Tap Pay | Records a payment; must not open the edit sheet |
| 300 / 3 boarders, type 50 | 50 / 125 / 125 |
| Type 50 when one other row was previously 60 | 50 / 125 / 125 (the 60 is reset to equal) |
| Type 0 for one boarder | Pinned at 0; the others split the full 300 |
| Type the full total for one of several boarders | Blocked: "Leave room for the other boarders." |
| One-share bill, type less than total | Blocked: "The only share must equal the bill total." |
| Amount > total | Inline error, Save disabled |
| Remainder too small to split on the step | Inline error, Save disabled |
| Centavos off, type 50.50 | Inline error: whole pesos only |
| Pinned amount leaves a remainder not divisible by the step | Leftover centavos go to the earliest non-pinned shares (same rule as equal split) |
| Charge 65 on a 100 share | Row: 165 left to pay, "100.00 paid of 100.00", "+ 65.00 additional charge"; bill Expected 365, Total 300 |
| Charge set to 0 | Charge line disappears; balance returns to the base share |
| Charge on a fully settled bill | Bill status returns to Open; outstanding rises |
| Charge on a partially paid share | Balance rises by the charge; existing payments stay applied |
| Locked bill (any payment) | Share amount field disabled; charge editable; preview shows base amounts unchanged |
| Archived bill | Both fields editable; warning banner; changes persist and survive restore |
| Archived + locked | Charge editable, share amount disabled |
| Archived bill summary | Status shows "Archived" (archived wins over Settled/Open) |
| Dashboard after charge | Expected/outstanding/progress include the charge in the selected month |
| Boarder ledger after charge | The charged boarder's expected/outstanding rise; other boarders unaffected |
| History after charge | Month totals include charges |
| Backup export → import | Charges preserved exactly |
| Old backup without charges | Imports with charges 0 |
| Legacy DB without the column | Migration adds it; existing rows read 0 |
| Two share rows with the same display name | Filter matches both; tapping either edits its own share (ID-based) |
| Rapid Save taps | Second call is a no-op (same values) or harmless; one notification |
| Very small screen | Filter row stacks full width; preview list scrolls inside the sheet |

---

## 12. Acceptance criteria

1. A bill's Shares section shows a name field and All/Unpaid/Paid chips; typing filters rows live, case-insensitively, and Payments/Archive stay unaffected.
2. The filter resets to empty + All after leaving and reopening a bill, and survives in-bill refreshes such as recording a payment.
3. Filtering does not break Pay: the Pay button pays the boarder shown on that row.
4. Tapping a share's name (not Pay) opens "Edit share" with Share amount, Additional charge, and a live preview of every boarder's resulting amount.
5. Saving `50` on a 300/3 bill yields 50 / 125 / 125, and the bill Total stays 300.
6. The same redistribution behavior applies on the New Bill form when one or more amounts are typed.
7. Adding a 65 charge to a 100 share yields a 165 balance for that boarder only; other boarders are unchanged.
8. The charge appears as its own line on the row; the bill's Expected becomes 365 while the subtitle Total stays 300.
9. Charges appear in dashboard totals, bill outstanding/progress, history totals, boarder balances, and the boarder ledger.
10. Share amounts cannot be changed once a payment exists; charges still can; the sheet explains both.
11. Archived bills can still be edited and show a warning.
12. Adding a charge to a settled bill returns it to Open.
13. Invalid input (over total, cannot split, whole pesos only, negative/invalid charge) shows a specific inline error and blocks Save.
14. Charges survive backup export/import, and old backups without the field import with zero charges.
15. Existing databases migrate in place; existing bills read with charge 0.
16. `cargo test -p treasurer-core` passes, including new tests for resplit, charges, migration, and backup round-trip.
17. `cargo check -p treasurer-mobile` (or `cargo build`) passes with no new warnings from touched files.

---

## 13. Verification plan

1. `cargo test -p treasurer-core` — new and existing tests.
2. `cargo check -p treasurer-mobile` — Slint markup compiles.
3. Manual run with the desktop build (`cargo run -p treasurer-mobile --bin treasurer`) against a copy of a real database:
   - open a bill with 3 shares; type a partial name and check filtering + chips;
   - tap a name, type 50, confirm preview and saved 50/125/125;
   - add a 65 charge, confirm row, Expected, outstanding, dashboard, history, and boarder ledger;
   - record a payment, reopen the sheet, confirm the amount field is disabled while the charge field still works;
   - archive the bill, edit a charge, restore it;
   - settle a bill, then add a charge and confirm it reopens;
   - set centavos off and confirm decimal input is rejected;
   - export a backup containing a charge, wipe/import, confirm the charge returns.
4. Migration check: open an existing pre-change `treasurer.db` (or a copy) and confirm the column is added and bills still load.
5. Android smoke test if available: numeric keyboard, sheet height, and scroll behavior when the preview list is long.

---

## 14. Out of scope

- Multiple itemized charges, charge labels/reasons, recurring late fees, or automatic fee rules.
- Editing bill type, period, due date, note, or total from the bill screen.
- Per-payment allocation controls, refunds, or partial charge waivers.
- Filtering the Payments section, the dashboard, history, or boarders by name.
- Redesigning the summary card, Shares layout, or Payments layout beyond the additions above.
- Charging a boarder who is not on the bill; splitting a charge across boarders.
- Charge-aware receipts/exports beyond the JSON backup.
- Changes to the Expo/native, desktop, or server apps.
- Search icon art (the placeholder label is enough) or a global search feature.

---

## 15. Risks and open questions

1. **Filter index mapping is the highest-risk bug.** Any path that rebuilds the `shares` model must rebuild `display_share_ids` in the same pass; Pay and Edit both resolve through that list. Add a debug assertion that `display_share_ids.len() == shares.len()`.
2. **Nested tap targets.** The Shares row moves from non-tappable to partially tappable. Verify on Android that tapping Pay does not also open the edit sheet, and that tapping the row does not absorb the Pay press.
3. **Expected vs Total semantics.** Expected now legitimately exceeds Total. The spec keeps both labels; confirm the summary does not look like a math error. If it does, the subtitle can become "Bill total 300.00 · Expected 365.00" without changing the model.
4. **Locked-bill preview.** With payments present, changing a charge changes the balance but not the split; the preview must not imply the base share can move. It should show the frozen bases and the new charge.
5. **Legacy share sums.** `bill_detail` currently trusts whatever is in `bill_shares`. If an adopted (Drizzle-era) database has shares that do not sum to the bill total, the first resplit silently normalizes them. Note this in the PR; optionally block resplit with "This bill's shares don't match its total" instead of normalizing.
6. **Money step.** Whole-peso mode forbids charges like 65.50. This follows the existing setting, but late fees are often round numbers; confirm this is acceptable.
7. **Backup compatibility.** Keeping version 1 with `#[serde(default)]` is backward compatible for readers, but an older app importing a new backup will ignore the charge. Acceptable for a local-first app; flagged for awareness.
8. **Migration on adopted databases.** `ensure_schema` deliberately skips DDL when `bills` exists. The new `ensure_columns` must be independent of that shortcut and must not assume `bill_shares` exists.
