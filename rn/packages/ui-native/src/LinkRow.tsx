import { Pressable, StyleSheet } from 'react-native';
import { AppText } from './AppText';
import { Icon } from './Icon';
import { colors, fonts } from './tokens';
import { u } from './scale';

/** Settings list row: label with a trailing chevron, or an external-link icon for URLs. */
export function LinkRow({
  label,
  onPress,
  external = false,
  selected = false,
  height = 42,
  inline = false,
}: {
  label: string;
  onPress: () => void;
  external?: boolean;
  selected?: boolean;
  /** Row height in design dp. */
  height?: number;
  /** Icon right after the label instead of at the far edge; only external rows get one. */
  inline?: boolean;
}) {
  return (
    <Pressable
      accessibilityRole="link"
      accessibilityLabel={label}
      accessibilityState={{ selected }}
      onPress={onPress}
      style={({ pressed }) => [
        styles.row,
        { minHeight: u(height) },
        inline && styles.inlineRow,
        selected && styles.selected,
        pressed && styles.pressed,
      ]}
    >
      <AppText variant="body" style={[styles.label, selected && styles.bold]}>
        {label}
      </AppText>
      {(!inline || external) && (
        <Icon
          name={external ? 'externalLink' : 'chevronRight'}
          size={external ? 16 : 18}
        />
      )}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  row: {
    minHeight: u(42),
    paddingHorizontal: u(6),
    borderRadius: u(4),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  inlineRow: { justifyContent: 'flex-start', gap: u(8), paddingHorizontal: 0 },
  selected: { backgroundColor: colors.rowSelected },
  pressed: { opacity: 0.8 },
  label: { fontFamily: fonts.medium, fontSize: u(16) },
  bold: { fontFamily: fonts.bold },
});
