import "@/global.css";

import { Ionicons } from "@expo/vector-icons";
import { QueryClientProvider } from "@tanstack/react-query";
import { useFonts } from "expo-font";
import { Stack } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { HeroUINativeProvider, useThemeColor } from "heroui-native";
import { GestureHandlerRootView } from "react-native-gesture-handler";
import { useEffect, useState } from "react";
import { KeyboardProvider } from "react-native-keyboard-controller";

import { AppThemeProvider, useAppTheme } from "@/contexts/app-theme-context";
import { DatabaseBootstrap } from "@/src/db/bootstrap";
import { queryClient } from "@/src/query-client";

export const unstable_settings = {
  anchor: "(tabs)",
};

function RootNavigator() {
  const background = useThemeColor("background");
  const foreground = useThemeColor("foreground");
  const { isDark } = useAppTheme();

  return (
    <>
      {/* The app theme can differ from the OS theme, so the bar cannot rely on "auto". */}
      <StatusBar style={isDark ? "light" : "dark"} />
      <Stack
        screenOptions={{
          headerStyle: { backgroundColor: background },
          headerTintColor: foreground,
          headerTitleStyle: { color: foreground, fontWeight: "600" },
          contentStyle: { backgroundColor: background },
        }}
      >
        <Stack.Screen name="(tabs)" options={{ headerShown: false }} />
        <Stack.Screen name="bills/new" options={{ title: "Bill" }} />
        <Stack.Screen name="bills/[id]" options={{ title: "Bill details" }} />
        <Stack.Screen name="history/[id]" options={{ title: "Bill history" }} />
        <Stack.Screen name="boarders/[id]" options={{ title: "Boarder" }} />
        <Stack.Screen name="bill-types" options={{ title: "Bill types" }} />
        <Stack.Screen name="settings" options={{ title: "Settings" }} />
        <Stack.Screen
          name="payments/new"
          options={{ title: "Record payment", presentation: "modal" }}
        />
      </Stack>
    </>
  );
}

export default function Layout() {
  const [iconsLoaded, iconsError] = useFonts(Ionicons.font);
  const [fontWaitExpired, setFontWaitExpired] = useState(false);

  useEffect(() => {
    if (iconsError) {
      console.warn("[icons] Ionicons font failed to load:", iconsError.message);
    }
  }, [iconsError]);

  // Never block the app on font loading for longer than a moment.
  useEffect(() => {
    const timeout = setTimeout(() => setFontWaitExpired(true), 3000);
    return () => clearTimeout(timeout);
  }, []);

  // Icons must not mount before the font is ready: @expo/vector-icons renders an
  // empty Text on first mount and would otherwise stay blank.
  if (!iconsLoaded && !iconsError && !fontWaitExpired) return null;

  return (
    <QueryClientProvider client={queryClient}>
      <GestureHandlerRootView style={{ flex: 1 }}>
        <KeyboardProvider>
          <AppThemeProvider>
            <HeroUINativeProvider>
              <DatabaseBootstrap>
                <RootNavigator />
              </DatabaseBootstrap>
            </HeroUINativeProvider>
          </AppThemeProvider>
        </KeyboardProvider>
      </GestureHandlerRootView>
    </QueryClientProvider>
  );
}
