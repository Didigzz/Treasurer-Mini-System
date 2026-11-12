import { Ionicons } from "@expo/vector-icons";
import { LinearGradient } from "expo-linear-gradient";
import * as Haptics from "expo-haptics";
import { router, Tabs } from "expo-router";
import { Popover } from "heroui-native";
import { type ComponentProps, useEffect, useRef, useState } from "react";
import { Platform, Pressable, StyleSheet, Text, View } from "react-native";
import { useKeyboardState } from "react-native-keyboard-controller";
import Animated, {
  type SharedValue,
  useAnimatedStyle,
  useReducedMotion,
  useSharedValue,
  withSpring,
  withTiming,
} from "react-native-reanimated";
import { useSafeAreaInsets } from "react-native-safe-area-context";

import { useAppTheme } from "@/contexts/app-theme-context";

import { PillTabIcon, type PillTabIconName } from "./pill-tab-icons";
import {
  inactiveLabelColor,
  NAV_COLORS,
  PILL,
  PILL_SPRING,
} from "./pill-theme";

type TabBarProps = Parameters<NonNullable<ComponentProps<typeof Tabs>["tabBar"]>>[0];

const TAB_CONFIG: Record<string, { label: string; icon: PillTabIconName }> = {
  index: { label: "Dashboard", icon: "dashboard" },
  boarders: { label: "Boarders", icon: "boarders" },
  history: { label: "History", icon: "history" },
};

function fireHaptic() {
  Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light).catch(() => {
    // Haptics are unavailable on web and some devices.
  });
}

function PillTabItem({
  label,
  icon,
  index,
  isFocused,
  progress,
  labelColor,
  muted,
  onPress,
  onLongPress,
}: {
  label: string;
  icon: PillTabIconName;
  index: number;
  isFocused: boolean;
  progress: SharedValue<number>;
  labelColor: string;
  muted: boolean;
  onPress: () => void;
  onLongPress: () => void;
}) {
  const iconStyle = useAnimatedStyle(() => {
    const distance = Math.abs(progress.value - index);
    return {
      transform: [{ scale: 1 + 0.06 * Math.max(0, 1 - distance) }],
    };
  });

  return (
    <Pressable
      accessibilityRole="tab"
      accessibilityLabel={label}
      accessibilityState={{ selected: isFocused }}
      onPress={onPress}
      onLongPress={onLongPress}
      className="flex-1 items-center justify-center"
      style={({ pressed }) => ({ opacity: pressed ? 0.85 : 1 })}
    >
      <Animated.View style={iconStyle}>
        <PillTabIcon name={icon} muted={muted} />
      </Animated.View>
      <Text
        numberOfLines={1}
        maxFontSizeMultiplier={1.4}
        style={{
          marginTop: 3,
          fontSize: 11,
          fontWeight: "600",
          color: isFocused ? NAV_COLORS.labelActive : labelColor,
        }}
      >
        {label}
      </Text>
    </Pressable>
  );
}

export function PillTabBar({ state, emitter, navigateToTab }: TabBarProps) {
  const insets = useSafeAreaInsets();
  const { isLight } = useAppTheme();
  const reducedMotion = useReducedMotion();
  const keyboardShown = useKeyboardState((keyboard) => keyboard.isVisible);
  const keyboardVisible = Platform.OS !== "web" && keyboardShown;

  const [menuOpen, setMenuOpen] = useState(false);
  const [pillWidth, setPillWidth] = useState(0);
  const isNavigating = useRef(false);

  const focusedKey = state.routes[state.index]?.key;
  const routes = state.routes.filter((route) => TAB_CONFIG[route.name]);
  const activeIndex = Math.max(
    0,
    routes.findIndex((route) => route.key === focusedKey),
  );

  const progress = useSharedValue(activeIndex);
  const fabProgress = useSharedValue(0);
  const hidden = useSharedValue(0);

  const itemWidth = pillWidth > 0 && routes.length > 0 ? (pillWidth - 8) / routes.length : 0;

  useEffect(() => {
    progress.value = reducedMotion ? activeIndex : withSpring(activeIndex, PILL_SPRING);
  }, [activeIndex, reducedMotion, progress]);

  useEffect(() => {
    fabProgress.value = withTiming(menuOpen ? 1 : 0, {
      duration: reducedMotion ? 0 : 150,
    });
  }, [fabProgress, menuOpen, reducedMotion]);

  useEffect(() => {
    hidden.value = withTiming(keyboardVisible ? 1 : 0, {
      duration: reducedMotion ? 0 : 180,
    });
  }, [hidden, keyboardVisible, reducedMotion]);

  useEffect(() => {
    if (keyboardVisible) setMenuOpen(false);
  }, [keyboardVisible]);

  const groupStyle = useAnimatedStyle(() => ({
    opacity: 1 - hidden.value,
    transform: [{ translateY: hidden.value * (insets.bottom + PILL.height + 48) }],
  }));

  const indicatorStyle = useAnimatedStyle(() => ({
    transform: [{ translateX: progress.value * itemWidth }],
  }));

  const fabScaleStyle = useAnimatedStyle(() => ({
    transform: [{ scale: 1 + fabProgress.value * 0.04 }],
  }));

  const gradientStyle = useAnimatedStyle(() => ({
    opacity: fabProgress.value,
  }));

  function openSettings() {
    if (isNavigating.current) return;
    isNavigating.current = true;
    setMenuOpen(false);
    router.push("/settings");
    setTimeout(() => {
      isNavigating.current = false;
    }, 500);
  }

  return (
    <View
      pointerEvents="box-none"
      style={[styles.host, { bottom: insets.bottom + PILL.bottomOffset }]}
    >
      <Animated.View
        pointerEvents={keyboardVisible ? "none" : "box-none"}
        style={[styles.group, groupStyle]}
      >
        <View
          onLayout={(event) => setPillWidth(event.nativeEvent.layout.width)}
          style={styles.pill}
        >
          {itemWidth > 0 ? (
            <Animated.View
              pointerEvents="none"
              style={[
                styles.indicator,
                { width: itemWidth },
                indicatorStyle,
              ]}
            />
          ) : null}

          {routes.map((route, index) => {
            const config = TAB_CONFIG[route.name]!;
            const isFocused = route.key === focusedKey;

            return (
              <PillTabItem
                key={route.key}
                label={config.label}
                icon={config.icon}
                index={index}
                isFocused={isFocused}
                progress={progress}
                labelColor={inactiveLabelColor(isLight)}
                muted={isLight}
                onPress={() => {
                  if (!isFocused) fireHaptic();
                  setMenuOpen(false);
                  const event = emitter.emit({
                    type: "tabPress",
                    target: route.key,
                    canPreventDefault: true,
                  });
                  if (!isFocused && !event.defaultPrevented) {
                    navigateToTab(route.key);
                  }
                }}
                onLongPress={() => {
                  emitter.emit({ type: "tabLongPress", target: route.key });
                }}
              />
            );
          })}
        </View>

        <Popover
          isOpen={menuOpen}
          onOpenChange={setMenuOpen}
          presentation="popover"
          animation={reducedMotion ? "disable-all" : undefined}
        >
          <Popover.Trigger
            accessibilityRole="button"
            accessibilityLabel="Settings menu"
            accessibilityHint="Opens the Settings option"
            accessibilityState={{ expanded: menuOpen }}
            onPress={fireHaptic}
            style={styles.fab}
          >
            <Animated.View style={[styles.fabCircle, fabScaleStyle]}>
              <View pointerEvents="none" style={styles.fabIdle} />
              <Animated.View
                pointerEvents="none"
                style={[styles.fabGradientClip, gradientStyle]}
              >
                <LinearGradient
                  colors={[NAV_COLORS.gradientStart, NAV_COLORS.gradientEnd]}
                  start={{ x: 0, y: 0 }}
                  end={{ x: 1, y: 1 }}
                  style={StyleSheet.absoluteFill}
                />
              </Animated.View>
              <Ionicons name="add" size={28} color="#FFFFFF" />
            </Animated.View>
          </Popover.Trigger>

          <Popover.Portal>
            <Popover.Overlay style={styles.overlay} />
            <Popover.Content
              presentation="popover"
              placement="top"
              align="end"
              offset={10}
              width={188}
              background={null}
              animation={reducedMotion ? false : undefined}
              style={styles.popover}
            >
              <Popover.Arrow
                width={18}
                height={9}
                fill={NAV_COLORS.popoverBg}
                stroke={NAV_COLORS.popoverBorder}
                strokeWidth={1}
              />
              <Pressable
                accessibilityRole="menuitem"
                accessibilityLabel="Settings"
                accessibilityHint="Opens currency, backup, and other app settings"
                onPress={openSettings}
                style={({ pressed }) => [styles.popoverRow, pressed && styles.popoverRowPressed]}
              >
                <View style={styles.iconTile}>
                  <Ionicons name="settings" size={16} color={NAV_COLORS.gear} />
                </View>
                <Text style={styles.popoverLabel}>Settings</Text>
              </Pressable>
            </Popover.Content>
          </Popover.Portal>
        </Popover>
      </Animated.View>
    </View>
  );
}

const styles = StyleSheet.create({
  host: {
    position: "absolute",
    left: 0,
    right: 0,
    paddingHorizontal: PILL.horizontalMargin,
    alignItems: "center",
    zIndex: 10,
  },
  group: {
    width: "100%",
    maxWidth: PILL.maxGroupWidth,
    flexDirection: "row",
    alignItems: "center",
  },
  pill: {
    flex: 1,
    maxWidth: PILL.maxWidth,
    height: PILL.height,
    flexDirection: "row",
    alignItems: "stretch",
    padding: 4,
    borderRadius: PILL.radius,
    borderWidth: 1,
    borderColor: NAV_COLORS.pillBorder,
    backgroundColor: NAV_COLORS.pillBg,
    shadowColor: "#000000",
    shadowOpacity: 0.35,
    shadowRadius: 18,
    shadowOffset: { width: 0, height: 8 },
    elevation: 12,
  },
  indicator: {
    position: "absolute",
    top: 4,
    bottom: 4,
    left: 4,
    borderRadius: 26,
    borderWidth: 1,
    borderColor: NAV_COLORS.activeCapsuleBorder,
    backgroundColor: NAV_COLORS.activeCapsule,
  },
  fab: {
    width: PILL.fabSize,
    height: PILL.fabSize,
    marginLeft: PILL.gap,
    alignItems: "center",
    justifyContent: "center",
  },
  fabCircle: {
    width: PILL.fabSize,
    height: PILL.fabSize,
    borderRadius: PILL.fabSize / 2,
    alignItems: "center",
    justifyContent: "center",
    shadowColor: "#000000",
    shadowOpacity: 0.35,
    shadowRadius: 18,
    shadowOffset: { width: 0, height: 8 },
    elevation: 12,
  },
  fabIdle: {
    position: "absolute",
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
    borderRadius: PILL.fabSize / 2,
    borderWidth: 1,
    borderColor: NAV_COLORS.fabBorder,
    backgroundColor: NAV_COLORS.fabIdle,
  },
  fabGradientClip: {
    position: "absolute",
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
    borderRadius: PILL.fabSize / 2,
    overflow: "hidden",
  },
  overlay: {
    position: "absolute",
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
    backgroundColor: "transparent",
  },
  popover: {
    borderRadius: 18,
    borderWidth: 1,
    borderColor: NAV_COLORS.popoverBorder,
    backgroundColor: NAV_COLORS.popoverBg,
    padding: 4,
    shadowColor: "#000000",
    shadowOpacity: 0.4,
    shadowRadius: 20,
    shadowOffset: { width: 0, height: 8 },
    elevation: 16,
  },
  popoverRow: {
    minHeight: 52,
    flexDirection: "row",
    alignItems: "center",
    gap: 10,
    paddingHorizontal: 12,
    borderRadius: 12,
  },
  popoverRowPressed: {
    backgroundColor: NAV_COLORS.rowPressed,
  },
  iconTile: {
    width: 28,
    height: 28,
    borderRadius: 8,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: NAV_COLORS.iconTile,
  },
  popoverLabel: {
    fontSize: 15,
    fontWeight: "600",
    color: "#FFFFFF",
  },
});
