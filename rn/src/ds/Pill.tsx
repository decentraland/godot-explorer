import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors, radii } from './tokens';
import { u } from './scale';

/** Selectable chip. `tone="solid"` is the ruby main-mode switch, `"soft"` the lavender sub-filter. */
export function Pill({
  label,
  icon,
  selected,
  tone = 'solid',
  onPress,
}: {
  label: string;
  icon?: IconName;
  selected: boolean;
  tone?: 'solid' | 'soft';
  onPress: () => void;
}) {
  const selectedBg = tone === 'solid' ? colors.ruby : colors.lavender;
  const idleBg = tone === 'solid' ? colors.purple550 : colors.glass;
  const textColor = selected
    ? tone === 'solid'
      ? colors.snow
      : colors.shadow
    : tone === 'solid'
    ? colors.snow
    : colors.white70;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ selected }}
      onPress={onPress}
      style={({ pressed }) => [
        styles.pill,
        { backgroundColor: selected ? selectedBg : idleBg },
        pressed && styles.pressed,
      ]}
    >
      {icon && (
        <View style={styles.icon}>
          <Icon name={icon} size={18} color={textColor} />
        </View>
      )}
      <AppText
        variant="button"
        style={[
          tone === 'solid' ? styles.solidText : styles.softText,
          { color: textColor },
        ]}
      >
        {label}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  pill: {
    height: u(30),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: u(8),
    borderRadius: radii.pill,
  },
  icon: { marginRight: u(4) },
  solidText: { fontSize: u(14) },
  softText: { fontSize: u(13) },
  pressed: { opacity: 0.8 },
});
