import { setCoreForTests, type CoreApi } from '../src/core';
import { resetCoreEventsForTests } from '../src/core/events';
import { createMockCore } from '../src/core/mock';
import { createMockNotifications } from '../src/core/mock/notifications';
import { createMockSocial } from '../src/core/mock/social';
import {
  CoreError,
  NotificationKind,
  type Notification,
} from '../src/core/native';
import {
  createCoreSocialApi,
  GuestError,
  toMenuNotification,
} from '../src/menu/coreSocialApi';

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

describe('mock social core', () => {
  it('moves an accepted request into the friends list and announces it', async () => {
    const changed = jest.fn();
    const social = createMockSocial(changed);
    const [first] = (await social.friendRequests()).received;
    await social.acceptFriendRequest(first.address);
    const friends = await social.friends(0, 0);
    expect(friends.map(f => f.address)).toContain(first.address);
    expect((await social.friendRequests()).received).toHaveLength(1);
    expect(changed).toHaveBeenCalledTimes(1);
  });

  it('rejects answering a request that does not exist', async () => {
    await expect(
      createMockSocial().acceptFriendRequest('0xnope'),
    ).rejects.toBeInstanceOf(CoreError.InvalidCall);
  });

  it('turns a nearby user into a sent request, and hides blocked users', async () => {
    const social = createMockSocial();
    const [user, other] = await social.nearby();
    await social.sendFriendRequest(user.address, '');
    expect((await social.friendRequests()).sent.map(f => f.address)).toContain(
      user.address,
    );
    await social.blockUser(other.address);
    const nearby = await social.nearby();
    expect(nearby.map(u => u.address)).not.toContain(user.address);
    expect(nearby.map(u => u.address)).not.toContain(other.address);
  });

  it('pages the friends list', async () => {
    const social = createMockSocial();
    expect(await social.friends(2, 1)).toHaveLength(2);
  });
});

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

describe('core social menu api', () => {
  it('maps friends, requests and nearby', async () => {
    const data = await createCoreSocialApi().social();
    expect(data.friends.map(f => f.name)).toContain('Ana');
    expect(data.friends.every(f => f.status === 'offline')).toBe(true);
    expect(data.requests.filter(r => r.direction === 'received')).toHaveLength(
      2,
    );
    expect(data.nearby.length).toBeGreaterThan(0);
    expect(data.friends.find(f => f.name === 'Dani')).toMatchObject({
      claimed: false,
    });
  });

  it('survives nearby failing', async () => {
    install({
      nearby: async () => {
        throw new CoreError.InvalidCall({ detail: 'no scene' });
      },
    });
    expect((await createCoreSocialApi().social()).nearby).toEqual([]);
  });

  it('accepting drops the request from the next load', async () => {
    const api = createCoreSocialApi();
    const [first] = (await api.social()).requests;
    await api.acceptRequest(first.address);
    const after = await api.social();
    expect(after.requests.map(r => r.address)).not.toContain(first.address);
    expect(after.friends.map(f => f.address)).toContain(first.address);
  });

  it('sends requests with an empty message', async () => {
    const send = jest.fn(async () => {});
    install({ sendFriendRequest: send });
    await createCoreSocialApi().sendRequest('0x1');
    expect(send).toHaveBeenCalledWith('0x1', '');
  });

  it('propagates action failures', async () => {
    install({
      rejectFriendRequest: async () => {
        throw new Error('boom');
      },
    });
    await expect(createCoreSocialApi().rejectRequest('0x1')).rejects.toThrow(
      'boom',
    );
  });

  it('raises GuestError when the account has no social service', async () => {
    install({
      friends: async () => {
        throw new CoreError.InvalidCall({ detail: 'not signed in' });
      },
    });
    await expect(createCoreSocialApi().social()).rejects.toBeInstanceOf(
      GuestError,
    );
  });

  it('reports guest and error notification states', async () => {
    install({
      notifications: async () => {
        throw new CoreError.InvalidCall({ detail: 'user not authenticated' });
      },
    });
    expect(await createCoreSocialApi().notifications()).toEqual({
      status: 'guest',
    });
    install({
      notifications: async () => {
        throw new CoreError.InvalidCall({ detail: 'network error: down' });
      },
    });
    expect(await createCoreSocialApi().notifications()).toEqual({
      status: 'error',
    });
  });

  it('marks one and then every unread notification read', async () => {
    const api = createCoreSocialApi();
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
