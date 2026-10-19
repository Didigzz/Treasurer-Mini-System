import { migrate } from "drizzle-orm/expo-sqlite/migrator";
import { useCallback, useEffect, useRef, useState, type PropsWithChildren } from "react";
import { ActivityIndicator, Pressable, Text, View } from "react-native";

import { db, initializeDatabase } from "./index";
import { migrations } from "./migrations";
import { seedDatabase } from "./seed";

type BootstrapState =
  | { status: "loading" }
  | { status: "ready" }
  | { status: "error"; message: string };

export function DatabaseBootstrap({ children }: PropsWithChildren) {
  const [state, setState] = useState<BootstrapState>({ status: "loading" });
  const started = useRef(false);

  const initialize = useCallback(async () => {
    setState({ status: "loading" });

    try {
      await initializeDatabase();
      await migrate(db, migrations);
      await seedDatabase();
      setState({ status: "ready" });
    } catch (error) {
      const message = error instanceof Error ? error.message : "The local database could not be opened.";
      setState({ status: "error", message });
    }
  }, []);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    void initialize();
  }, [initialize]);

  if (state.status === "ready") {
    return <>{children}</>;
  }

  return (
    <View className="flex-1 items-center justify-center bg-background px-8">
      {state.status === "loading" ? (
        <>
          <ActivityIndicator size="large" />
          <Text className="mt-4 text-base font-medium text-foreground">Opening your records…</Text>
          <Text className="mt-2 text-center text-sm text-muted">
            Everything is stored only on this phone.
          </Text>
        </>
      ) : (
        <>
          <Text className="text-center text-lg font-semibold text-foreground">
            Your records could not be opened
          </Text>
          <Text className="mt-2 text-center text-sm text-muted">{state.message}</Text>
          <Pressable
            accessibilityRole="button"
            className="mt-6 rounded-xl bg-accent px-6 py-3"
            onPress={() => void initialize()}
          >
            <Text className="font-semibold text-accent-foreground">Try again</Text>
          </Pressable>
        </>
      )}
    </View>
  );
}
