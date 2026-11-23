import { ReactNode } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon } from './Icon';
import { color, colors, space } from './tokens';

/** Collapsible list group header (e.g. ONLINE (2) in the friends list): chevron, caps title and an
 * optional count; `children` show while `expanded`.
 * Figma Section/Collapsible: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1079-18
 */
export function SectionCollapsible({
  title,
  count,
  expanded,
  onToggle,
  children,
}: {
  title: string;
  /** Shown as "(n)" after the title. */
  count?: number;
  expanded: boolean;
  onToggle: () => void;
  children?: ReactNode;
}) {
  return (
    <View>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={title}
        accessibilityState={{ expanded }}
        onPress={onToggle}
        style={styles.header}
      >
        <Icon
          name={expanded ? 'chevronDown' : 'chevronUp'}
          size={16}
          color={color.icon.default}
        />
        <View style={styles.text}>
          <AppText variant="Tag/Label" style={styles.title}>
            {title}
          </AppText>
          {count !== undefined && (
            <AppText variant="Tag/Label" style={styles.count}>
              ({count})
            </AppText>
          )}
        </View>
      </Pressable>
      {expanded && children}
    </View>
  );
}

const styles = StyleSheet.create({
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: space['10'],
    paddingVertical: space.sm,
  },
  text: { flexDirection: 'row', alignItems: 'center', gap: space['3xs'] },
  title: { color: color.text.primary },
  count: { color: colors.iconHud },
});
