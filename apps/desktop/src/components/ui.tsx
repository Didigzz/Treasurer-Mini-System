import {
  AlertCircle,
  CalendarDays,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Inbox,
  Loader2,
  X,
} from "lucide-react";
import { useEffect, type ReactNode } from "react";

import type { BillStatus, ShareFilter, ShareStatus } from "@my-better-t-app/api/treasurer/types";
import { formatPeriod, shiftPeriod } from "@my-better-t-app/api/treasurer/money";

export function cn(...values: Array<string | false | null | undefined>): string {
  return values.filter(Boolean).join(" ");
}

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ButtonSize = "sm" | "md";

const BUTTON_VARIANTS: Record<ButtonVariant, string> = {
  primary: "bg-accent text-accent-ink hover:bg-accent-hover border-transparent",
  secondary: "bg-panel text-ink hover:bg-canvas border-line",
  ghost: "bg-transparent text-ink hover:bg-canvas border-transparent",
  danger: "bg-danger-soft text-danger hover:bg-danger/15 border-transparent",
};

const BUTTON_SIZES: Record<ButtonSize, string> = {
  sm: "h-8 px-3 text-xs gap-1.5",
  md: "h-10 px-4 text-sm gap-2",
};

export function Button({
  variant = "primary",
  size = "md",
  className,
  type = "button",
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
}) {
  return (
    <button
      type={type}
      className={cn(
        "inline-flex items-center justify-center rounded-xl border font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-50",
        BUTTON_SIZES[size],
        BUTTON_VARIANTS[variant],
        className,
      )}
      {...props}
    />
  );
}

export function Card({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  return <div className={cn("card", className)}>{children}</div>;
}

export function PageHeader({
  eyebrow,
  title,
  description,
  actions,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-start justify-between gap-4">
      <div>
        {eyebrow ? <p className="text-xs font-semibold tracking-wide text-muted uppercase">{eyebrow}</p> : null}
        <h1 className="mt-1 text-2xl font-bold tracking-tight text-ink">{title}</h1>
        {description ? <p className="mt-1 max-w-2xl text-sm text-muted">{description}</p> : null}
      </div>
      {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
    </div>
  );
}

export function SectionHeader({
  title,
  subtitle,
  action,
}: {
  title: string;
  subtitle?: string;
  action?: ReactNode;
}) {
  return (
    <div className="mt-7 mb-3 flex flex-wrap items-end justify-between gap-2">
      <div>
        <h2 className="text-sm font-bold text-ink">{title}</h2>
        {subtitle ? <p className="mt-0.5 text-xs text-muted">{subtitle}</p> : null}
      </div>
      {action}
    </div>
  );
}

export function Field({
  label,
  hint,
  required,
  children,
  className,
}: {
  label: string;
  hint?: string;
  required?: boolean;
  children: ReactNode;
  className?: string;
}) {
  return (
    <label className={cn("block", className)}>
      <span className="mb-1.5 flex items-baseline justify-between gap-3">
        <span className="text-xs font-semibold text-ink">
          {label}
          {required ? <span className="ml-0.5 text-danger">*</span> : null}
        </span>
        {hint ? <span className="text-[11px] text-muted">{hint}</span> : null}
      </span>
      {children}
    </label>
  );
}

const BADGE_TONES = {
  neutral: "bg-canvas text-muted border-line",
  accent: "bg-accent-soft text-accent border-accent/20",
  success: "bg-success-soft text-success border-success/20",
  warning: "bg-warning-soft text-warning border-warning/20",
  danger: "bg-danger-soft text-danger border-danger/20",
} as const;

export type BadgeTone = keyof typeof BADGE_TONES;

export function Badge({
  tone = "neutral",
  children,
  className,
}: {
  tone?: BadgeTone;
  children: ReactNode;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-semibold",
        BADGE_TONES[tone],
        className,
      )}
    >
      {children}
    </span>
  );
}

export function billStatusTone(status: BillStatus): BadgeTone {
  return status === "open" ? "accent" : status === "settled" ? "success" : "neutral";
}

export function shareStatusTone(status: ShareStatus): BadgeTone {
  return status === "paid" ? "success" : status === "partial" ? "warning" : "danger";
}

export function BillStatusBadge({ status }: { status: BillStatus }) {
  return (
    <Badge tone={billStatusTone(status)}>
      {status === "open" ? "Open" : status === "settled" ? "Settled" : "Archived"}
    </Badge>
  );
}

export function ShareStatusBadge({ status }: { status: ShareStatus }) {
  return (
    <Badge tone={shareStatusTone(status)}>
      {status === "paid" ? "Paid" : status === "partial" ? "Partial" : "Unpaid"}
    </Badge>
  );
}

export function SummaryCard({
  label,
  value,
  tone = "neutral",
  icon,
}: {
  label: string;
  value: string;
  tone?: "neutral" | "success" | "danger" | "warning";
  icon?: ReactNode;
}) {
  const toneClass = {
    neutral: "text-ink",
    success: "text-success",
    danger: "text-danger",
    warning: "text-warning",
  }[tone];

  return (
    <div className="card flex-1 p-4">
      <div className="flex items-center justify-between gap-2">
        <p className="text-xs font-semibold text-muted">{label}</p>
        {icon ? <span className="text-muted">{icon}</span> : null}
      </div>
      <p className={cn("mt-2 text-xl font-bold tabular-nums", toneClass)}>{value}</p>
    </div>
  );
}

export function EmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <div className="card flex flex-col items-center justify-center px-6 py-10 text-center">
      <div className="flex h-11 w-11 items-center justify-center rounded-full bg-canvas text-muted">
        <Inbox size={20} />
      </div>
      <p className="mt-3 text-sm font-semibold text-ink">{title}</p>
      {description ? <p className="mt-1 max-w-md text-xs text-muted">{description}</p> : null}
      {action ? <div className="mt-4">{action}</div> : null}
    </div>
  );
}

export function LoadingBlock({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="flex items-center justify-center gap-2 py-16 text-sm text-muted">
      <Loader2 className="animate-spin" size={18} />
      {label}
    </div>
  );
}

export function ErrorBlock({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-danger/20 bg-danger-soft px-4 py-3">
      <p className="flex items-center gap-2 text-sm font-medium text-danger">
        <AlertCircle size={16} />
        {message}
      </p>
      {onRetry ? (
        <Button variant="secondary" size="sm" onClick={onRetry}>
          Try again
        </Button>
      ) : null}
    </div>
  );
}

export function SuccessNote({ children }: { children: ReactNode }) {
  return (
    <div className="flex items-center gap-2 rounded-xl border border-success/20 bg-success-soft px-3 py-2 text-sm font-medium text-success">
      <CheckCircle2 size={16} />
      {children}
    </div>
  );
}

export function Modal({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  wide,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children?: ReactNode;
  footer?: ReactNode;
  wide?: boolean;
}) {
  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-ink/30 p-4 backdrop-blur-[2px]"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={cn("card max-h-[85vh] w-full overflow-y-auto p-5 shadow-xl", wide ? "max-w-xl" : "max-w-md")}
      >
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-base font-bold text-ink">{title}</h2>
            {description ? <p className="mt-1 text-xs leading-relaxed text-muted">{description}</p> : null}
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="rounded-lg p-1 text-muted hover:bg-canvas hover:text-ink"
          >
            <X size={16} />
          </button>
        </div>
        {children ? <div className="mt-4">{children}</div> : null}
        {footer ? <div className="mt-5 flex justify-end gap-2">{footer}</div> : null}
      </div>
    </div>
  );
}

export function MonthSwitcher({
  year,
  month,
  onChange,
}: {
  year: number;
  month: number;
  onChange: (year: number, month: number) => void;
}) {
  const previous = shiftPeriod(year, month, -1);
  const next = shiftPeriod(year, month, 1);

  return (
    <div className="flex items-center gap-1 rounded-xl border border-line bg-panel p-1">
      <button
        type="button"
        aria-label="Previous month"
        className="rounded-lg p-1.5 text-muted hover:bg-canvas hover:text-ink"
        onClick={() => onChange(previous.year, previous.month)}
      >
        <ChevronLeft size={16} />
      </button>
      <div className="flex items-center gap-1.5 px-1">
        <CalendarDays size={14} className="text-muted" />
        <span className="min-w-32 text-center text-sm font-semibold text-ink">
          {formatPeriod(year, month)}
        </span>
      </div>
      <button
        type="button"
        aria-label="Next month"
        className="rounded-lg p-1.5 text-muted hover:bg-canvas hover:text-ink"
        onClick={() => onChange(next.year, next.month)}
      >
        <ChevronRight size={16} />
      </button>
    </div>
  );
}

export const FILTER_LABELS: Array<{ value: ShareFilter; label: string }> = [
  { value: "all", label: "All" },
  { value: "unpaid", label: "Unpaid" },
  { value: "paid", label: "Paid" },
  { value: "change", label: "Change" },
];

export function FilterChips({
  value,
  onChange,
  counts,
}: {
  value: ShareFilter;
  onChange: (value: ShareFilter) => void;
  counts?: Partial<Record<ShareFilter, number>>;
}) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {FILTER_LABELS.map((filter) => (
        <button
          key={filter.value}
          type="button"
          onClick={() => onChange(filter.value)}
          className={cn(
            "rounded-full border px-3 py-1 text-xs font-semibold transition-colors",
            value === filter.value
              ? "border-accent bg-accent text-accent-ink"
              : "border-line bg-panel text-muted hover:text-ink",
          )}
        >
          {filter.label}
          {counts?.[filter.value] !== undefined ? (
            <span className={cn("ml-1.5", value === filter.value ? "text-accent-ink/80" : "text-muted")}>
              {counts[filter.value]}
            </span>
          ) : null}
        </button>
      ))}
    </div>
  );
}
