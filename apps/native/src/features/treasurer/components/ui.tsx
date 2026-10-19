import { Ionicons } from "@expo/vector-icons";
import { Button, Chip, Dialog, Input, TextField, cn, useThemeColor } from "heroui-native";
import { type PropsWithChildren, type ReactNode, useState } from "react";
import {
  ActivityIndicator,
  Pressable,
  ScrollView,
  Text,
  View,
  type TextInputProps,
} from "react-native";

import { Container } from "@/components/container";

import { formatMoney, formatPeriod, shiftPeriod } from "../money";
import type { BillStatus, ShareFilter, ShareStatus } from "../types";

export function Screen({
  children,
  className,
  scrollable = true,
}: PropsWithChildren<{ className?: string; scrollable?: boolean }>) {
  return (
    <Container className={cn("px-4 pb-8", className)} isScrollable={scrollable}>
      {children}
    </Container>
  );
}

export function SectionHeader({
  title,
  action,
  subtitle,
}: {
  title: string;
  subtitle?: string;
  action?: ReactNode;
}) {
  return (
    <View className="mb-3 mt-6 flex-row items-end justify-between gap-3">
      <View className="flex-1">
        <Text className="text-base font-semibold text-foreground">{title}</Text>
        {subtitle ? <Text className="mt-0.5 text-xs text-muted">{subtitle}</Text> : null}
      </View>
      {action}
    </View>
  );
}

export function MoneyText({
  minor,
  currency,
  className,
  size = "md",
}: {
  minor: number;
  currency: string;
  className?: string;
  size?: "sm" | "md" | "lg" | "xl";
}) {
  const sizeClass =
    size === "sm"
      ? "text-sm"
      : size === "lg"
        ? "text-xl"
        : size === "xl"
          ? "text-3xl"
          : "text-base";

  return (
    <Text
      className={cn(sizeClass, "font-semibold text-foreground", className)}
      style={{ fontVariant: ["tabular-nums"] }}
    >
      {formatMoney(minor, currency)}
    </Text>
  );
}

type BadgeTone = "success" | "warning" | "danger" | "accent" | "default";

export function StatusBadge({
  label,
  tone = "default",
  icon,
  className,
}: {
  label: string;
  tone?: BadgeTone;
  icon?: keyof typeof Ionicons.glyphMap;
  className?: string;
}) {
  const iconColor = useThemeColor(
    tone === "success"
      ? "success"
      : tone === "warning"
        ? "warning"
        : tone === "danger"
          ? "danger"
          : tone === "accent"
            ? "accent"
            : "muted",
  );

  return (
    <Chip color={tone === "default" ? "default" : tone} size="sm" className={className}>
      {icon ? <Ionicons name={icon} size={12} color={iconColor} /> : null}
      <Chip.Label>{label}</Chip.Label>
    </Chip>
  );
}

export function BillStatusBadge({ status }: { status: BillStatus }) {
  if (status === "settled") {
    return <StatusBadge label="Settled" tone="success" icon="checkmark-done" />;
  }
  if (status === "archived") {
    return <StatusBadge label="Archived" tone="default" icon="archive-outline" />;
  }
  return <StatusBadge label="Open" tone="accent" icon="time-outline" />;
}

export function ShareStatusBadge({ status }: { status: ShareStatus }) {
  if (status === "paid") {
    return <StatusBadge label="Paid" tone="success" icon="checkmark-circle" />;
  }
  if (status === "partial") {
    return <StatusBadge label="Partial" tone="warning" icon="contrast-outline" />;
  }
  return <StatusBadge label="Unpaid" tone="danger" icon="alert-circle-outline" />;
}

export function ChangeBadge({ minor, currency }: { minor: number; currency: string }) {
  return (
    <StatusBadge
      label={`${formatMoney(minor, currency)} change`}
      tone="warning"
      icon="cash-outline"
    />
  );
}

const FILTER_OPTIONS: Array<{ key: ShareFilter; label: string }> = [
  { key: "all", label: "All" },
  { key: "unpaid", label: "Unpaid" },
  { key: "paid", label: "Paid" },
  { key: "change", label: "Change due" },
];

export function FilterChips({
  value,
  onChange,
  counts,
}: {
  value: ShareFilter;
  onChange: (value: ShareFilter) => void;
  counts: Partial<Record<ShareFilter, number>>;
}) {
  return (
    <ScrollView
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={{ gap: 8, paddingVertical: 2 }}
      className="mt-3"
    >
      {FILTER_OPTIONS.map((option) => {
        const selected = option.key === value;
        const count = counts[option.key];
        return (
          <Pressable
            key={option.key}
            accessibilityRole="button"
            accessibilityState={{ selected }}
            accessibilityLabel={`${option.label} filter${count !== undefined ? `, ${count} boarders` : ""}`}
            onPress={() => onChange(option.key)}
            className={cn(
              "min-h-11 flex-row items-center rounded-full border px-4",
              selected
                ? "border-accent bg-accent"
                : "border-separator bg-surface",
            )}
          >
            <Text
              className={cn(
                "text-sm font-medium",
                selected ? "text-accent-foreground" : "text-foreground",
              )}
            >
              {option.label}
              {count !== undefined ? ` ${count}` : ""}
            </Text>
          </Pressable>
        );
      })}
    </ScrollView>
  );
}

export function EmptyState({
  icon = "file-tray-outline",
  title,
  description,
  actionLabel,
  onAction,
}: {
  icon?: keyof typeof Ionicons.glyphMap;
  title: string;
  description?: string;
  actionLabel?: string;
  onAction?: () => void;
}) {
  const muted = useThemeColor("muted");
  return (
    <View className="items-center rounded-2xl border border-dashed border-separator px-6 py-8">
      <Ionicons name={icon} size={30} color={muted} />
      <Text className="mt-3 text-center text-base font-medium text-foreground">{title}</Text>
      {description ? (
        <Text className="mt-1 text-center text-sm text-muted">{description}</Text>
      ) : null}
      {actionLabel && onAction ? (
        <Button size="sm" className="mt-4" onPress={onAction}>
          <Button.Label>{actionLabel}</Button.Label>
        </Button>
      ) : null}
    </View>
  );
}

export function FormField({
  label,
  error,
  hint,
  children,
  required,
}: PropsWithChildren<{
  label: string;
  error?: string;
  hint?: string;
  required?: boolean;
}>) {
  return (
    <TextField isInvalid={Boolean(error)} className="mb-4">
      <Text className="mb-1.5 text-sm font-medium text-foreground">
        {label}
        {required ? <Text className="text-danger"> *</Text> : null}
      </Text>
      {children}
      {error ? (
        <Text className="mt-1.5 text-xs text-danger">{error}</Text>
      ) : hint ? (
        <Text className="mt-1.5 text-xs text-muted">{hint}</Text>
      ) : null}
    </TextField>
  );
}

export function AppInput({ className, ...props }: TextInputProps) {
  return <Input className={cn("min-h-12", className)} {...props} />;
}

export function AppDialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  actions,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  children?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <Dialog isOpen={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay />
        <Dialog.Content>
          <Dialog.Close />
          <View className="mb-4 gap-1.5">
            <Dialog.Title>{title}</Dialog.Title>
            {description ? <Dialog.Description>{description}</Dialog.Description> : null}
          </View>
          {children}
          {actions ? <View className="mt-2 flex-row justify-end gap-2">{actions}</View> : null}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog>
  );
}

export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  destructive = false,
  pending = false,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
  pending?: boolean;
  onConfirm: () => void;
}) {
  return (
    <AppDialog
      open={open}
      onOpenChange={onOpenChange}
      title={title}
      description={description}
      actions={
        <>
          <Button variant="ghost" size="sm" onPress={() => onOpenChange(false)}>
            <Button.Label>{cancelLabel}</Button.Label>
          </Button>
          <Button
            variant={destructive ? "danger" : "primary"}
            size="sm"
            isDisabled={pending}
            onPress={onConfirm}
          >
            {pending ? <ActivityIndicator size="small" /> : null}
            <Button.Label>{confirmLabel}</Button.Label>
          </Button>
        </>
      }
    />
  );
}

const MONTHS = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

export function MonthSwitcher({
  year,
  month,
  onChange,
}: {
  year: number;
  month: number;
  onChange: (year: number, month: number) => void;
}) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <View className="flex-row items-center gap-1">
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Previous month"
          className="h-11 w-11 items-center justify-center rounded-full"
          onPress={() => {
            const next = shiftPeriod(year, month, -1);
            onChange(next.year, next.month);
          }}
        >
          <Ionicons name="chevron-back" size={22} className="text-foreground" />
        </Pressable>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`Selected period ${formatPeriod(year, month)}. Change month.`}
          className="min-h-11 flex-row items-center gap-1.5 rounded-full px-2"
          onPress={() => setOpen(true)}
        >
          <Text className="text-base font-semibold text-foreground">
            {formatPeriod(year, month)}
          </Text>
          <Ionicons name="chevron-down" size={15} className="text-muted" />
        </Pressable>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Next month"
          className="h-11 w-11 items-center justify-center rounded-full"
          onPress={() => {
            const next = shiftPeriod(year, month, 1);
            onChange(next.year, next.month);
          }}
        >
          <Ionicons name="chevron-forward" size={22} className="text-foreground" />
        </Pressable>
      </View>

      <AppDialog
        open={open}
        onOpenChange={setOpen}
        title="Choose billing period"
        description="Bills are grouped by calendar month."
      >
        <View className="mb-4 flex-row items-center justify-between rounded-xl bg-surface px-3 py-2">
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Previous year"
            className="h-10 w-10 items-center justify-center"
            onPress={() => onChange(year - 1, month)}
          >
            <Ionicons name="chevron-back" size={20} className="text-foreground" />
          </Pressable>
          <Text className="text-lg font-semibold text-foreground">{year}</Text>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Next year"
            className="h-10 w-10 items-center justify-center"
            onPress={() => onChange(year + 1, month)}
          >
            <Ionicons name="chevron-forward" size={20} className="text-foreground" />
          </Pressable>
        </View>
        <View className="flex-row flex-wrap gap-2">
          {MONTHS.map((label, index) => {
            const selected = month === index + 1;
            return (
              <Pressable
                key={label}
                accessibilityRole="button"
                accessibilityState={{ selected }}
                className={cn(
                  "min-h-11 flex-1 basis-[28%] items-center justify-center rounded-xl border",
                  selected ? "border-accent bg-accent" : "border-separator bg-surface",
                )}
                onPress={() => {
                  onChange(year, index + 1);
                  setOpen(false);
                }}
              >
                <Text
                  className={cn(
                    "text-sm font-medium",
                    selected ? "text-accent-foreground" : "text-foreground",
                  )}
                >
                  {label}
                </Text>
              </Pressable>
            );
          })}
        </View>
      </AppDialog>
    </>
  );
}

export function SummaryCard({
  label,
  value,
  tone = "default",
  icon,
}: {
  label: string;
  value: string;
  tone?: "default" | "success" | "danger" | "warning";
  icon?: keyof typeof Ionicons.glyphMap;
}) {
  const color = useThemeColor(
    tone === "success"
      ? "success"
      : tone === "danger"
        ? "danger"
        : tone === "warning"
          ? "warning"
          : "muted",
  );

  return (
    <View className="flex-1 rounded-2xl border border-separator bg-surface p-3">
      <View className="flex-row items-center gap-1.5">
        {icon ? <Ionicons name={icon} size={13} color={color} /> : null}
        <Text className="text-[11px] font-medium uppercase tracking-wide text-muted">{label}</Text>
      </View>
      <Text
        className={cn(
          "mt-1.5 text-base font-semibold",
          tone === "success"
            ? "text-success"
            : tone === "danger"
              ? "text-danger"
              : tone === "warning"
                ? "text-warning"
                : "text-foreground",
        )}
        style={{ fontVariant: ["tabular-nums"] }}
      >
        {value}
      </Text>
    </View>
  );
}

export function LoadingBlock({ label = "Loading…" }: { label?: string }) {
  return (
    <View className="items-center justify-center py-12">
      <ActivityIndicator />
      <Text className="mt-3 text-sm text-muted">{label}</Text>
    </View>
  );
}

export function ErrorBlock({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <View className="rounded-2xl border border-danger/40 bg-danger/10 p-4">
      <Text className="text-sm font-medium text-danger">{message}</Text>
      {onRetry ? (
        <Button size="sm" variant="danger-soft" className="mt-3 self-start" onPress={onRetry}>
          <Button.Label>Retry</Button.Label>
        </Button>
      ) : null}
    </View>
  );
}
