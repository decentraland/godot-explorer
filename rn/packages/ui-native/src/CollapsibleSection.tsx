import { ReactNode } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon } from './Icon';
import { colors, fonts } from './tokens';
import { u } from './scale';

/** Lavender caps caption with a divider; tapping the header toggles `children`. */
export function CollapsibleSection({
  title,
  open,
  onToggle,
  children,
}: {
  title: string;
  open: boolean;
  onToggle: () => void;
  children?: ReactNode;
}) {
  return (
    <View>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={title}
        accessibilityState={{ expanded: open }}
        onPress={onToggle}
        style={styles.header}
      >
        <AppText variant="label" style={styles.title}>
          {title}
        </AppText>
        <Icon
          name={open ? 'chevronUp' : 'chevronDown'}
          size={16}
          color={colors.lavender}
        />
      </Pressable>
      <View style={styles.divider} />
      {open && <View style={styles.body}>{children}</View>}
    </View>
  );
}

const styles = StyleSheet.create({
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingVertical: u(8),
  },
  title: { color: colors.lavender, fontFamily: fonts.bold },
  divider: { height: 1, backgroundColor: colors.lavender },
  body: { paddingTop: u(12) },
});
