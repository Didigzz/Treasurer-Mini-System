import "@/global.css";

import { QueryClientProvider } from "@tanstack/react-query";
import { Stack } from "expo-router";
import { HeroUINativeProvider } from "heroui-native";
import { GestureHandlerRootView } from "react-native-gesture-handler";
import { KeyboardProvider } from "react-native-keyboard-controller";

import { AppThemeProvider } from "@/contexts/app-theme-context";
import { DatabaseBootstrap } from "@/src/db/bootstrap";
import { queryClient } from "@/src/query-client";

export const unstable_settings = {
  anchor: "(tabs)",
};

export default function Layout() {
  return (
    <QueryClientProvider client={queryClient}>
      <GestureHandlerRootView style={{ flex: 1 }}>
        <KeyboardProvider>
          <AppThemeProvider>
            <HeroUINativeProvider>
              <DatabaseBootstrap>
                <Stack screenOptions={{}}>
                  <Stack.Screen name="(tabs)" options={{ headerShown: false }} />
                  <Stack.Screen name="bills/new" options={{ title: "Bill" }} />
                  <Stack.Screen name="bills/[id]" options={{ title: "Bill details" }} />
                  <Stack.Screen name="boarders/[id]" options={{ title: "Boarder" }} />
                  <Stack.Screen name="bill-types" options={{ title: "Bill types" }} />
                  <Stack.Screen
                    name="payments/new"
                    options={{ title: "Record payment", presentation: "modal" }}
                  />
                </Stack>
              </DatabaseBootstrap>
            </HeroUINativeProvider>
          </AppThemeProvider>
        </KeyboardProvider>
      </GestureHandlerRootView>
    </QueryClientProvider>
  );
}
