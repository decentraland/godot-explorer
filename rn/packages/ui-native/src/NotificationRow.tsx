import { ReactNode } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { color, colors, radius, space } from './tokens';
import { u } from './scale';

export type NotificationKind = 'player' | 'event' | 'item';

const TITLE_COLOR: Record<NotificationKind, string> = {
  player: colors.periwinkle,
  event: color.text.primary,
  item: colors.lavender,
};

/** Notifications panel row: an 80 px thumbnail (avatar, event image or item card), title in the
 * kind's colour, body and date. `read` is Figma's `State=Old` (70% opacity); no `title` is NoTitle.
 * Figma Notifications: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1090-6609
 */
export function NotificationRow({
  thumbnail,
  kind = 'player',
  title,
  body,
  date,
  read = false,
  onPress,
}: {
  thumbnail: ReactNode;
  kind?: NotificationKind;
  title?: string;
  body: string;
  date: string;
  read?: boolean;
  onPress?: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      onPress={onPress}
      disabled={!onPress}
      style={[styles.card, read && styles.read]}
    >
      <View style={styles.thumb}>{thumbnail}</View>
      <View style={styles.text}>
        <View style={styles.copy}>
          {title !== undefined && (
            <AppText
              variant="B2"
              style={{ color: TITLE_COLOR[kind] }}
              numberOfLines={1}
            >
              {title}
            </AppText>
          )}
          <AppText variant="Caption2" style={styles.body} numberOfLines={2}>
            {body}
          </AppText>
        </View>
        <AppText style={styles.date}>{date}</AppText>
      </View>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  card: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: space.lg,
    padding: u(11),
    borderRadius: radius.xl,
    backgroundColor: color.surface.overlay,
  },
  read: { opacity: 0.7 },
  thumb: { width: u(40), height: u(40) },
  text: { flex: 1, gap: space['3xs'] },
  copy: { gap: space['2xs'] },
  body: { color: color.text.primary },
  date: { fontSize: u(7.5), color: colors.silver },
});
