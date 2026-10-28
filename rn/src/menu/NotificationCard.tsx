import { Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, u } from '../ds';
import { notificationView, relativeTime } from './notifications';
import { Thumb } from './Cards';
import { Avatar } from './PersonRow';
import type { MenuNotification } from './types';

const SIZE = u(40);

export function NotificationCard({
  item,
  now,
  onPress,
}: {
  item: MenuNotification;
  now: number;
  onPress: () => void;
}) {
  const view = notificationView(item);
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`${view.title}. ${view.body}`}
      onPress={onPress}
      style={({ pressed }) => [
        styles.card,
        item.read ? styles.read : styles.unread,
        pressed && styles.pressed,
      ]}
    >
      {view.round ? (
        <Avatar
          uri={item.image}
          size={SIZE}
          backdrop={item.avatarColor ?? colors.periwinkle}
        />
      ) : (
        <Thumb uri={item.image} width={SIZE} height={SIZE} radius={u(4)} />
      )}
      <View style={styles.texts}>
        <AppText
          variant="button"
          numberOfLines={1}
          style={[styles.title, { color: view.titleColor }]}
        >
          {view.title}
        </AppText>
        {!!view.body && (
          <AppText variant="body" numberOfLines={2} style={styles.body}>
            {view.body}
          </AppText>
        )}
        <AppText variant="caption" style={styles.time}>
          {relativeTime(item.timestamp, now)}
        </AppText>
      </View>
      {!item.read && <View style={styles.dot} />}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  card: {
    height: u(65),
    borderRadius: u(8),
    paddingHorizontal: u(11),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(12),
    backgroundColor: 'rgba(18,0,36,0.55)',
  },
  unread: { borderWidth: 0.5, borderColor: 'rgba(232,185,255,0.35)' },
  read: { opacity: 0.72 },
  pressed: { opacity: 0.85 },
  texts: { flex: 1 },
  title: { fontSize: u(14) },
  body: { fontSize: u(13), color: colors.snow, marginTop: u(1) },
  time: { fontSize: u(10), color: 'rgba(255,255,255,0.5)', marginTop: u(1) },
  dot: {
    width: u(6),
    height: u(6),
    borderRadius: u(3),
    backgroundColor: colors.ruby,
  },
});
