import { ReactNode } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, Icon, LinearFill } from '../ds';

/** Purple full-screen backdrop with safe-area padding and an optional back button. */
export function FlowFrame({
  children,
  onBack,
}: {
  children: ReactNode;
  onBack?: () => void;
}) {
  const insets = useSafeAreaInsets();
  return (
    <View style={styles.root}>
      <LinearFill
        stops={[
          { offset: 0, color: colors.purple350 },
          { offset: 0.55, color: colors.purple450 },
          { offset: 1, color: colors.purple550 },
        ]}
      />
      <View
        style={[
          styles.content,
          { paddingTop: insets.top + 16, paddingBottom: insets.bottom + 24 },
        ]}
      >
        {onBack ? (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Back"
            onPress={onBack}
            hitSlop={12}
            style={styles.back}
          >
            <Icon name="back" size={24} />
          </Pressable>
        ) : null}
        {children}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple500 },
  content: { flex: 1, paddingHorizontal: 24 },
  back: { width: 40, height: 40, justifyContent: 'center' },
});
