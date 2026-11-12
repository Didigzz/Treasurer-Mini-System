import { Ionicons } from "@expo/vector-icons";
import { Link, Stack } from "expo-router";
import { Button, Surface, useThemeColor } from "heroui-native";
import { Text, View } from "react-native";

import { Container } from "@/components/container";

export default function NotFoundScreen() {
  const accent = useThemeColor("accent");

  return (
    <>
      <Stack.Screen options={{ title: "Not found" }} />
      <Container>
        <View className="flex-1 items-center justify-center p-4">
          <Surface variant="secondary" className="w-full max-w-sm items-center rounded-2xl p-6">
            <View className="h-14 w-14 items-center justify-center rounded-full bg-accent/10">
              <Ionicons name="compass-outline" size={26} color={accent} />
            </View>
            <Text className="mt-4 text-center text-lg font-semibold text-foreground">
              This screen doesn’t exist
            </Text>
            <Text className="mt-1 text-center text-sm text-muted">
              The link may be out of date. Head back to the dashboard to keep going.
            </Text>
            <Link href="/" asChild>
              <Button size="sm" className="mt-5">
                <Button.Label>Go to dashboard</Button.Label>
              </Button>
            </Link>
          </Surface>
        </View>
      </Container>
    </>
  );
}
