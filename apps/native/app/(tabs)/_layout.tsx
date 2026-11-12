import { Tabs } from "expo-router";
import { useThemeColor } from "heroui-native";

import { PillTabBar } from "@/components/navigation/pill-tab-bar";

export default function TabLayout() {
  const background = useThemeColor("background");

  return (
    <Tabs
      tabBar={(props) => <PillTabBar {...props} />}
      screenOptions={{
        headerShown: false,
        sceneStyle: { backgroundColor: background },
      }}
    >
      <Tabs.Screen name="index" options={{ title: "Dashboard" }} />
      <Tabs.Screen name="boarders" options={{ title: "Boarders" }} />
      <Tabs.Screen name="history" options={{ title: "History" }} />
    </Tabs>
  );
}
