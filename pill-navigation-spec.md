# Pill Navigation — Feature Spec

**Status:** Draft, ready for implementation
**Request:** Copy the design in `pill-navigation.png`
**Short name:** pill-navigation
**Platform:** `apps/native` (Expo / React Native, Android + iOS + web build)
**Scope:** Navigation chrome only. No database, API, or business-logic changes.

---

## 1. Summary

Replace the default four-tab bottom bar in `apps/native` with a floating pill navigation that matches `pill-navigation.png`:

- Three equal tabs inside one rounded capsule: **Dashboard**, **Boarders**, **History**.
- A separate circular **+** button sits outside the capsule, to its right.
- Tapping **+** turns it into a blue-to-purple gradient circle and opens a small dark popover with a single item: **Settings** (gear icon).
- Tapping **Settings** pushes the existing Settings screen as a normal stack screen (with the standard back header).
- The pill disappears while the keyboard is open and while any pushed screen (bill, payment, boarder detail, Settings, bill types) is open.

The visual language is a literal copy of the mock: dark translucent pill, colored custom icons, active item shown by a lighter capsule, floating + with a gradient open state.

---

## 2. Reference

- Mock file: `pill-navigation.png` at the repo root.
- Frame 1: idle pill (Dashboard active) + dark idle `+` circle.
- Frame 2: `+` active — gradient circle, popover above it containing two rows (Reservations, Amenities) with a tail/arrow. In this app the popover has exactly one row, **Settings**.

The mock is dark-only. Light-mode behavior was decided in the interview (see §4).

---

## 3. Current state (verified)

| Area | Current behavior |
| --- | --- |
| `apps/native/app/(tabs)/_layout.tsx` | Expo Router `Tabs` with four screens: `index` (Dashboard), `boarders`, `history`, `settings`. Uses default bottom tab bar, header shown, `useThemeColor` tokens. |
| Tab screens | Each uses `Screen` → `Container`, which pads the scroll content by `insets.bottom + 24` and does **not** add a top safe-area inset because headers currently provide it. |
| Settings | `apps/native/app/(tabs)/settings.tsx` — currency, bill types, backup/restore, storage, centavos, appearance, about. Reachable only via its tab. |
| Root stack | `apps/native/app/_layout.tsx` registers `bills/new`, `bills/[id]`, `history/[id]`, `boarders/[id]`, `bill-types`, `payments/new`. Header styles are set per stack. |
| Installed deps | `react-native-svg@15.16.0`, `expo-haptics`, `react-native-keyboard-controller`, `heroui-native` (including a compound `Popover`), `react-native-reanimated@4.7.1`, `expo-router@58`. |
| Not installed | `expo-linear-gradient`, `expo-blur` (not needed). |
| Theme | `heroui-native` + `uniwind`; `light` / `dark` themes, system default; app-level theme toggle exists. Dark background is `oklch(12% …)`, accent is blue. |

---

## 4. Interview decisions (source of truth)

| # | Decision | Answer |
| --- | --- | --- |
| 1 | Which three tabs | **Dashboard, Boarders, History** (Settings leaves the tab row) |
| 2 | What `+` does | **Opens a small popover with one item: Settings** |
| 3 | Visual fidelity | **Match the mock exactly** (dark pill, colored icons, gradient +) |
| 4 | Labels | **Keep existing wording**: Dashboard, Boarders, History |
| 5 | Settings presentation | **Pushed stack screen** with back header; pill hidden |
| 6 | Light mode | **Always dark pill**, muted accent treatment for icons/labels in light theme |
| 7 | Gradients | **Use `expo-linear-gradient`** |
| 8 | Icon art | **Custom SVG icons** closely matching the mock |
| 9 | Third tab icon | **Purple clock for History** (not the mock's bed) |
| 10 | `+` open state | **Idle dark +; on open becomes gradient and shows an arrowed popover** |
| 11 | Keyboard | **Hide the pill while the keyboard is open** |
| 12 | Content clearance | **Add bottom padding on the three tab screens only** |
| 13 | Motion | **Spring-sliding active highlight + icon scale + haptics** |
| 14 | Wide screens | **Pill group centered with a capped width** |
| 15 | Popover row | **Gear icon + “Settings” only** (no subtitle) |
| 16 | Settings content | **Navigation only — do not redesign the Settings screen** |
| 17 | Tab headers | **Remove the header bar on the three tabs**, content only |
| 18 | Popover scrim | **No dim**; outside tap closes |
| 19 | Haptics platform | **iOS and Android** |
| 20 | Pushed screens | **Pill hidden completely** |

---

## 5. Information architecture

### 5.1 Tab routes (unchanged files, new chrome)

| Route | File | Label | Icon |
| --- | --- | --- | --- |
| `/` | `app/(tabs)/index.tsx` | Dashboard | Colorful bar-chart |
| `/boarders` | `app/(tabs)/boarders.tsx` | Boarders | Purple person |
| `/history` | `app/(tabs)/history.tsx` | History | Purple clock |

All three tabs keep their current screen content, filters, dialogs, and data behavior.

### 5.2 Settings

- Move `app/(tabs)/settings.tsx` → `app/settings.tsx` (content unchanged).
- Register in the root stack: `<Stack.Screen name="settings" options={{ title: "Settings" }} />`.
- Navigate with `router.push("/settings")` after closing the popover.
- Back returns to the previously active tab, which stays active in the pill.
- `/settings` becomes a valid typed route; regenerate Expo Router types before typechecking.

### 5.3 Surfaces where the pill is **not** rendered

Because the pill is implemented as the `Tabs` tab bar, it naturally does not render on stack routes: `bills/new`, `bills/[id]`, `history/[id]`, `boarders/[id]`, `bill-types`, `payments/new`, `settings`. No screen-specific hiding logic is required.

---

## 6. Visual specification

### 6.1 Layout

```
screen bottom
┌──────────────────────────────────────────────┐
│   ┌────────────────────────────┐   ┌──────┐  │
│   │  [Dashboard] Boarders Hist │   │  +   │  │
│   └────────────────────────────┘   └──────┘  │
│                    ↑ inset.bottom + 12        │
└──────────────────────────────────────────────┘
```

- The pill and the + are one centered horizontal group.
- Group horizontal padding: 16 dp from the screen edges.
- Pill: flexible width, `maxWidth: 300`; `height: 68`; `borderRadius: 34`.
- FAB: `56 × 56`, `borderRadius: 28`, `marginLeft: 12`, vertically centered with the pill.
- On screens narrower than 400 dp the pill shrinks; the FAB size stays fixed.
- On tablets the group stays centered at its capped width; it must not stretch.
- Vertical placement: `bottom = insets.bottom + 12`.
- The group floats over content. Outer wrapper uses `pointerEvents="box-none"` so taps outside the controls reach the screen.

### 6.2 Pill internals

- Three equal-width items (`flex: 1`).
- Item content: icon (26 × 26) above label (`fontSize: 11`, `fontWeight: 600`, `lineHeight: 14`), gap 3–4.
- Item height: full pill height minus 8 dp inset.
- Active item background: lighter capsule, inset 4 dp on all sides, `borderRadius: 26`.
- Inactive item: transparent background, muted label.
- Labels are always visible (mock behavior), one line, `numberOfLines={1}`, `maxFontSizeMultiplier={1.4}`.
- Whole item is the tap target (minimum 48 × 68).

### 6.3 Palette (fixed “dark chrome”, not theme-driven)

These values are intentionally independent of the light/dark theme so the pill matches the mock in both modes.

| Token | Value | Use |
| --- | --- | --- |
| `pillBg` | `#141416` at 96% opacity | Pill background |
| `pillBorder` | `rgba(255,255,255,0.10)`, 1 px | Pill outline |
| `activeCapsule` | `#2C2C2E` | Active item capsule |
| `activeCapsuleBorder` | `rgba(255,255,255,0.06)`, 1 px | Subtle capsule outline |
| `fabIdle` | `#2A2A2C` | Idle + background |
| `fabBorder` | `rgba(255,255,255,0.08)`, 1 px | Idle + outline |
| `labelActive` | `#FFFFFF` | Active label |
| `labelInactive` | `#9C9CA3` | Inactive labels |
| `popoverBg` | `#2C2C2E` | Popover surface |
| `popoverBorder` | `rgba(255,255,255,0.08)`, 1 px | Popover outline |
| `popoverLabel` | `#FFFFFF` | “Settings” label |
| `iconTile` | `#F4F4F6` | White tile behind the gear |
| `gear` | `#7C3AED` | Gear glyph |
| `fabGradientStart` | `#5B8CFF` | Gradient top-left |
| `fabGradientEnd` | `#B45CFF` | Gradient bottom-right |
| `iconPurple` | `#8B5CF6` | Person/clock/bed-frame strokes |
| `iconPurpleSoft` | `#A78BFA` | Person head, clock hands |
| `iconBlue` | `#4C8DFF` | Chart bar, bed blanket |
| `iconTeal` | `#3DDC97` | Chart bar |
| `iconPink` | `#FF6B8A` | Chart bar |
| `iconAmber` | `#FFC24B` | Chart bar |

Shadows: pill and FAB get a soft black shadow (iOS `shadowOpacity 0.35`, radius 18, offset y 8; Android `elevation 12`). Popover: `shadowOpacity 0.4`, radius 20, `elevation 16`.

### 6.4 Custom SVG icons

Use `react-native-svg` (already installed). All icons are 26 × 26 with `viewBox="0 0 26 26"` and rounded caps.

1. **Dashboard** — four rounded vertical bars of different heights, like the mock:
   - teal (`#3DDC97`), blue (`#4C8DFF`), pink (`#FF6B8A`), amber (`#FFC24B`);
   - bar width ≈ 4.5, radius 2.25, heights ≈ 10 / 16 / 22 / 13, baseline aligned;
   - very subtle white top highlight is optional.
2. **Boarders** — person silhouette:
   - head circle, `iconPurpleSoft`;
   - shoulders/body path with rounded corners, `iconPurple`.
3. **History** — clock:
   - ring stroke 2.4, `iconPurple`;
   - hands stroke 2.4, `iconPurpleSoft`;
   - small hour ticks optional.
4. **Settings (popover)** — Ionicons `settings` (or `settings-sharp`) glyph at 16–18 px inside the white tile, colored `gear` purple. No custom SVG needed.

Icons must remain recognizable at 26 px and render identically on iOS, Android, and web.

### 6.5 Light-theme accent muting

The pill, active capsule, FAB idle, and popover stay dark in light mode. Only accent treatment changes:

- Icon colors are desaturated/softened by roughly 15–20% (mix toward `#8A8A90`), so they do not glare against a light app background.
- Inactive labels use `#B4B4BA` in light mode instead of `#9C9CA3`.
- Active label stays `#FFFFFF`.
- Borders may be reduced to `rgba(255,255,255,0.08)`.
- No blur/glass is used; the pill is a solid translucent dark surface.

### 6.6 Popover

- Anchored above the FAB, right-aligned to it, with a small tail/arrow pointing at the FAB (mock behavior).
- Size: width ≈ 188 dp; content padding 4; radius 18; background `popoverBg`; 1 px border; arrow fill matches `popoverBg`.
- Single row:
  - height ≈ 52 dp;
  - leading white rounded tile 28 × 28, radius 8, with purple gear;
  - label “Settings”, 15 px, semibold, `popoverLabel`;
  - pressed state: row background `rgba(255,255,255,0.06)`, radius 12.
- No scrim/dim over the screen. Tapping outside closes the menu.
- The + switches from `fabIdle` to the gradient while open (and scales to ~1.04). Closing returns it to idle.
- The popover must close on: Settings tap, outside tap, Android hardware back, tab change, keyboard open, and when the tab bar unmounts.

### 6.7 States

| Element | Idle | Pressed | Active/Open |
| --- | --- | --- | --- |
| Tab item | transparent, muted label | opacity 0.85 | `activeCapsule` + white label, icon scale 1.06 |
| FAB | `fabIdle` + white plus | opacity 0.85 | blue→purple gradient + plus, scale 1.04 |
| Popover row | transparent | `rgba(255,255,255,0.06)` | n/a (navigates) |

---

## 7. Interaction behavior

### 7.1 Tab press

1. Fire light haptic (`Haptics.impactAsync(ImpactFeedbackStyle.Light)`) on iOS and Android; ignore failures (web is a no-op).
2. Navigate with the standard tab navigation event:
   - if already focused, scroll the screen to top only if the screen already supports it — **not required** in this iteration; otherwise no-op;
   - otherwise emit `tabPress` + navigate via the `BottomTabBarProps.navigation` helpers.
3. Animate the active capsule to the new index with a spring; scale the newly active icon up and the previous icon back.
4. Close the popover if it was open.
5. Inactive screens stay mounted (default Tabs behavior), preserving scroll/filter state.

### 7.2 + press

1. Fire light haptic.
2. Toggle the popover. While opening, animate the FAB to the gradient state.
3. Popover appears with a short spring (scale 0.96 → 1, translateY 8 → 0, opacity 0 → 1, ~160 ms). Exit is a 120 ms fade/scale.
4. While open, FAB accessibility state is `expanded`.

### 7.3 Settings row press

1. Close the popover.
2. Restore the FAB to idle.
3. `router.push("/settings")`.
4. The tab press that was in flight is not duplicated; avoid double navigation on rapid taps.

### 7.4 Keyboard

- Use `useKeyboardState((state) => state.isVisible)` from `react-native-keyboard-controller` (provider is already mounted in the root layout).
- When visible: close the popover and animate the whole group down by ~120 dp + `insets.bottom` with opacity → 0, ~180 ms, so forms have full room.
- When hidden: animate back to the resting position, ~200 ms spring.
- Skip this behavior on web.
- The pill must not reflow the screen; it slides out, content does not jump.

### 7.5 Motion

- Active capsule: `translateX` spring (`damping 18`, `stiffness 220`, `mass 0.7`).
- Icon scale: `withSpring(1.06)` on active, `withSpring(1)` on inactive.
- FAB gradient: crossfade/cross-scale 150 ms (two layered circles: idle and gradient, animate opacity).
- Popover: as in §7.2.
- Respect `useReducedMotion()` from `react-native-reanimated`: when true, snap states instantly and skip the popover scale/translate (fade only, or no animation).
- Do not animate layout properties that trigger screen re-measurement.

### 7.6 Haptics

- `expo-haptics` `impactAsync(Light)` on every tab change and every + press.
- Only on press-in of a *different* tab, to avoid buzz on re-tapping the active tab.
- Wrapped in a helper that silently catches errors and does nothing on web.

---

## 8. Implementation plan

### 8.1 Files

**New**

- `apps/native/components/navigation/pill-tab-bar.tsx` — the custom tab bar, popover, keyboard hiding, haptics, motion.
- `apps/native/components/navigation/pill-tab-icons.tsx` — custom SVG icons (Dashboard, Boarders, History) and icon color helpers.
- `apps/native/components/navigation/pill-theme.ts` — fixed colors, dimensions, spring configs, and `PILL_TAB_BAR_CLEARANCE`.

**Moved**

- `apps/native/app/(tabs)/settings.tsx` → `apps/native/app/settings.tsx` (content unchanged; use `git mv`).

**Edited**

- `apps/native/app/(tabs)/_layout.tsx`
  - `headerShown: false` for all three screens;
  - remove the `settings` screen;
  - pass `tabBar={(props) => <PillTabBar {...props} />}`;
  - keep `sceneStyle: { backgroundColor: background }`;
  - remove default tab bar styling options that no longer apply.
- `apps/native/app/_layout.tsx`
  - register `<Stack.Screen name="settings" options={{ title: "Settings" }} />`.
- `apps/native/src/features/treasurer/components/ui.tsx` (`Screen`)
  - add optional `topInset?: boolean` (adds `insets.top + 8` to scroll content top);
  - add optional `bottomClearance?: number` (extra bottom padding).
- `apps/native/components/container.tsx`
  - forward the new top/bottom content padding through `contentContainerStyle` (and the non-scrollable variant).
- `apps/native/app/(tabs)/index.tsx`, `boarders.tsx`, `history.tsx`
  - use `topInset`;
  - add `PILL_TAB_BAR_CLEARANCE` (≈104 dp) as bottom clearance so the last row can scroll above the pill.
- `apps/native/package.json`, `bun.lock`
  - add `expo-linear-gradient` at the Expo SDK 58-compatible version (install with `bunx expo install expo-linear-gradient` from `apps/native`).

**No changes**

- Any repository, query, schema, server, or backup code.
- Settings screen contents and behavior.
- Desktop/mobile (Tauri/Slint) apps.

### 8.2 `PillTabBar` composition

Use the `BottomTabBarProps` supplied by Expo Router's `Tabs` (`state`, `descriptors`, `navigation`, `insets`).

- Render an absolutely positioned, centered row above the bottom safe area.
- Map `state.routes` → the three visible items in route order (`index`, `boarders`, `history`); read label/icon from a local config keyed by route name (do **not** depend on `options.tabBarIcon`, which is no longer used).
- Active index = `state.index`.
- Active capsule position: `onLayout` the pill to get inner width, `itemWidth = (width - 8) / 3`, indicator `left = 4 + activeIndex * itemWidth`.
- Navigation: use `navigation.emit({ type: 'tabPress', target: route.key, canPreventDefault: true })` then `navigation.navigate(route.name)` following the standard React Navigation custom tab bar pattern, respecting `tabBarButton` if present.
- FAB uses `Popover.Root` + `Popover.Trigger` (heroui-native) with a custom dark `Popover.Content` and `Popover.Arrow`, or an equivalent hand-rolled overlay if the library's arrow/background cannot be matched exactly. Requirements are behavior-first: anchored above FAB, tail, no dim, outside-tap dismissal, Android back dismissal.
- Popover content is built locally (white gear tile + “Settings”); do not reuse `AppDialog` (it is a centered modal and would add a scrim).
- Hide group on keyboard using the animated style described in §7.4.
- Accessibility: see §9.

### 8.3 Content insets

- `Container` currently: `paddingBottom: insets.bottom + 24`.
- Tab screens pass `bottomClearance={PILL_TAB_BAR_CLEARANCE}` so the total is `insets.bottom + 24 + 104`.
- Because headers are removed, tab screens also need `topInset` → `insets.top + 8`. Without it, content renders under the status bar/notch.
- Non-tab screens keep current behavior (`topInset` false, clearance 0), since they still have stack headers.

### 8.4 Theming

- Pill/popover/FAB colors come from `pill-theme.ts` constants, not `useThemeColor`, to guarantee the mock look in light mode.
- Use `useAppTheme()` only to choose the muted accent variant (§6.5).
- App background remains theme-driven; do not force a black app background on tab screens.

---

## 9. Accessibility

- Pill container: `accessibilityRole="tablist"` where supported.
- Each tab: `accessibilityRole="tab"`, `accessibilityState={{ selected }}`, `accessibilityLabel` = label, hint e.g. “Opens the Boarders tab”.
- Active item must be distinguishable without color alone (capsule + white label + icon scale).
- FAB: `accessibilityRole="button"`, `accessibilityLabel="Settings menu"` (or “Open settings menu”), `accessibilityState={{ expanded }}`.
- Popover row: `accessibilityRole="menuitem"` (or button), label “Settings”, hint “Currency, backup, and other app settings”.
- Popover should be treated as a modal for screen readers while open (`accessibilityViewIsModal` / equivalent), and focus should move to the first row.
- All tap targets ≥ 44 × 44; the FAB is 56 × 56, each tab item is ~98 × 60.
- Support dynamic type up to `maxFontSizeMultiplier={1.4}` without clipping labels; long labels truncate rather than wrap.
- Announce nothing automatically on tab change beyond platform defaults (avoid duplicate announcements).
- Contrast: white on `#141416` and `#9C9CA3` on `#141416` must meet at least 4.5:1 for the small labels; verify and lighten `labelInactive` if needed.

---

## 10. Edge cases

| Case | Expected |
| --- | --- |
| Rapid tab tapping | No duplicate navigation/haptics storms; indicator settles on the last selected tab. |
| Rapid + tapping | Popover toggles cleanly; no stacked popovers. |
| + then tab | Popover closes, FAB returns to idle, tab switches. |
| + then Android back | Popover closes, app does not navigate back. |
| Settings open then back | Returns to the tab that was active; pill re-appears with that tab active. |
| Deep link to `/settings` | Opens the pushed Settings screen with header/back; pill hidden. |
| Deep link to a tab | Correct tab is active in the pill. |
| Keyboard open | Pill hidden; on dismiss it returns without content reflow. |
| Dialog open with input | Keyboard hides the pill; dialogs remain above. |
| Theme toggle while popover open | Pill/popover stay dark; accent muting updates; no remount. |
| Very small phone (~320 dp) | Pill shrinks, labels truncate; FAB remains 56 dp. |
| Tablet / wide web | Group centered at max width; no stretch. |
| Reduced motion enabled | No sliding/scale; instant state changes. |
| Screen reader | Tabs and menu are labeled, selection/expanded state exposed. |
| No haptics hardware | Haptic call fails silently. |

---

## 11. Acceptance criteria

1. The three tab screens show a floating pill with **Dashboard**, **Boarders**, **History**, each with the specified custom colored icon and label.
2. The active tab is shown by a lighter capsule behind icon + label, with a spring transition when switching.
3. A separate **+** circle sits to the right of the pill, vertically centered, and opens a dark popover with exactly one row: gear + “Settings”.
4. The + is dark gray when idle and blue→purple gradient while the popover is open; the popover has a small arrow/tail pointing at it.
5. Tapping “Settings” pushes the existing Settings screen; the app header shows “Settings” and back navigation works.
6. The pill is not visible on Settings or any pushed screen, and returns on back.
7. The pill slides out of view when the keyboard opens and returns when it closes.
8. Content on all three tab screens can scroll fully above the pill; no content is permanently hidden behind it or the status bar.
9. Light and dark themes both show the dark pill with appropriately muted accents in light mode.
10. Motion respects the OS reduce-motion setting; haptics fire on iOS and Android.
11. Tablet/web layouts keep the group centered at capped width.
12. `bun run --cwd apps/native check-types` passes; no new TypeScript or lint errors.
13. No changes to settings functionality, data, or business logic.

---

## 12. Verification plan

1. Install the dependency: `cd apps/native && bunx expo install expo-linear-gradient`.
2. Regenerate typed routes by starting Expo once (`bun run dev` in `apps/native`) so `/settings` is typed.
3. Run `bun run --cwd apps/native check-types`.
4. Run the app on an Android emulator and an iOS simulator:
   - visual comparison against `pill-navigation.png` in dark mode;
   - light-mode check;
   - tap through all three tabs;
   - open popover, tap outside, open again, press Settings, go back;
   - open a keyboard-bearing dialog (Add boarder) and confirm the pill hides/returns;
   - scroll each tab to the bottom and confirm the last row clears the pill;
   - check a pushed bill screen for pill absence;
   - enable reduce motion and confirm the animation changes.
5. Web smoke test (`bun run web`): pill renders, no haptics, no keyboard hiding, popover works.
6. Note any visual drift (especially icon shapes and gradient stops) in the PR description with screenshots.

---

## 13. Out of scope

- Add-bill / add-payment quick actions from the + menu.
- Badges, notification dots, or Settings-in-pill indicators.
- Renaming tabs to Tenants/Rooms.
- Restyling the Settings screen contents.
- Replacing the app header globally, or changing headers of pushed screens.
- Tab bar scroll-hide behavior (only keyboard hiding is in scope).
- Syncing this navigation to the desktop/mobile (Tauri/Slint) apps.
- Any database, backup, schema, or server changes.

---

## 14. Risks and open questions

1. **Icon fidelity:** the mock's dashboard icon is a specific multicolor glyph; the SVG will need one visual-tuning pass on device to match proportions.
2. **Popover implementation:** `heroui-native` `Popover` supports portals, arrows, and outside dismissal, but its themed background assumes app surface tokens. If theming the arrow/background to the fixed dark palette fights the library, switch to a hand-rolled absolute overlay inside the tab bar (still no dim, still Android-back aware).
3. **Dark pill in light mode:** this is intentional, but verify the shadow and inactive-label contrast on a light background; adjust `labelInactive` if the small text falls below 4.5:1.
4. **Expo Router typed routes:** `router.push("/settings")` will only typecheck after route generation; note this for CI if the typegen step is not part of `check-types`.
5. **Gradient package version:** use `bunx expo install` so the version matches Expo SDK 58; do not hand-pick a version.
6. **Headers removed:** top inset behavior must be confirmed on devices with a notch and on Android edge-to-edge; this is the most likely source of layout regressions.
