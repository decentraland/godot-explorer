import { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Button } from './Button';
import { LinearFill } from './Gradient';
import { color, colors, radius, space } from './tokens';
import { u } from './scale';

const PANEL_STOPS = [
  { offset: 0, color: colors.purple450 },
  { offset: 1, color: colors.purple500 },
];

/** Landscape side menu: the icon rail and a gradient panel with the landscape header (title +
 * optional outline action) and the section's content (Discover, Notifications, Social).
 * Figma Menu/Drawer: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1068-372
 */
export function MenuDrawer({
  rail,
  title,
  action,
  children,
}: {
  /** The landscape `NavBar` rail. */
  rail: ReactNode;
  title: string;
  action?: { label: string; onPress: () => void };
  children?: ReactNode;
}) {
  return (
    <View style={styles.root}>
      <View style={styles.rail}>{rail}</View>
      <View style={styles.panel}>
        <LinearFill stops={PANEL_STOPS} />
        <View style={styles.header}>
          <AppText variant="Header/Title Landscape" style={styles.title}>
            {title}
          </AppText>
          {action && (
            <Button
              tone="outline"
              label={action.label}
              onPress={action.onPress}
            />
          )}
        </View>
        <View style={styles.content}>{children}</View>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flexDirection: 'row', height: u(320) },
  rail: { width: u(50) },
  panel: {
    width: u(312),
    paddingTop: space.xl,
    paddingHorizontal: space['2xl'],
    gap: u(19),
    borderTopRightRadius: radius.sm,
    borderBottomRightRadius: radius.sm,
    overflow: 'hidden',
  },
  header: {
    minHeight: u(37),
    flexDirection: 'row',
    alignItems: 'flex-end',
    justifyContent: 'space-between',
    gap: space.md,
  },
  title: { color: color.text.primary },
  content: { flex: 1, gap: space.md, overflow: 'hidden' },
});
