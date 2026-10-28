import { Pressable, StyleSheet } from 'react-native';
import { AppText, colors, Icon } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';

/** EDIT PROFILE: outlined, dark fill; `compact` is the landscape size. */
export function EditButton({
  onPress,
  compact = false,
}: {
  onPress: () => void;
  compact?: boolean;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t('PROFILE_EDIT_PROFILE_CAPS')}
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        compact ? styles.compact : styles.large,
        pressed && styles.pressed,
      ]}
    >
      <Icon name="pencil" size={compact ? 12 : 16} strokeWidth={2} />
      <AppText
        variant="button"
        style={[styles.label, { fontSize: u(compact ? 11 : 16) }]}
      >
        {t('PROFILE_EDIT_PROFILE_CAPS')}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  button: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: u(8),
    borderWidth: 1,
    borderColor: colors.white30,
    backgroundColor: 'rgba(20,0,40,0.55)',
    gap: u(8),
  },
  large: { height: u(48), alignSelf: 'stretch' },
  compact: { height: u(30), paddingHorizontal: u(12) },
  label: { fontFamily: 'Inter-Bold', textTransform: 'uppercase' },
  pressed: { opacity: 0.8 },
});
