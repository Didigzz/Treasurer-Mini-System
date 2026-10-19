import { Ionicons } from "@expo/vector-icons";
import { Tabs } from "expo-router";
import { useThemeColor } from "heroui-native";

export default function TabLayout() {
  const foreground = useThemeColor("foreground");
  const background = useThemeColor("background");
  const accent = useThemeColor("accent");

  return (
    <Tabs
      screenOptions={{
        headerShown: true,
        headerStyle: { backgroundColor: background },
        headerTintColor: foreground,
        headerTitleStyle: { color: foreground, fontWeight: "600" },
        tabBarStyle: { backgroundColor: background },
        tabBarActiveTintColor: accent,
        tabBarInactiveTintColor: foreground,
        tabBarLabelStyle: { fontSize: 11, fontWeight: "600" },
        tabBarIconStyle: { marginTop: 2 },
        sceneStyle: { backgroundColor: background },
      }}
    >
      <Tabs.Screen
        name="index"
        options={{
          title: "Dashboard",
          tabBarLabel: "Dashboard",
          tabBarIcon: ({ color, size, focused }) => (
            <Ionicons name={focused ? "home" : "home-outline"} size={size} color={color} />
          ),
        }}
      />
      <Tabs.Screen
        name="boarders"
        options={{
          title: "Boarders",
          tabBarLabel: "Boarders",
          tabBarIcon: ({ color, size, focused }) => (
            <Ionicons name={focused ? "people" : "people-outline"} size={size} color={color} />
          ),
        }}
      />
      <Tabs.Screen
        name="settings"
        options={{
          title: "Settings",
          tabBarLabel: "Settings",
          tabBarIcon: ({ color, size, focused }) => (
            <Ionicons
              name={focused ? "settings" : "settings-outline"}
              size={size}
              color={color}
            />
          ),
        }}
      />
    </Tabs>
  );
}
