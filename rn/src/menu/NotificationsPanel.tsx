import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { AppText, colors, Skeleton, u } from '../ds';
import { t } from '../i18n';
import { NotificationCard } from './NotificationCard';
import { unreadCount } from './notifications';
import { PanelEmpty } from './PanelEmpty';
import type { NotificationsResult } from './types';
import type { Loaded } from './useLoad';

export type NotificationsPanelProps = {
  result: Loaded<NotificationsResult>;
  now: number;
  onOpen: (id: string) => void;
  onReadAll: () => void;
  onRetry: () => void;
};

export function NotificationsPanel({
  result,
  now,
  onOpen,
  onReadAll,
  onRetry,
}: NotificationsPanelProps) {
  const items =
    result.status === 'ready' && result.data.status === 'ok'
      ? result.data.items
      : [];
  const canReadAll = unreadCount(items) > 0;

  const body = () => {
    if (result.status === 'loading') {
      return (
        <View style={styles.list}>
          {[0, 1, 2].map(i => (
            <Skeleton key={i} width="100%" height={u(65)} radius={u(8)} />
          ))}
        </View>
      );
    }
    if (result.status === 'error' || result.data.status === 'error') {
      return (
        <Pressable accessibilityRole="button" onPress={onRetry}>
          <PanelEmpty
            icon="wifiOff"
            title={t('MENU_NOTIFICATIONS_ERROR')}
            body={`${t('MENU_NOTIFICATIONS_ERROR_BODY')} ${t('MENU_RETRY')}`}
          />
        </Pressable>
      );
    }
    if (result.data.status === 'guest') {
      return (
        <PanelEmpty
          icon="bell"
          title={t('NOTIFICATIONS_SIGN_IN_TO_GET_NOTIFICATIONS')}
        />
      );
    }
    if (items.length === 0) {
      return (
        <PanelEmpty
          icon="bell"
          title={t('NOTIFICATIONS_NO_NOTIFICATIONS_YET')}
          body={t('NOTIFICATIONS_CHECK_BACK_LATER_FOR_UPDATES')}
        />
      );
    }
    return (
      <ScrollView
        showsVerticalScrollIndicator={false}
        removeClippedSubviews={false}
        contentContainerStyle={styles.list}
      >
        {items.map(item => (
          <NotificationCard
            key={item.id}
            item={item}
            now={now}
            onPress={() => onOpen(item.id)}
          />
        ))}
      </ScrollView>
    );
  };

  return (
    <View style={styles.root}>
      <View style={styles.header}>
        <AppText variant="heading" style={styles.title}>
          {t('MENU_NOTIFICATIONS')}
        </AppText>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('NOTIFICATIONS_READ_ALL')}
          accessibilityState={{ disabled: !canReadAll }}
          disabled={!canReadAll}
          onPress={onReadAll}
          style={({ pressed }) => [
            styles.readAll,
            !canReadAll && styles.disabled,
            pressed && styles.pressed,
          ]}
        >
          <AppText variant="button" style={styles.readAllText}>
            {t('NOTIFICATIONS_READ_ALL')}
          </AppText>
        </Pressable>
      </View>
      {body()}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, paddingLeft: u(24.5), paddingRight: u(24) },
  header: {
    height: u(42),
    marginTop: u(16),
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  title: { fontSize: u(22) },
  readAll: {
    width: u(80),
    height: u(26),
    borderRadius: u(4),
    borderWidth: 1,
    borderColor: colors.snow,
    alignItems: 'center',
    justifyContent: 'center',
  },
  readAllText: { fontFamily: 'Inter-Bold', fontSize: u(12) },
  disabled: { opacity: 0.35 },
  pressed: { opacity: 0.8 },
  list: { gap: u(8), paddingTop: u(10), paddingBottom: u(24) },
});
