import { Surface, cn } from "heroui-native";
import { type PropsWithChildren, useEffect } from "react";
import { View } from "react-native";
import Animated, {
  Easing,
  makeMutable,
  useAnimatedStyle,
  useReducedMotion,
  withRepeat,
  withTiming,
} from "react-native-reanimated";

// One worklet loop drives every placeholder on screen, so a skeleton-heavy
// screen costs a single animation instead of one per block.
const pulse = makeMutable(0.55);
let pulseRunning = false;

function startPulse() {
  if (pulseRunning) return;
  pulseRunning = true;
  pulse.set(
    withRepeat(withTiming(1, { duration: 700, easing: Easing.inOut(Easing.quad) }), -1, true),
  );
}

function Skeleton({ className }: { className?: string }) {
  const reduceMotion = useReducedMotion();

  useEffect(() => {
    if (!reduceMotion) startPulse();
  }, [reduceMotion]);

  const animatedStyle = useAnimatedStyle(() => ({
    opacity: reduceMotion ? 0.4 : pulse.get(),
  }));

  return (
    <Animated.View
      className={cn("rounded-lg bg-muted/25", className)}
      style={animatedStyle}
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
    />
  );
}

// Groups a set of placeholders into one announced element instead of letting
// assistive tech walk every decorative block.
function LoadingGroup({ children, className }: PropsWithChildren<{ className?: string }>) {
  return (
    <View
      accessible
      accessibilityRole="progressbar"
      accessibilityLabel="Loading"
      className={className}
    >
      {children}
    </View>
  );
}

function SummaryCardBlock() {
  return (
    <View className="flex-1 rounded-2xl border border-separator bg-surface p-3">
      <Skeleton className="h-2.5 w-16" />
      <Skeleton className="mt-2 h-5 w-20" />
    </View>
  );
}

/** Four summary cards, laid out like the 2×2 totals grid on details and lists. */
export function SummaryCardsSkeleton() {
  return (
    <LoadingGroup className="gap-2">
      <View className="flex-row gap-2">
        <SummaryCardBlock />
        <SummaryCardBlock />
      </View>
      <View className="flex-row gap-2">
        <SummaryCardBlock />
        <SummaryCardBlock />
      </View>
    </LoadingGroup>
  );
}

function RowCardBlock() {
  return (
    <Surface variant="secondary" className="mb-2 rounded-2xl p-3.5">
      <View className="flex-row items-center justify-between gap-3">
        <View className="flex-1">
          <Skeleton className="h-4 w-32" />
          <Skeleton className="mt-2 h-3 w-44" />
        </View>
        <View className="items-end gap-2">
          <Skeleton className="h-5 w-16 rounded-full" />
          <Skeleton className="h-3 w-12" />
        </View>
      </View>
    </Surface>
  );
}

/** Boarder and bill-type rows: title, one meta line, status on the right. */
export function CardListSkeleton({ rows = 3 }: { rows?: number }) {
  return (
    <LoadingGroup>
      {Array.from({ length: rows }, (_, index) => (
        <RowCardBlock key={index} />
      ))}
    </LoadingGroup>
  );
}

/** The taller dashboard bill card: header, payment line, progress, action. */
export function BillCardSkeleton() {
  return (
    <LoadingGroup className="mb-3">
      <Surface variant="secondary" className="rounded-2xl p-4">
        <View className="flex-row items-start justify-between gap-3">
          <View className="flex-1">
            <Skeleton className="h-4 w-28" />
            <Skeleton className="mt-2 h-3 w-20" />
          </View>
          <View className="items-end gap-2">
            <Skeleton className="h-5 w-14 rounded-full" />
            <Skeleton className="h-4 w-20" />
          </View>
        </View>
        <View className="mt-3 flex-row gap-2">
          <Skeleton className="h-3 w-24" />
          <Skeleton className="h-3 w-16" />
        </View>
        <Skeleton className="mt-3 h-2 rounded-full" />
        <View className="mt-3 flex-row justify-end">
          <Skeleton className="h-3 w-16" />
        </View>
      </Surface>
    </LoadingGroup>
  );
}

/** Two filter controls sitting side by side above a list. */
export function FilterBarSkeleton() {
  return (
    <LoadingGroup className="flex-row gap-2">
      <View className="flex-1">
        <Skeleton className="h-11 rounded-xl" />
      </View>
      <View className="flex-1">
        <Skeleton className="h-11 rounded-xl" />
      </View>
    </LoadingGroup>
  );
}

/** Share rows on a bill: name, due/paid line, status, payment action. */
export function ShareRowsSkeleton({ rows = 3 }: { rows?: number }) {
  return (
    <LoadingGroup>
      {Array.from({ length: rows }, (_, index) => (
        <Surface key={index} variant="secondary" className="mb-2 rounded-2xl p-3.5">
          <View className="flex-row items-start justify-between gap-3">
            <View className="flex-1">
              <Skeleton className="h-4 w-28" />
              <View className="mt-2 flex-row gap-4">
                <Skeleton className="h-3 w-16" />
                <Skeleton className="h-3 w-16" />
              </View>
            </View>
            <Skeleton className="h-5 w-16 rounded-full" />
          </View>
          <View className="mt-3 flex-row items-center justify-between gap-2">
            <Skeleton className="h-3 w-20" />
            <Skeleton className="h-8 w-28 rounded-full" />
          </View>
        </Surface>
      ))}
    </LoadingGroup>
  );
}

/** The payment log card: divided rows of amount and timestamp. */
export function PaymentListSkeleton({ rows = 2 }: { rows?: number }) {
  return (
    <LoadingGroup>
      <Surface variant="secondary" className="rounded-2xl px-4">
        {Array.from({ length: rows }, (_, index) => (
          <View
            key={index}
            className={cn("py-3", index > 0 && "border-t border-separator")}
          >
            <View className="flex-row items-start justify-between gap-3">
              <View className="flex-1">
                <Skeleton className="h-4 w-24" />
                <Skeleton className="mt-1.5 h-3 w-40" />
              </View>
              <Skeleton className="h-4 w-16" />
            </View>
          </View>
        ))}
      </Surface>
    </LoadingGroup>
  );
}

/** Large title plus one subtitle line, matching a detail screen's header. */
export function DetailHeaderSkeleton() {
  return (
    <LoadingGroup className="pt-4">
      <View className="flex-row items-start justify-between gap-3">
        <View className="flex-1">
          <Skeleton className="h-7 w-40" />
          <Skeleton className="mt-2 h-4 w-28" />
        </View>
        <Skeleton className="h-7 w-16 rounded-full" />
      </View>
      <View className="mt-3 flex-row gap-2">
        <Skeleton className="h-4 w-24" />
        <Skeleton className="h-4 w-20" />
      </View>
    </LoadingGroup>
  );
}

/** Screen title and subtitle for form screens. */
export function TitleSkeleton() {
  return (
    <LoadingGroup className="pt-4">
      <Skeleton className="h-7 w-44" />
      <Skeleton className="mt-2 h-4 w-56" />
    </LoadingGroup>
  );
}

/** Stacked labelled fields, each a 48pt input, plus the submit button. */
export function FormSkeleton({ fields = 3 }: { fields?: number }) {
  return (
    <LoadingGroup className="mt-4">
      {Array.from({ length: fields }, (_, index) => (
        <View key={index} className="mb-4">
          <Skeleton className="h-3 w-24" />
          <Skeleton className="mt-2 h-12 rounded-xl" />
        </View>
      ))}
      <Skeleton className="mt-2 h-12 rounded-xl" />
    </LoadingGroup>
  );
}

/** The selected-type chip strip above the bill form. */
export function ChipRowSkeleton() {
  return (
    <LoadingGroup className="flex-row gap-2">
      <Skeleton className="h-11 w-20 rounded-full" />
      <Skeleton className="h-11 w-24 rounded-full" />
      <Skeleton className="h-11 w-16 rounded-full" />
    </LoadingGroup>
  );
}

export { Skeleton };
