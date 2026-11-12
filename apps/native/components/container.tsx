import { cn } from "heroui-native";
import { type PropsWithChildren } from "react";
import { ScrollView, View, type ScrollViewProps, type ViewProps } from "react-native";
import Animated, { type AnimatedProps } from "react-native-reanimated";
import { useSafeAreaInsets } from "react-native-safe-area-context";

const AnimatedView = Animated.createAnimatedComponent(View);

type Props = AnimatedProps<ViewProps> & {
  className?: string;
  isScrollable?: boolean;
  scrollViewProps?: Omit<ScrollViewProps, "contentContainerStyle">;
  contentPaddingTop?: number;
  contentPaddingBottom?: number;
};

export function Container({
  children,
  className,
  isScrollable = true,
  scrollViewProps,
  contentPaddingTop = 0,
  contentPaddingBottom = 0,
  ...props
}: PropsWithChildren<Props>) {
  const insets = useSafeAreaInsets();

  return (
    <AnimatedView className={cn("flex-1 bg-background", className)} {...props}>
      {isScrollable ? (
        <ScrollView
          contentContainerStyle={{
            flexGrow: 1,
            paddingTop: contentPaddingTop,
            paddingBottom: insets.bottom + 24 + contentPaddingBottom,
          }}
          keyboardShouldPersistTaps="handled"
          keyboardDismissMode="on-drag"
          // Screens without a navigation header set an explicit top inset, so
          // automatic adjustment would double the safe-area padding on iOS.
          contentInsetAdjustmentBehavior={contentPaddingTop > 0 ? "never" : "automatic"}
          showsVerticalScrollIndicator={false}
          showsHorizontalScrollIndicator={false}
          {...scrollViewProps}
        >
          {children}
        </ScrollView>
      ) : (
        <View
          className="flex-1"
          style={{
            paddingTop: contentPaddingTop,
            paddingBottom: insets.bottom + contentPaddingBottom,
          }}
        >
          {children}
        </View>
      )}
    </AnimatedView>
  );
}
