import { ReactNode } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, Icon, LinearFill } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';

/** Magenta-to-purple full-screen backdrop (flat purple when `flat`) with safe-area padding and an optional back button. */
export function FlowFrame({
  children,
  onBack,
  flat = false,
}: {
  children: ReactNode;
  onBack?: () => void;
  flat?: boolean;
}) {
  const insets = useSafeAreaInsets();
  return (
    <View style={styles.root}>
      {flat ? null : (
        <LinearFill
          stops={[
            { offset: 0, color: colors.magenta },
            { offset: 1, color: colors.purple350 },
          ]}
        />
      )}
      <View
        style={[
          styles.content,
          {
            paddingTop: insets.top + u(16),
            paddingBottom: insets.bottom + u(24),
          },
        ]}
      >
        {onBack ? (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('APP_BACK')}
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
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple350 },
  content: { flex: 1, paddingHorizontal: u(24) },
  back: { width: u(40), height: u(40), justifyContent: 'center' },
});
