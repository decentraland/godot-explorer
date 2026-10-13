import { ReactNode, useState } from 'react';
import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { AppText, Badge, colors, Icon, Skeleton, u } from '../ds';
import { t } from '../i18n';
import { PanelEmpty } from './PanelEmpty';
import { AddFriendButton, PersonRow, SquareButton } from './PersonRow';
import {
  groupFriends,
  groupRequests,
  SocialState,
  SocialTab,
  tabBadges,
} from './social';

export type SocialActions = {
  jump: (address: string) => void;
  accept: (address: string) => void;
  decline: (address: string) => void;
  cancel: (address: string) => void;
  add: (address: string) => void;
  unblock: (address: string) => void;
  retry: () => void;
};

export type SocialPanelProps = {
  state: SocialState;
  actions: SocialActions;
  initialTab?: SocialTab;
  /** Gallery states. */
  initialCollapsed?: string[];
  forcePressed?: { address: string; kind: 'accept' | 'decline' };
};

function SectionHeader({
  label,
  open,
  onToggle,
}: {
  label: string;
  open: boolean;
  onToggle: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ expanded: open }}
      onPress={onToggle}
      style={styles.sectionHeader}
    >
      <Icon
        name={open ? 'chevronUp' : 'chevronDown'}
        size={12}
        strokeWidth={3}
      />
      <AppText variant="label" style={styles.sectionLabel}>
        {label}
      </AppText>
    </Pressable>
  );
}

function Tab({
  label,
  count,
  dot,
  selected,
  onPress,
}: {
  label: string;
  count?: number;
  dot?: boolean;
  selected: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      accessibilityRole="tab"
      accessibilityLabel={label}
      accessibilityState={{ selected }}
      onPress={onPress}
      style={styles.tab}
    >
      <View style={styles.tabLabel}>
        <AppText
          variant="tab"
          style={[styles.tabText, selected && styles.tabOn]}
        >
          {label}
        </AppText>
        {!!count && (
          <AppText variant="body" style={styles.tabCount}>
            {count}
          </AppText>
        )}
        {dot && (
          <View style={styles.dot}>
            <Badge />
          </View>
        )}
      </View>
      <View style={[styles.underline, selected && styles.underlineOn]} />
    </Pressable>
  );
}

export function SocialPanel({
  state,
  actions,
  initialTab = 'friends',
  initialCollapsed = [],
  forcePressed,
}: SocialPanelProps) {
  const [tab, setTab] = useState<SocialTab>(initialTab);
  const [collapsed, setCollapsed] = useState(new Set(initialCollapsed));
  const toggle = (key: string) =>
    setCollapsed(prev => {
      const next = new Set(prev);
      if (!next.delete(key)) {
        next.add(key);
      }
      return next;
    });
  const badges = tabBadges(state);

  const section = (key: string, label: string, rows: ReactNode[]) => (
    <View key={key}>
      <SectionHeader
        label={label}
        open={!collapsed.has(key)}
        onToggle={() => toggle(key)}
      />
      {!collapsed.has(key) && rows}
    </View>
  );

  const friends = () => {
    if (state.status !== 'ready') {
      return null;
    }
    const { online, offline, blocked } = groupFriends(state.friends);
    if (state.friends.length === 0) {
      return (
        <PanelEmpty
          icon="peopleHeart"
          title={t('MENU_SOCIAL_NO_FRIENDS')}
          body={t('MENU_SOCIAL_NO_FRIENDS_BODY')}
        />
      );
    }
    return (
      <>
        {section(
          'online',
          t('MENU_SOCIAL_ONLINE', { count: online.length }),
          online.map(f => (
            <PersonRow
              key={f.address}
              person={f}
              subtitle={f.samePlace ? t('MENU_SOCIAL_SAME_PLACE') : f.place}
              dim={f.samePlace}
              right={
                <SquareButton
                  kind="jump"
                  name={f.name}
                  disabled={f.samePlace}
                  onPress={() => actions.jump(f.address)}
                />
              }
            />
          )),
        )}
        {section(
          'offline',
          t('MENU_SOCIAL_OFFLINE', { count: offline.length }),
          offline.map(f => (
            <PersonRow key={f.address} person={f} subtitle={f.place} />
          )),
        )}
        {section(
          'blocked',
          t('MENU_SOCIAL_BLOCKED', { count: blocked.length }),
          blocked.map(f => (
            <PersonRow
              key={f.address}
              person={f}
              right={
                <Pressable
                  accessibilityRole="button"
                  onPress={() => actions.unblock(f.address)}
                >
                  <AppText variant="button" style={styles.unblock}>
                    {t('MENU_SOCIAL_UNBLOCK')}
                  </AppText>
                </Pressable>
              }
            />
          )),
        )}
      </>
    );
  };

  const requests = () => {
    if (state.status !== 'ready') {
      return null;
    }
    const { received, sent } = groupRequests(state.requests);
    if (state.requests.length === 0) {
      return (
        <PanelEmpty
          icon="peopleHeart"
          title={t('MENU_SOCIAL_NO_REQUESTS')}
          body={t('MENU_SOCIAL_NO_REQUESTS_BODY')}
        />
      );
    }
    const pressedKind = (address: string) =>
      forcePressed?.address === address ? forcePressed.kind : undefined;
    return (
      <>
        {section(
          'received',
          t('MENU_SOCIAL_RECEIVED', { count: received.length }),
          received.map(r => (
            <PersonRow
              key={r.address}
              person={r}
              subtitle={r.place}
              right={
                <View style={styles.pair}>
                  <SquareButton
                    kind="accept"
                    pressed={pressedKind(r.address) === 'accept'}
                    onPress={() => actions.accept(r.address)}
                  />
                  <SquareButton
                    kind="decline"
                    pressed={pressedKind(r.address) === 'decline'}
                    onPress={() => actions.decline(r.address)}
                  />
                </View>
              }
            />
          )),
        )}
        {section(
          'sent',
          t('MENU_SOCIAL_SENT', { count: sent.length }),
          sent.map(r => (
            <PersonRow
              key={r.address}
              person={r}
              subtitle={r.place}
              right={
                <SquareButton
                  kind="cancel"
                  onPress={() => actions.cancel(r.address)}
                />
              }
            />
          )),
        )}
      </>
    );
  };

  const nearby = () => {
    if (state.status !== 'ready') {
      return null;
    }
    if (state.nearby.length === 0) {
      return (
        <PanelEmpty
          icon="peopleHeart"
          title={t('MENU_SOCIAL_NO_NEARBY')}
          body={t('MENU_SOCIAL_NO_NEARBY_BODY')}
        />
      );
    }
    return state.nearby.map(p => (
      <PersonRow
        key={p.address}
        person={p}
        subtitle={p.place}
        right={<AddFriendButton onPress={() => actions.add(p.address)} />}
      />
    ));
  };

  const body = () => {
    switch (state.status) {
      case 'loading':
        return (
          <View style={styles.skeletons}>
            {[0, 1, 2].map(i => (
              <Skeleton key={i} width="100%" height={u(40)} radius={u(8)} />
            ))}
          </View>
        );
      case 'error':
        return (
          <Pressable accessibilityRole="button" onPress={actions.retry}>
            <PanelEmpty
              icon="wifiOff"
              title={t('MENU_SOCIAL_ERROR')}
              body={`${t('MENU_SOCIAL_ERROR_BODY')} ${t('MENU_RETRY')}`}
            />
          </Pressable>
        );
      case 'guest':
        return (
          <PanelEmpty
            icon="lock"
            title={t('MENU_SOCIAL_GUEST')}
            body={t('MENU_SOCIAL_GUEST_BODY')}
          />
        );
      default:
        return { friends, requests, nearby }[tab]();
    }
  };

  return (
    <View style={styles.root}>
      <AppText variant="heading" style={styles.title}>
        {t('MENU_SOCIAL')}
      </AppText>
      <View style={styles.tabs}>
        <Tab
          label={t('MENU_SOCIAL_TAB_FRIENDS')}
          selected={tab === 'friends'}
          onPress={() => setTab('friends')}
        />
        <Tab
          label={t('MENU_SOCIAL_TAB_REQUESTS')}
          dot={badges.requests > 0}
          selected={tab === 'requests'}
          onPress={() => setTab('requests')}
        />
        <Tab
          label={t('MENU_SOCIAL_TAB_NEARBY')}
          count={badges.nearby}
          selected={tab === 'nearby'}
          onPress={() => setTab('nearby')}
        />
      </View>
      <ScrollView
        showsVerticalScrollIndicator={false}
        removeClippedSubviews={false}
        contentContainerStyle={styles.list}
      >
        {body()}
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, paddingLeft: u(24.5), paddingRight: u(24) },
  title: { fontSize: u(22), marginTop: u(14), lineHeight: u(30) },
  tabs: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginTop: u(14),
  },
  tab: { paddingTop: u(2) },
  tabLabel: { flexDirection: 'row', alignItems: 'center', gap: u(4) },
  tabText: { fontSize: u(15), color: colors.white70 },
  tabOn: { color: colors.snow },
  tabCount: { fontSize: u(13), color: colors.white70 },
  dot: { position: 'absolute', right: -u(4), top: -u(2) },
  underline: { height: u(2), marginTop: u(6), backgroundColor: 'transparent' },
  underlineOn: { backgroundColor: colors.ruby },
  list: { paddingTop: u(10), paddingBottom: u(24) },
  sectionHeader: {
    height: u(30),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(6),
    marginTop: u(4),
  },
  sectionLabel: { fontFamily: 'Inter-SemiBold', fontSize: u(13) },
  pair: { flexDirection: 'row', gap: u(12) },
  unblock: { fontSize: u(12), color: colors.lavender },
  skeletons: { gap: u(14), paddingTop: u(16) },
});
