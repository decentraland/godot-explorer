import { Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';

/** CANCEL / SAVE bar that replaces the bottom navigation while editing. */
export function EditFooter({
  canSave,
  bottomInset,
  onCancel,
  onSave,
}: {
  canSave: boolean;
  bottomInset: number;
  onCancel: () => void;
  onSave: () => void;
}) {
  return (
    <View style={[styles.bar, { paddingBottom: Math.max(bottomInset, u(20)) }]}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('COMMON_CANCEL')}
        onPress={onCancel}
        style={[styles.button, styles.cancel]}
      >
        <AppText variant="button" style={styles.label}>
          {t('COMMON_CANCEL')}
        </AppText>
      </Pressable>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('PROFILE_SAVE')}
        accessibilityState={{ disabled: !canSave }}
        disabled={!canSave}
        onPress={onSave}
        style={[styles.button, styles.save, !canSave && styles.disabled]}
      >
        <AppText variant="button" style={styles.label}>
          {t('PROFILE_SAVE')}
        </AppText>
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  bar: {
    flexDirection: 'row',
    gap: u(9),
    paddingHorizontal: u(24),
    paddingTop: u(9),
    backgroundColor: colors.purple450,
  },
  button: {
    flex: 1,
    height: u(48),
    borderRadius: u(8),
    alignItems: 'center',
    justifyContent: 'center',
  },
  cancel: { backgroundColor: colors.black40 },
  save: { backgroundColor: colors.ruby },
  disabled: { opacity: 0.5 },
  label: {
    fontFamily: 'Inter-Bold',
    fontSize: u(15),
    textTransform: 'uppercase',
  },
});
