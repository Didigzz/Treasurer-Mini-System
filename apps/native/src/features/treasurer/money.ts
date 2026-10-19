export const MINOR_UNITS_PER_UNIT = 100;
export const MAX_AMOUNT_MINOR = 9_999_999_999;

const MONTH_NAMES = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
] as const;

export const CURRENCY_OPTIONS = [
  { code: "PHP", label: "Philippine peso (PHP)" },
  { code: "USD", label: "US dollar (USD)" },
  { code: "EUR", label: "Euro (EUR)" },
  { code: "GBP", label: "British pound (GBP)" },
  { code: "AUD", label: "Australian dollar (AUD)" },
  { code: "CAD", label: "Canadian dollar (CAD)" },
  { code: "SGD", label: "Singapore dollar (SGD)" },
  { code: "HKD", label: "Hong Kong dollar (HKD)" },
  { code: "NZD", label: "New Zealand dollar (NZD)" },
  { code: "MYR", label: "Malaysian ringgit (MYR)" },
  { code: "THB", label: "Thai baht (THB)" },
  { code: "AED", label: "UAE dirham (AED)" },
  { code: "SAR", label: "Saudi riyal (SAR)" },
] as const;

export type CurrencyCode = (typeof CURRENCY_OPTIONS)[number]["code"];

export function isCurrencyCode(value: string): value is CurrencyCode {
  return CURRENCY_OPTIONS.some((option) => option.code === value);
}

/**
 * Parse a user-entered amount into integer minor units without floating-point
 * multiplication. Returns null for invalid, negative, or over-limit input.
 */
export function parseMoneyToMinor(input: string): number | null {
  const trimmed = input.trim();
  if (!trimmed) return null;

  if (trimmed.startsWith("-")) return null;

  const hasCommas = trimmed.includes(",");
  if (hasCommas && !/^\d{1,3}(,\d{3})*(\.\d{0,2})?$/.test(trimmed)) {
    return null;
  }

  const normalized = trimmed.replace(/,/g, "");
  if (!/^\d*(\.\d{0,2})?$/.test(normalized)) return null;
  if (!/\d/.test(normalized)) return null;

  const [wholePart = "0", decimalPart = ""] = normalized.split(".");
  const whole = Number.parseInt(wholePart || "0", 10);
  const cents = Number.parseInt(decimalPart.padEnd(2, "0") || "0", 10);

  if (!Number.isFinite(whole) || !Number.isFinite(cents)) return null;

  const minor = whole * MINOR_UNITS_PER_UNIT + cents;
  if (!Number.isSafeInteger(minor) || minor < 0 || minor > MAX_AMOUNT_MINOR) {
    return null;
  }

  return minor;
}

export function minorToInputValue(minor: number): string {
  const whole = Math.floor(minor / MINOR_UNITS_PER_UNIT);
  const cents = Math.abs(minor % MINOR_UNITS_PER_UNIT);
  return `${whole}.${String(cents).padStart(2, "0")}`;
}

export function formatMoney(minor: number, currencyCode: string): string {
  const amount = minor / MINOR_UNITS_PER_UNIT;

  try {
    return new Intl.NumberFormat("en-PH", {
      style: "currency",
      currency: currencyCode,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
      currencyDisplay: "narrowSymbol",
    }).format(amount);
  } catch {
    return `${currencyCode} ${formatNumber(amount)}`;
  }
}

function formatNumber(amount: number): string {
  try {
    return new Intl.NumberFormat("en-PH", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(amount);
  } catch {
    return amount.toFixed(2);
  }
}

export function formatPeriod(year: number, month: number): string {
  return `${MONTH_NAMES[month - 1] ?? `Month ${month}`} ${year}`;
}

export function formatShortPeriod(year: number, month: number): string {
  const name = MONTH_NAMES[month - 1] ?? `Month ${month}`;
  return `${name.slice(0, 3)} ${year}`;
}

export function formatDate(epochMs: number): string {
  return new Intl.DateTimeFormat("en-PH", {
    month: "short",
    day: "numeric",
    year: "numeric",
  }).format(new Date(epochMs));
}

export function formatDateTime(epochMs: number): string {
  return new Intl.DateTimeFormat("en-PH", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(epochMs));
}

export function formatDateOnly(isoDate: string): string {
  const parts = isoDate.split("-").map(Number);
  if (parts.length !== 3) return isoDate;
  const [year, month, day] = parts;
  if (!year || !month || !day) return isoDate;
  return new Intl.DateTimeFormat("en-PH", {
    month: "short",
    day: "numeric",
    year: "numeric",
  }).format(new Date(year, month - 1, day));
}

export function toIsoDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function isoDateToDate(isoDate: string): Date {
  const [year, month, day] = isoDate.split("-").map(Number);
  return new Date(year ?? 1970, (month ?? 1) - 1, day ?? 1);
}

export function todayIsoDate(now = new Date()): string {
  return toIsoDate(now);
}

export function isOverdue(dueDate: string | null, hasBalance: boolean, now = new Date()): boolean {
  if (!dueDate || !hasBalance) return false;
  return dueDate < todayIsoDate(now);
}

export function daysOverdue(dueDate: string, now = new Date()): number {
  const due = isoDateToDate(dueDate);
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  return Math.max(0, Math.round((today.getTime() - due.getTime()) / 86_400_000));
}

export function monthKey(year: number, month: number): string {
  return `${year}-${String(month).padStart(2, "0")}`;
}

export function parseMonthKey(value: string): { year: number; month: number } | null {
  const match = /^(\d{4})-(\d{2})$/.exec(value);
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  if (year < 2000 || year > 2100 || month < 1 || month > 12) return null;
  return { year, month };
}

export function currentPeriod(now = new Date()): { year: number; month: number } {
  return { year: now.getFullYear(), month: now.getMonth() + 1 };
}

export function shiftPeriod(
  year: number,
  month: number,
  delta: number,
): { year: number; month: number } {
  const date = new Date(year, month - 1 + delta, 1);
  return { year: date.getFullYear(), month: date.getMonth() + 1 };
}
