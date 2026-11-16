# Share Row Name Visibility and Mobile Layout — Specification

**Status:** Draft, ready for implementation
**Date:** 2026-11-16
**App:** Boarding House Treasurer 1.4.0 → 1.4.1
**Scope:** Restack the bill screen's share row on narrow phones so the boarder name is readable, move per-share Undo into the Edit share sheet, and give the other name rows a minimum width
**Request:** "this is the UI in mobile, but here in desktop is different, fix the mobile UI, the name is not visible"

This document records what the code does today, the decisions from the clarification interview, the exact layout behavior, the edge cases, and the acceptance checks. No code has been changed yet.

---

## 1. Summary of decisions

| Topic | Decision |
|---|---|
| Reported bug | On a ~360 dp phone the share row shows one or two characters of the boarder name (`A...`, `D...`, `J...`). At the desktop window width the same row shows `Alistair` in full. |
| Root shape | The row is a `HorizontalLayout`: a stretchable left region (name + status pill + chevron) and a fixed right column (Pay over Undo payment). The name is the only element allowed to shrink (`min-width: 0px`, `overflow: elide`), so it absorbs all of the shortfall. |
| Fix strategy | Responsive restack, not a global redesign: keep today's side-by-side row on wide windows, restack the share info on narrow phones. |
| Breakpoint | Reuse the existing `Responsive.compact` flag (`width < 380px`, already fed from `AppWindow.width`). No new breakpoint token. |
| Narrow row | Name first (full row width, up to two lines), status pill on the line beneath it, the paid-of line and charge/change lines under that, then a **full-width Pay button**. |
| Wide row | Unchanged apart from losing the Undo button: name + pill + chevron inline, single Pay button on the right. |
| Undo payment | Removed from the share row **at every width**. Per-share Undo moves into the Edit share sheet as a "Payments on this share" list, newest first, one Undo per active payment. |
| Multi-payment picker | The old "Undo which payment?" sheet was only reachable from the row's Undo; it is removed together with the row callback (see §5.5). |
| Payment rows | The bill's Payments section and the ledger's payment rows keep their own Undo and Return change buttons. Unchanged. |
| Ledger bill rows / dashboard boarder rows / new-bill pick rows | **Minimum name width only.** Keep their current side-by-side structure; they have no Share-row action column crowding them. |
| Long names | Wrap up to **two lines**, then elide. Rows grow taller for two-line names; no horizontal squeeze. |
| Full name elsewhere | The name is never truncated in the accessibility label, the Edit share sheet title, or the Pay sheet subtitle. |
| Pay button | Same component and wording (`Pay`, cash icon). On narrow screens it is full width; on wide screens it stays compact in the right column. |
| Chevron | Kept at the end of the name line on narrow screens (the row still opens the Edit share sheet); the Pay button is a separate target. |
| Data / schema | No schema, query, or backup changes. |
| Tests | `cargo check -p treasurer-mobile`, the existing version test, and a manual desktop pass at 320/360/412/≥420 px plus an Android install pass. No new core tests (no core logic changes). |
| Version | Bump to 1.4.1 (`version_code` 6) in `manifest.yaml`, `Cargo.toml`, and `src/version.rs`; README bullets and the tag example updated. |

---

## 2. Context and current behavior

### 2.1 Architecture

- Rust + Slint, phone-first. UI: `apps/mobile/ui/*.slint`; state and wiring: `apps/mobile/src/app/*.rs`.
- One window (`AppWindow`), `preferred-width: 420px`, `min-width: 320px`.
- Window width is mirrored into the `Responsive` global (`theme.slint`) by `init`/`changed width` in `app.slint`; `Responsive.compact` is `width < 380px` and is already used by `SummaryMetrics` and the New bill footer.
- The bill screen is screen id 10. Its Shares list is built in `bills.rs::render_shares()` and rendered by `BillScreen` in `ui/screens/bill.slint`.
- Design tokens live in `theme.slint`; shared controls in `components.slint` / `layout.slint`.

### 2.2 The share row today

`apps/mobile/ui/screens/bill.slint`, inside `ListCard` → `for share[i] in root.shares : ListRow`:

```
HorizontalLayout (padding 16/12/12/12, spacing 12)
├── Rectangle (horizontal-stretch: 1)      ← tap region, opens Edit share
│   └── VerticalLayout
│       ├── HorizontalLayout
│       │   ├── Text name  (stretch 1, min-width 0, overflow: elide)
│       │   ├── StatusPill share.status
│       │   └── Icon chevron-right (16px)
│       ├── Text "Fully paid" / "{balance} left to pay"
│       ├── Text "{paid} paid of {due}"
│       ├── [Text "+ {charge} additional charge"]
│       └── [Text change-status]
└── VerticalLayout                          ← fixed width, never shrinks
    ├── AppButton "Pay" (compact)
    └── AppButton "Undo payment" (compact, when settled and not archived)
```

### 2.3 Why the name collapses

At a 360 dp window: `Responsive.gutter` is 16, so the card is ~328 wide and the row content ~300. The right column is as wide as the "Undo payment" button (~135 px including padding); spacing is 12; the pill `Paid – Online` is ~90 px plus 12 + 16 chevron + 12. That leaves roughly 25–35 px for the name — about one glyph and an ellipsis, which is exactly the `A...` in the report. At 440 px (the desktop screenshot) the same arithmetic leaves ~110 px, enough for `Alistair`.

So the bug is not a missing binding: the name is the only element with `min-width: 0`, and the fixed-width action column plus the pill take priority.

### 2.4 Other rows

- Boarder ledger bill rows (`screens/ledger.slint`): name + pill + paid-of, chevron on the outside. No action column, so names survive, but a long bill name plus a `Paid – Online` pill can still squeeze below a readable width on 320 px.
- Dashboard "Boarders this month" rows (`screens/dashboard.slint`): avatar + name/stats + amount-or-pill column + chevron.
- New bill "Split among" rows (`screens/new-bill.slint`): checkbox + name + a `Responsive.compact ? 104px : 128px` amount field.

These are not the reported failure; the interview asked for a guaranteed floor only (see §6).

---

## 3. Requirement — narrow share row restacks

### 3.1 Rule

`Responsive.compact` (`width < 380px`) selects the stacked layout. At or above 380 px the wide layout is used, so the desktop preview at its 420 px preferred width keeps the familiar side-by-side row.

### 3.2 Narrow layout (< 380 px)

```
┌────────────────────────────────────────────┐
│ Alistair                                ›  │  name, up to 2 lines
│ [Paid – Online]  Fully paid                 │  pill + balance state
│ ₱64.75 paid of ₱64.75                       │
│ [Change to Return ₱10.00]                   │  when present, tappable
│                                            │
│ ┌────────────────────────────────────────┐ │
│ │  ₱  Pay                                 │ │  full-width Pay
│ └────────────────────────────────────────┘ │
└────────────────────────────────────────────┘
```

- Name: `horizontal-stretch: 1`, `wrap: word-wrap`, `max-lines: 2`, then `overflow: elide`. Minimum expected width is the full row minus the chevron.
- Status pill: on its own line under the name, left-aligned, followed by the existing balance line (`Fully paid` in success, `{balance} left to pay` in danger) on the same line.
  - The pill keeps `Theme.status-tone(share.status)`.
  - If the pill plus the balance text cannot fit one line, the balance text elides before the pill does; the pill is never truncated (it is the status).
- Paid-of line, additional-charge line, and change-status line keep their current order, fonts, and colors.
- Pay: `AppButton` with `horizontal-stretch: 1` (full row width), `compact: false` (48 px tall) so it is a comfortable target, same `kind: share.has-balance ? primary : secondary`, icon `Icons.cash`, text `Pay`.
- Chevron: 16 px, at the end of the name line, vertically aligned to the first line. It remains the only visual cue that the row opens the editor, so it stays.
- The name/pill/paid-of region keeps its tap behavior (opens Edit share) and its full accessible label. The Pay button remains its own tap target outside that region.
- No Undo button anywhere in the row (see §5).

### 3.3 Wide layout (≥ 380 px)

Unchanged from today except that the right column carries only Pay:

```
┌──────────────────────────────────────────────────────────┐
│ Alistair   [Paid – Online]  ›      ┌────────┐            │
│ Fully paid                        │  ₱ Pay │            │
│ ₱600.00 paid of ₱600.00           └────────┘            │
└──────────────────────────────────────────────────────────┘
```

- Name: single line, `overflow: elide`, and now with `min-width: 96px` so it can never be squeezed to a couple of glyphs at 380–420 px either.
- Pill and chevron hold their right-hand position in the info block, as today.
- Pay stays compact (`compact: true`, 40 px) in a `VerticalLayout` column that now contains a single button.

### 3.4 Implementation shape

Slint cannot switch a `HorizontalLayout` to a `VerticalLayout` conditionally, and duplicating the whole info block twice invites drift. Recommended structure in `bill.slint`:

- Extract the info block into a private component in the same file, e.g. `component ShareInfo`, with:
  - `in property <ShareRow> share;`
  - `in property <bool> stacked;` — when true, the pill moves out of the name line onto the balance line and the name allows two lines;
  - the same two callbacks it needs (`edit`), or keep the tap region in the parent and have `ShareInfo` render content only.
- `BillScreen`'s share row then has a single `VerticalLayout` (row padding) containing:
  - `if Responsive.compact : VerticalLayout { ShareInfo { stacked: true; ... } Pay { full width } }`
  - `if !Responsive.compact : HorizontalLayout { ShareInfo { stacked: false; ... } Pay { compact } }`
- Keep the existing `ListRow` (`tappable: false`) wrapper, `show-divider`, label, and accessible text exactly as they are.
- No new theme tokens, colors, radii, or components outside `bill.slint`.
- Guard against layout thrash: the breakpoint only changes on window resize, and `Responsive.compact` is already reactive.

### 3.5 Edge cases — layout

- **320 px window** — name line has ~230 px; most names fit on one line.
- **Two-line names** — the row grows; the divider still spans the row; the Pay button stays full width below both lines.
- **One very long unbroken token** (no spaces) — word wrap cannot break it, so it elides on line 2. The full name remains in the accessibility label and sheet title.
- **Very short names** — no change in appearance beyond the restack.
- **Additional charge and change-status lines present** — they stack under the paid-of line as today; Pay remains last.
- **Archived bill** — no Undo existed in the row for archived bills and there is still none here; Pay remains visible as today.
- **Landscape / tablet** — at ≥ 380 px the wide row applies; nothing new to design.
- **Text at large system font size** — the name wraps to two lines instead of eliding; the pill line may elide its balance text first.
- **Bills with many shares** — nothing changes about scrolling; rows are simply taller on phones.

---

## 4. Requirement — other name rows get a floor

### 4.1 Ledger bill rows (`screens/ledger.slint`)

- Keep the current side-by-side structure (name + pill inline, chevron on the outside).
- Add `min-width` to the name so it cannot collapse: target **96 px** at 320 px width.
- Allow the name to wrap to two lines (`wrap: word-wrap; max-lines: 2; overflow: elide`) so long bill names are readable instead of truncated.
- If at 320 px the name plus a wide pill (`Paid – Online`) cannot both hold, the name keeps its 96 px and the pill is allowed to elide its text before the name shrinks further.

### 4.2 Dashboard boarder rows (`screens/dashboard.slint`)

- Keep avatar, name/stats, amount-or-pill column, and chevron.
- Name: `min-width: 88px`, `wrap: word-wrap`, `max-lines: 2`, `overflow: elide`.
- The trailing amount stays on one line (money is never wrapped); if the pair still does not fit at 320 px, drop the chevron in these rows on `Responsive.compact` only.

### 4.3 New bill pick rows (`screens/new-bill.slint`)

- Keep the checkbox, amount field, and its `Responsive.compact ? 104px : 128px` width.
- Name: `min-width: 88px`, `wrap: word-wrap`, `max-lines: 2`, `overflow: elide`.

### 4.4 What must not change in these rows

Order of elements, icons, colors, tap targets, accessible labels, `show-divider`, and the amount/pill column semantics. The only changes are the name's minimum width, the two-line wrap, and the dashboard chevron fallback.

---

## 5. Requirement — per-share Undo moves into the Edit share sheet

### 5.1 Why

The row's Undo button is the widest thing in the layout and the reason the name lost its space. It also sits next to a destructive action that already has its own confirmation dialog. Moving it into the Edit share sheet removes the squeeze at every width and keeps the payment history in one predictable place.

### 5.2 Sheet content

Add a section to the existing Edit share sheet (`app.slint`, `SheetKind.edit-share`), below the split preview and above Save share / Cancel:

```
Payments on this share
2 payments, newest first

┌──────────────────────────────────────────────┐
│ Nov 16, 2026 at 6:32 PM          ₱600.00     │
│ Cash                              [Undo]     │
├──────────────────────────────────────────────┤
│ Nov 01, 2026 at 9:10 AM          ₱200.00     │
│ Online · Undone                              │
└──────────────────────────────────────────────┘
```

- Section header reuses `SectionHeader` inside the sheet: title `Payments on this share`, caption `1 payment, newest first` / `{n} payments, newest first`; caption empty when there are none.
- Rows are newest first, ordered by `paid_at`, then `created_at`, then `id` — the same ordering the undo picker used.
- Each row shows the date/time (existing `money::format_date` / `format_datetime` formatting), the payment type (`Cash` / `Online`), the amount, and an `Undo` `AppButton` (`ButtonKind.danger-soft`, `compact: true`).
- Active payments show the Undo button; undone payments show an `Undone` `StatusPill` (`Tone.neutral`) and no button.
- If the bill is archived, rows still list but no Undo button appears (matches today's archived rule).
- The list is bounded like the split preview: `ScrollView { height: min(rows * 52px + 2px, 208px) }` so the sheet never exceeds the screen; the sheet's existing max-height clamp still applies.
- No payments: a single muted line `No payments recorded yet.` instead of the card.
- Rows are not tappable and do not open the payment detail sheet (that stays the Payments section's job); the Undo button is the only action.
- Accessibility: the list is a list with an accessible label; each Undo button's label reads `Undo {amount} payment from {date}`; undone rows announce `{date}, {type}, {amount}, undone`.

### 5.3 Undo flow from the sheet

1. Tap Undo on a row → `prepare_undo(payment_id, share_id)` runs the exact guards used today: payment exists, is active, change not already returned, bill not archived. Failures surface in the sheet's own error banner (`Sheet.error` is already bound to `root.sheet-error`).
2. Success fills and opens the existing centered `UndoPaymentDialog` above the sheet, with the same context/warning copy as today.
3. Cancel closes only the dialog; the sheet stays open.
4. Confirm writes the undo, closes the dialog, refreshes every list, and the open sheet re-renders the payment list with that row now `Undone`.
5. `change-returned` payments: keep the Undo button (consistent with today, where the button only disappears once undone) and let the guard explain; alternatively hide it — implementation may pick one, but the row must never reach a state where Undo succeeds illegally.

### 5.4 Rust wiring

- New `App` fields:
  - `share_sheet_payment_ids: Vec<String>` — payment IDs parallel to the sheet list, newest first;
  - reuse `edit_share_id` as the share the list belongs to.
- `open_edit_share(index)` additionally loads the share's payments (from the already-loaded `ShareEntry`/`detail_shares` or `db.share_payment_context`) and sets a new window model, e.g. `share-payments` of a row model (`title`, `subtitle`, `amount`, `undoable`), plus the count caption.
- New callback `undo-share-payment(int)` (or `undo-sheet-payment(int)`) that maps the row index through `share_sheet_payment_ids` and calls `prepare_undo`.
- `refresh_after_change()` (or `undo_confirm`) reloads the sheet list when `edit_share_id.is_some()`, mirroring the existing `if ui.get_screen() == 11 { refresh_ledger() }` pattern. A hidden model refresh is harmless.
- `clear_detail_lists()` / opening another bill clears the new model and IDs so a stale list can never point at another share's payments.

### 5.5 Removals

With Undo gone from the row and the picker gone, these become dead and are removed in the same change:

- `bill.slint`: `callback undo-share(int)`, the row's `Undo payment` `AppButton`, and its `share.settled` condition.
- `app.slint`: `BillScreen.undo-share` wiring, `callback undo-share-payment(int)`, `SheetKind.undo-pick`, the `undo-pick` sheet block, `undo-picker-subtitle`, `undo-payments`, `callback undo-pick(int)`, and the `undo-picker` references in `undo-share`.
- `bills.rs`: `open_undo_share`, `undo_pick`, and the `undo_picker_ids` field; keep `prepare_undo`, `undo_confirm`, `undo_cancel`, `open_undo_payment`, `open_undo_detail` (still used by payment rows and the payment detail sheet).
- `models.slint`: `UndoPickRow` (no remaining users) and `ShareRow.settled` (only ever used by the row Undo), plus the two constructors that fill it (`bills.rs`, `boarders.rs`).
- `mod.rs`: `undo_picker_ids` initialization.

Anything still referenced elsewhere must be kept; the build (`cargo check -p treasurer-mobile`) is the check that nothing was missed.

### 5.6 Edge cases — Undo in the sheet

- **Share with one payment** — one row, Undo records the reversal directly after confirmation (no picker).
- **Share with several active payments** — every active payment is listed with its own Undo; no "which payment?" sheet is needed.
- **Undo the last payment while the sheet is open** — the row becomes `Undone`, the pill on the bill becomes `Unpaid`, the share may leave an active filter; the sheet stays open on the same share.
- **Undo a payment with change outstanding** — the dialog shows the existing change warning; the list refreshes to `Undone`.
- **Payment with change already returned** — the guard blocks it and the sheet banner explains; no partial state is written.
- **Archived bill** — sheet opens, list is read-only (no Undo buttons).
- **Filtered share list** — the sheet is keyed by share ID, so filtering while the sheet is closed can never mis-target Undo.
- **Share deleted / bill deleted mid-flow** — the existing "no longer available" messages still apply through `prepare_undo`.
- **Save share while the list is showing** — both actions are independent; the list refreshes after `refresh_after_change`.

---

## 6. Cross-cutting requirements

### 6.1 Design system and platform

- Only `Theme` tokens; no new colors, radii, or spacing values.
- No new dependencies.
- The wide layout must look identical at 420 px except that the Undo button is gone.
- The narrow layout must fit 320 px with the gutter and card padding unchanged.
- 44 px minimum touch target: the full-width Pay is 48 px tall; the chevron row opens the editor via the whole info region.
- Desktop preview and Android must both be checked (§8).

### 6.2 Accessibility

- The share row's accessible label keeps the full name, status, balance, charge note, and "opens share editor".
- The Pay button keeps its label; on narrow screens the full-width variant uses the same `describe` text.
- The sheet's payment list items are announced with date, type, amount, and undone state; each Undo says which payment it reverses.
- Two-line names do not become color-only or icon-only status anywhere.

### 6.3 Documentation and version

- `apps/mobile/manifest.yaml`: `version_name: 1.4.1`, `version_code: 6`.
- `apps/mobile/Cargo.toml`: `[package] version = "1.4.1"` and `[package.metadata.android] version_code = 6`, `version_name = "1.4.1"`.
- `apps/mobile/src/version.rs`: `VERSION_NAME = "1.4.1"`, `VERSION_CODE = 6`.
- `README.md`:
  - the Undo bullet notes that per-share Undo now lives in the Edit share sheet's payments list, while payment rows keep their own Undo;
  - a short line (in Features, near Share filters) that the bill's share rows restack on narrow phones so names stay readable;
  - the release example command becomes `git tag v1.4.1+6 && git push origin v1.4.1+6`.
- Do not restructure the README.

### 6.4 Out of scope

- Any change to split math, additional charges, change-return accounting, payment recording rules, filters, or the backup format.
- Redesigning the dashboard, ledger, history, settings, or new-bill screens beyond the name floor in §4.
- Moving the Payments section, adding a per-share payment history outside the Edit share sheet, or making payment rows in the sheet open the payment detail.
- A tablet-specific two-column layout.
- New theme tokens or a component library refactor.

---

## 7. Acceptance criteria

### 7.1 Narrow layout (320–379 px)

1. A share named `Alistair` renders `Alistair` in full at 360 px; no `A…`.
2. The status pill sits on the line under the name, with `Fully paid` / `{balance} left to pay` beside it.
3. Pay is a full-width button beneath the share info and is comfortably tappable.
4. No Undo button appears anywhere in the row.
5. A 45-character name shows two lines, then elides; the row grows instead of squeezing.
6. The accessibility label still contains the complete name.
7. Tapping anywhere in the info region opens Edit share; tapping Pay opens the payment sheet.

### 7.2 Wide layout (≥ 380 px)

8. The row keeps name + pill + chevron inline with Pay on the right.
9. Only Pay is in the right column; no Undo.
10. At 380 px with the longest available status text, the name shows at least ~10 characters before eliding (the new `min-width: 96px` floor).
11. The desktop preview at its 420 px default matches the intended desktop look (with Undo relocated).

### 7.3 Undo in the Edit share sheet

12. Opening Edit share on a paid share lists its payments newest first with amount, type, and date.
13. Undo on an active payment opens the existing confirmation dialog above the sheet; Cancel returns to the sheet unchanged.
14. Confirm marks the payment Undone, the list updates in place, the bill totals and pill update, and the row stays on the same share.
15. A share with several active payments offers one Undo per payment; no "which payment?" sheet appears.
16. An archived bill shows the list without Undo buttons.
17. A payment whose change was already returned cannot be undone: the sheet banner explains, and nothing is written.
18. Payment rows in the Payments section and the ledger still offer Undo and Return change exactly as before.

### 7.4 Other rows

19. At 320 px, ledger bill rows, dashboard boarder rows, and new-bill pick rows never display fewer than ~6 characters of a name.
20. Their order, icons, amounts, pills, and tap targets are otherwise unchanged.

### 7.5 Build and release

21. `cargo check -p treasurer-mobile` passes (compiles the Slint UI and all removed symbols).
22. `cargo test -p treasurer-mobile --lib version` passes with 1.4.1 / 6.
23. `cargo test -p treasurer-core` still passes (no core changes expected).
24. Version 1.4.1 / `version_code` 6 in manifest, Cargo.toml, and `version.rs`; README updated.

---

## 8. Verification plan

### 8.1 Automated

```bash
cargo check -p treasurer-mobile
cargo test -p treasurer-mobile --lib version
cargo test -p treasurer-core
```

No new core tests are required: no core logic changes. If any core file is touched, add tests for the touched behavior instead of relying on the UI pass.

### 8.2 Desktop preview

```bash
cargo run -p treasurer-mobile --bin treasurer
```

Resize the window and check the bill screen with a bill that has:

- a short name (`Alistair`), a long name (40+ characters, one with spaces), and one with an additional charge, change to return, and a two-payment history;
- one paid cash, one paid online, one unpaid;
- an archived bill.

Steps: 360 px (stacked, full name, full-width Pay), 320 px (nothing clipped, long name two lines), 379 px (still stacked), 380 px (two-line → single line, wide row; the boundary must not flicker), 420 px (desktop look), then Edit share on a paid share and run the full Undo flow including Cancel, Confirm, archived, and change-returned cases.

### 8.3 Android

Build and install per `docs/installation.md`:

```bash
export ANDROID_HOME="$LOCALAPPDATA/Android/Sdk"
export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/27.1.12297006"
export PATH="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/windows-x86_64/bin:$PATH"
(cd apps/mobile && x build --platform android --arch arm64 --format apk)
adb install -r target/x/debug/android/treasurer-mobile.apk
```

On the phone: confirm full names on the bill screen, the stacked layout with a full-width Pay, the two-line wrap for a long name, scrolling a long share list, opening Edit share and running one Undo end to end, and that the sheet's payments list fits without covering its buttons or the keyboard area.

If no device is available, report the Android pass as unverified rather than claiming it.

### 8.4 Definition of done

- Every acceptance criterion in §7 passes (Android reported honestly).
- `cargo check -p treasurer-mobile`, the version test, and the core tests pass.
- The removed row-Undo symbols leave no dead code and no unused Slint callbacks.
- Version is 1.4.1 / 6 in all three places; README reflects the new Undo location and tag.
- No unrelated files, features, schema, or copy changes are included.

---

## 9. Implementation plan (suggested order)

1. **Slint layout**
   - Extract the share info block in `bill.slint` into a `ShareInfo`-style component with a `stacked` property, or an equivalent single-source structure.
   - Add the compact `VerticalLayout` branch (two-line name, pill under the name, full-width Pay) and keep the wide branch (inline pill, compact Pay, no Undo).
   - Remove the row's Undo button, the `undo-share` callback, and the `settled` dependency.
2. **Other rows**
   - Add the name `min-width` + two-line wrap to `ledger.slint`, `dashboard.slint`, and `new-bill.slint`; add the compact chevron fallback to the dashboard row if needed at 320 px.
3. **Sheet payments list**
   - Add the row model and window properties/callbacks; render the `SectionHeader`, bounded `ScrollView`, row card, empty line, and per-row Undo button in the `edit-share` sheet.
   - Load the list in `open_edit_share`; map the new callback through the ID vector; reload after `refresh_after_change` when the sheet's share is set; clear in `clear_detail_lists`.
4. **Remove the old path**
   - Delete the undo-pick sheet, `undo_picker_ids`, `open_undo_share`, `undo_pick`, the `undo-share-payment` callback wiring, the `UndoPickRow` model, and `ShareRow.settled` in both constructors.
5. **Docs and version**
   - Bump manifest, `Cargo.toml` (both places), `version.rs`; update the README Undo bullet, the layout note, and the tag example.
6. **Verify**
   - Run the checks in §8.1, the desktop script in §8.2, and the Android pass in §8.3.

---

## 10. Resolved interview decisions

- Fix by responsive restack, not by a single global layout.
- Breakpoint is the existing `Responsive.compact` (`< 380px`); wide windows keep the current side-by-side row.
- Narrow layout: name (up to two lines) on top, status pill on its own line beside the balance state, full-width Pay beneath the info.
- Long names wrap to two lines, then elide; the full name is still available to accessibility and the sheet.
- Undo payment leaves the share row at every width and moves into the Edit share sheet as a per-payment list with one Undo per active payment.
- The multi-payment "Undo which payment?" picker is removed with the row path; the payment rows keep their own Undo.
- The ledger, dashboard, and new-bill rows get a minimum name width and two-line wrap only — no restack, since they have no share action column.
- The chevron stays on narrow screens; Pay is a separate full-width target.
- Verification is the desktop preview at 320/360/379/380/420 px plus a real Android build/install.
- This ships as 1.4.1 (`version_code` 6) with README updates.

## 11. Assumptions and open questions

Assumptions (no further input needed):

- `Responsive.compact` already tracks window width correctly on Android (it is fed from `AppWindow.width`), so no platform-specific branch is needed.
- "The name is not visible" means the elided share-row name; the desktop row is the reference for how much name should be readable.
- Row heights may grow on phones; no fixed row height is relied on anywhere in the list.
- The sheet can scroll its payment list independently; the sheet's own panel height clamp remains the outer bound.
- Undo inside the sheet uses the existing confirmation dialog and guards unchanged; only the entry point moves.
- No backup or schema impact, so old data and backups keep working.

Open questions (non-blocking; decide during implementation):

- Exact `min-width` values for the ledger (96 px), dashboard (88 px), and new-bill (88 px) rows may be tuned if 320 px testing shows clipping; the acceptance criterion is the visible-character floor, not the number.
- Whether the dashboard's chevron should already be dropped at `Responsive.compact` or only when the amount column actually crowds the name.
- Whether the sheet's payment list should also offer "Return change" for payments with change outstanding, or stay strictly Undo-only (spec says Undo-only).
- Whether `ShareInfo` should own the tap region or the parent keeps it; either is fine as long as Pay stays a separate target and the accessible label is unchanged.
- Whether a two-line name should also be allowed at 380–420 px if a long name still elides there (spec keeps the wide row single-line for now).
