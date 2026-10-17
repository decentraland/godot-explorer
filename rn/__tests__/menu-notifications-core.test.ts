import { setCoreForTests, type CoreApi } from '../src/core';
import { resetCoreEventsForTests } from '../src/core/events';
import { createMockCore } from '../src/core/mock';
import { createMockNotifications } from '../src/core/mock/notifications';
import {
  CoreError,
  NotificationKind,
  type Notification,
} from '../src/core/native';
import {
  createCoreNotificationsApi,
  toMenuNotification,
} from '../src/menu/coreNotificationsApi';

const NOW = Date.UTC(2026, 9, 12, 12);

function install(overrides: Partial<CoreApi> = {}) {
  const core = {
    ...createMockCore(),
    ...createMockNotifications(() => NOW),
    ...overrides,
  } as CoreApi;
  setCoreForTests(core);
  return core;
}

beforeEach(() => {
  resetCoreEventsForTests();
  install();
});
afterEach(() => setCoreForTests(undefined));

describe('mock notifications core', () => {
  it('lists newest first, filters unread and counts what it marks read', async () => {
    const api = createMockNotifications(() => NOW);
    const all = await api.notifications(undefined, undefined, false);
    expect(all.map(n => n.id)).toEqual(['n1', 'n2', 'n3', 'n4']);
    const unread = await api.notifications(undefined, undefined, true);
    expect(unread).toHaveLength(3);
    expect(await api.markNotificationsRead(['n1', 'n4'])).toBe(1);
    expect(await api.notifications(undefined, undefined, true)).toHaveLength(2);
  });

  it('honors the limit', async () => {
    const api = createMockNotifications(() => NOW);
    expect(await api.notifications(undefined, 1, false)).toHaveLength(1);
  });
});

describe('core notifications menu api', () => {
  it('reports guest and error notification states', async () => {
    install({
      notifications: async () => {
        throw new CoreError.InvalidCall({ detail: 'user not authenticated' });
      },
    });
    expect(await createCoreNotificationsApi().notifications()).toEqual({
      status: 'guest',
    });
    install({
      notifications: async () => {
        throw new CoreError.InvalidCall({ detail: 'network error: down' });
      },
    });
    expect(await createCoreNotificationsApi().notifications()).toEqual({
      status: 'error',
    });
  });

  it('marks one and then every unread notification read', async () => {
    const api = createCoreNotificationsApi();
    const result = await api.notifications();
    if (result.status !== 'ok') {
      throw new Error('expected ok');
    }
    const unread = result.items.filter(n => !n.read);
    await api.markRead(unread[0].id);
    const after = await api.notifications();
    expect(
      after.status === 'ok' &&
        after.items.find(n => n.id === unread[0].id)?.read,
    ).toBe(true);
    await api.markAllRead();
    const done = await api.notifications();
    expect(done.status === 'ok' && done.items.every(n => n.read)).toBe(true);
  });
});

describe('toMenuNotification', () => {
  const at = (kind: NotificationKind): Notification => ({
    id: 'x',
    timestamp: 1_700_000_000_000n,
    read: false,
    kind,
  });

  it('maps every kind', () => {
    const sender = {
      address: '0x1',
      name: 'Ana',
      profileImageUrl: 'u',
      hasClaimedName: true,
    };
    expect(
      toMenuNotification(
        at(new NotificationKind.FriendshipRequest({ sender })),
      ),
    ).toMatchObject({
      type: 'friendship_request',
      subject: 'Ana',
      image: 'u',
      timestamp: 1_700_000_000_000,
    });
    expect(
      toMenuNotification(
        at(new NotificationKind.FriendshipAccepted({ sender })),
      ).type,
    ).toBe('friendship_accepted');
    expect(
      toMenuNotification(
        at(
          new NotificationKind.EventStarted({
            title: 'Party',
            image: 'i',
            link: '',
            eventId: 'ev1',
          }),
        ),
      ),
    ).toMatchObject({ type: 'event', eventId: 'ev1', subject: 'Party' });
    expect(
      toMenuNotification(
        at(
          new NotificationKind.EventStarted({
            title: 'Party',
            image: 'i',
            link: '',
          }),
        ),
      ),
    ).not.toHaveProperty('eventId');
    expect(
      toMenuNotification(
        at(
          new NotificationKind.Reward({
            tokenName: '',
            tokenImage: 'i',
            title: 'Jacket',
          }),
        ),
      ),
    ).toMatchObject({ type: 'reward', subject: 'Jacket' });
  });
});
