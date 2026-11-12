/**
 * Fixed "dark chrome" palette for the floating pill navigation.
 *
 * These values are intentionally independent of the app's light/dark theme so
 * the pill matches the design mock in both modes. Light mode only softens the
 * icon accents (see `mutedIconOpacity` in pill-tab-icons.tsx).
 */
export const NAV_COLORS = {
  pillBg: "#141416",
  pillBorder: "rgba(255, 255, 255, 0.10)",
  activeCapsule: "#2C2C2E",
  activeCapsuleBorder: "rgba(255, 255, 255, 0.06)",
  fabIdle: "#2A2A2C",
  fabBorder: "rgba(255, 255, 255, 0.08)",
  labelActive: "#FFFFFF",
  labelInactive: "#9C9CA3",
  labelInactiveLight: "#B4B4BA",
  popoverBg: "#2C2C2E",
  popoverBorder: "rgba(255, 255, 255, 0.08)",
  rowPressed: "rgba(255, 255, 255, 0.06)",
  iconTile: "#F4F4F6",
  gear: "#7C3AED",
  gradientStart: "#5B8CFF",
  gradientEnd: "#B45CFF",
} as const;

export const PILL = {
  height: 68,
  radius: 34,
  maxWidth: 300,
  fabSize: 56,
  gap: 12,
  horizontalMargin: 16,
  bottomOffset: 12,
  maxGroupWidth: 300 + 12 + 56,
  /** Extra bottom padding for tab screens so content clears the floating pill. */
  clearance: 104,
} as const;

export const PILL_SPRING = {
  damping: 18,
  stiffness: 220,
  mass: 0.7,
} as const;

/** Lighter label color used when the app theme is light but the pill is dark. */
export function inactiveLabelColor(isLight: boolean) {
  return isLight ? NAV_COLORS.labelInactiveLight : NAV_COLORS.labelInactive;
}
