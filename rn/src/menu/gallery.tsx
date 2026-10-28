import '../settings/gallery';
import { ReactElement, ReactNode } from 'react';
import { registerAlias, registerPreview } from '../gallery/registry';
import { Backdrop } from './Backdrop';
import { BugReportModal } from './BugReportModal';
import { DiscoverIngame } from './DiscoverIngame';
import { DiscoverPanel, DiscoverPanelProps } from './DiscoverPanel';
import { MenuDrawerView } from './MenuDrawerView';
import {
  mockEvents,
  mockFeatured,
  mockFriends,
  mockNearby,
  mockNotifications,
  mockRequests,
  mockScene,
  MOCK_NOW,
} from './mockData';
import { NotificationCard } from './NotificationCard';
import { NotificationsPanel } from './NotificationsPanel';
import { ModalTarget, PlaceModal } from './PlaceModal';
import { SocialPanel, SocialPanelProps } from './SocialPanel';
import type { MenuSection } from './drawerState';
import type { SocialState } from './social';
import type { DiscoverFeed, NotificationsResult } from './types';
import type { Loaded } from './useLoad';
import { LinearFill, u } from '../ds';
import { StyleSheet, View } from 'react-native';

const noop = () => {};
const asyncNoop = () => Promise.resolve();
const feed: DiscoverFeed = {
  featured: mockFeatured,
  events: mockEvents,
  featuredEvents: ['event-2'],
};
const ready = <T,>(data: T): Loaded<T> => ({ status: 'ready', data });

const discoverProps: DiscoverPanelProps = {
  scene: mockScene,
  feed: ready(feed),
  now: MOCK_NOW,
  onPlace: noop,
  onEvent: noop,
  onExploreMore: noop,
  onKebab: noop,
  onRetry: noop,
};

const social = (
  over: Partial<Extract<SocialState, { status: 'ready' }>> = {},
): SocialState => ({
  status: 'ready',
  friends: mockFriends,
  requests: mockRequests,
  nearby: mockNearby,
  ...over,
});
const actions = {
  jump: noop,
  accept: noop,
  decline: noop,
  cancel: noop,
  add: noop,
  unblock: noop,
  retry: noop,
};
const socialProps = (
  state: SocialState,
  over: Partial<SocialPanelProps> = {},
): SocialPanelProps => ({ state, actions, ...over });

const notes: NotificationsResult = { status: 'ok', items: mockNotifications };
const notificationsProps = (result: Loaded<NotificationsResult>) => ({
  result,
  now: MOCK_NOW,
  onOpen: noop,
  onReadAll: noop,
  onRetry: noop,
});

const drawer = (
  section: MenuSection,
  panel: ReactNode,
  overlay?: ReactNode,
) => (
  <Backdrop>
    <MenuDrawerView
      section={section}
      unreadNotifications
      onSelect={noop}
      onClose={noop}
      overlay={overlay}
    >
      {panel}
    </MenuDrawerView>
  </Backdrop>
);

const landscape = (
  id: string,
  title: string,
  render: () => ReactElement,
  settleMs = 600,
) => registerPreview(id, { title, orientation: 'landscape', render, settleMs });

const M = 'menu__';
const discover = (over: Partial<DiscoverPanelProps> = {}) => (
  <DiscoverPanel {...discoverProps} {...over} />
);
const base = () => drawer('discover', discover());
const kebab = () => drawer('discover', discover({ initialKebabOpen: true }));

landscape(`${M}discover__367-5030`, 'Discover drawer: scrolled', () =>
  drawer('discover', discover({ scrolledToEnd: true })),
);
landscape(`${M}discover-menu-open__609-3721`, 'Discover drawer', base);
landscape(`${M}discover__888-16872`, 'Discover drawer (kebab closed)', base);
landscape(`${M}discover-event__1511-6307`, 'Discover drawer: events only', () =>
  drawer('discover', discover({ feed: ready({ ...feed, featured: [] }) })),
);
['893-15236', '893-15568', '894-17477'].forEach(node =>
  landscape(`${M}discover-menu-open__${node}`, `Discover kebab ${node}`, kebab),
);

const modal = (target: ModalTarget, reminderSet = false, guest = false) => (
  <Backdrop>
    <PlaceModal
      target={target}
      now={MOCK_NOW}
      reminderSet={reminderSet}
      guest={guest}
      onClose={noop}
      onJumpIn={noop}
      onToggleReminder={noop}
    />
  </Backdrop>
);
const placeTarget: ModalTarget = { kind: 'place', place: mockFeatured[0] };
const live: ModalTarget = {
  kind: 'event',
  event: mockEvents[0],
  featured: true,
};
const soon: ModalTarget = {
  kind: 'event',
  event: mockEvents[1],
  featured: true,
};

landscape(`${M}discover-place__1351-15649`, 'Place modal', () =>
  modal(placeTarget),
);
landscape(`${M}discover-event-live__1511-6210`, 'Event modal: live', () =>
  modal(live),
);
landscape(
  `${M}discover-event-reminder__1525-5237`,
  'Event modal: remind me',
  () => modal(soon),
);
landscape(
  `${M}discover-event-set__1525-5303`,
  'Event modal: reminder set',
  () => modal(soon, true),
);
landscape('extra/menu-event-guest', 'Event modal: guest', () =>
  modal(soon, false, true),
);
landscape('extra/menu-discover-loading', 'Discover drawer: loading', () =>
  drawer('discover', discover({ feed: { status: 'loading' } })),
);
landscape('extra/menu-discover-error', 'Discover drawer: error', () =>
  drawer('discover', discover({ feed: { status: 'error' } })),
);

const socialDrawer = (props: SocialPanelProps) =>
  drawer('social', <SocialPanel {...props} />);

landscape(`${M}social__426-5733`, 'Social: friends', () =>
  socialDrawer(socialProps(social())),
);
landscape(`${M}social-request__889-4540`, 'Social: requests', () =>
  socialDrawer(socialProps(social(), { initialTab: 'requests' })),
);
const received = mockRequests[0].address;
landscape(`${M}social-request-accept__889-5102`, 'Social: accept pressed', () =>
  socialDrawer(
    socialProps(social(), {
      initialTab: 'requests',
      forcePressed: { address: received, kind: 'accept' },
    }),
  ),
);
landscape(
  `${M}social-request-decline__889-5361`,
  'Social: decline pressed',
  () =>
    socialDrawer(
      socialProps(social(), {
        initialTab: 'requests',
        forcePressed: { address: received, kind: 'decline' },
      }),
    ),
);
landscape(`${M}social-nearby__889-5850`, 'Social: nearby', () =>
  socialDrawer(socialProps(social(), { initialTab: 'nearby' })),
);
landscape(`${M}social-nearby-empty__931-24777`, 'Social: nearby empty', () =>
  socialDrawer(socialProps(social({ nearby: [] }), { initialTab: 'nearby' })),
);
const noFriends = () => socialDrawer(socialProps(social({ friends: [] })));
landscape(`${M}social__933-25219`, 'Social: friends empty', noFriends);
landscape(
  `${M}social__1135-16597`,
  'Social: friends empty with counts',
  noFriends,
);
landscape(`${M}social__933-26017`, 'Social: sections collapsed', () =>
  socialDrawer(
    socialProps(social(), {
      initialCollapsed: ['online', 'offline', 'blocked'],
    }),
  ),
);
landscape(`${M}social__933-26834`, 'Social: sections expanded', () =>
  socialDrawer(
    socialProps(
      social({
        friends: [{ ...mockFriends[0] }, { ...mockFriends[2] }],
      }),
      { initialCollapsed: ['blocked'] },
    ),
  ),
);
landscape('extra/menu-social-requests-empty', 'Social: no requests', () =>
  socialDrawer(
    socialProps(social({ requests: [] }), { initialTab: 'requests' }),
  ),
);
landscape('extra/menu-social-loading', 'Social: loading', () =>
  socialDrawer(socialProps({ status: 'loading' })),
);
landscape('extra/menu-social-error', 'Social: error', () =>
  socialDrawer(socialProps({ status: 'error' })),
);
landscape('extra/menu-social-guest', 'Social: guest', () =>
  socialDrawer(socialProps({ status: 'guest' })),
);

const notifications = (result: Loaded<NotificationsResult>) =>
  drawer(
    'notifications',
    <NotificationsPanel {...notificationsProps(result)} />,
  );
landscape(`${M}notifications__426-6356`, 'Notifications', () =>
  notifications(ready(notes)),
);
landscape('extra/menu-notifications-empty', 'Notifications: empty', () =>
  notifications(ready({ status: 'ok', items: [] })),
);
landscape('extra/menu-notifications-guest', 'Notifications: guest', () =>
  notifications(ready({ status: 'guest' })),
);
landscape('extra/menu-notifications-error', 'Notifications: error', () =>
  notifications(ready({ status: 'error' })),
);
landscape('extra/menu-notifications-loading', 'Notifications: loading', () =>
  notifications({ status: 'loading' }),
);
landscape('extra/menu-notifications-all-read', 'Notifications: all read', () =>
  notifications(
    ready({
      status: 'ok',
      items: mockNotifications.map(n => ({ ...n, read: true })),
    }),
  ),
);

const card = (index: number) => () =>
  (
    <View style={styles.cardStage}>
      <LinearFill
        angle={135}
        stops={[
          { offset: 0, color: '#691FA9' },
          { offset: 1, color: '#460F6F' },
        ]}
      />
      <View style={styles.card}>
        <NotificationCard
          item={mockNotifications[index]}
          now={MOCK_NOW}
          onPress={noop}
        />
      </View>
    </View>
  );
landscape(
  `${M}notifications__891-8628`,
  'Notification: friend accepted',
  card(0),
);
landscape(
  `${M}notifications__891-6976`,
  'Notification: friend request',
  card(1),
);
landscape(`${M}notifications__891-6977`, 'Notification: event', card(3));
landscape(`${M}notifications__891-6978`, 'Notification: reward', card(2));
landscape(
  'extra/menu-notification-generic',
  'Notification: generic fallback',
  card(4),
);

registerPreview(`${M}discover-ingame__889-4068`, {
  title: 'Discover in game (portrait)',
  orientation: 'portrait',
  render: () => (
    <DiscoverIngame
      feed={feed}
      friends={mockFriends}
      played={mockFeatured}
      now={MOCK_NOW}
      onBack={noop}
      onPlace={noop}
      onEvent={noop}
    />
  ),
  settleMs: 800,
});

const bug =
  (sent = false) =>
  () =>
    (
      <View style={styles.portrait}>
        <LinearFill
          stops={[
            { offset: 0, color: '#7B27B8' },
            { offset: 1, color: '#460F6F' },
          ]}
        />
        <BugReportModal
          capture={() => Promise.resolve('mock://world-capture.jpg')}
          onSubmit={asyncNoop}
          onClose={noop}
          sent={sent}
          initial={{
            category: null,
            description: '',
            screenshots: ['mock://world-capture.jpg'],
          }}
        />
      </View>
    );
registerPreview(`${M}settings-account__893-16220`, {
  title: 'Bug report (portrait)',
  orientation: 'portrait',
  render: bug(),
});
registerPreview('extra/menu-bug-sent', {
  title: 'Bug report: sent',
  orientation: 'portrait',
  render: bug(true),
});

registerAlias(
  `${M}discover-share__893-14904`,
  `${M}discover-place__1351-15649`,
  'landscape',
  'Native OS share sheet (shareScene) over the place modal',
);
registerAlias(
  `${M}report-content-form__894-16932`,
  'settings-revamp__settings-account-upgraded__1398-7378',
  'portrait',
  'External Google Form in the browser (REPORT_CONTENT_URL), not RN UI; opened from Report Content',
);

const styles = StyleSheet.create({
  cardStage: { flex: 1, alignItems: 'center', justifyContent: 'center' },
  card: { width: u(264) },
  portrait: { flex: 1 },
});
