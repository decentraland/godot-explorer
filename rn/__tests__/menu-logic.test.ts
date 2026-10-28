import {
  addScreenshot,
  canSubmit,
  emptyForm,
  MAX_DESCRIPTION,
  MAX_SCREENSHOTS,
  removeScreenshot,
  withDescription,
} from '../src/menu/bugReport';
import { linkQueryFor, sceneLink } from '../src/menu/api';
import {
  closeMenu,
  closeZones,
  openMenu,
  resetDrawerForTests,
  selectSection,
} from '../src/menu/drawerState';
import { createMockApi } from '../src/menu/mockApi';
import { MOCK_NOW, mockNotifications, mockSocial } from '../src/menu/mockData';
import {
  markAllRead,
  markRead,
  notificationTarget,
  notificationView,
  relativeTime,
  unreadCount,
} from '../src/menu/notifications';
import { sceneShareMessage } from '../src/menu/share';
import {
  groupFriends,
  groupRequests,
  socialReducer,
  SocialState,
  tabBadges,
} from '../src/menu/social';
import { ctaFor } from '../src/menu/PlaceModal';
import { mainRealm } from '../src/discover/api';

const MIN = 60_000;
const DAY = 24 * 60 * MIN;
const ago = (ms: number) => relativeTime(MOCK_NOW - ms, MOCK_NOW);

describe('relativeTime', () => {
  it('uses the legacy copy at each scale', () => {
    expect(ago(10_000)).toBe('Now');
    expect(ago(5 * MIN)).toBe('5m ago');
    expect(ago(3 * 60 * MIN)).toBe('3h ago');
    expect(ago(DAY)).toBe('Yesterday');
    expect(ago(2 * DAY)).toBe('2 days ago');
    expect(ago(7 * DAY)).toBe('1 week ago');
    expect(ago(21 * DAY)).toBe('3 weeks ago');
    expect(ago(60 * DAY)).toBe('2 months ago');
    expect(ago(400 * DAY)).toBe('1 year ago');
    expect(ago(800 * DAY)).toBe('2 years ago');
  });

  it('never goes negative for a future timestamp', () => {
    expect(relativeTime(MOCK_NOW + DAY, MOCK_NOW)).toBe('Now');
  });
});

describe('notificationView', () => {
  const byType = (type: string) =>
    notificationView(mockNotifications.find(n => n.type === type)!);

  it('maps the four families to legacy copy and colours', () => {
    expect(byType('friendship_accepted')).toMatchObject({
      body: 'Accepted your friend request',
      titleColor: '#A0ABFF',
      round: true,
    });
    expect(byType('friendship_request').body).toBe('Want to be your friend!');
    expect(byType('event')).toMatchObject({
      body: 'Event started!',
      titleColor: '#73FFAF',
      round: false,
    });
    expect(byType('reward')).toMatchObject({
      title: 'New Item Received',
      body: 'Space Jacket',
      titleColor: '#DF9CFF',
    });
  });

  it('falls back to a generic card for other types', () => {
    const generic = byType('other');
    expect(generic.title).toBe('Notification');
    expect(generic.body).toBe('Voting has ended');
    expect(generic.titleColor).toBe('#FCFCFC');
  });
});

describe('notification state', () => {
  it('counts and marks read without mutating', () => {
    expect(unreadCount(mockNotifications)).toBe(3);
    const one = markRead(mockNotifications, 'n1');
    expect(unreadCount(one)).toBe(2);
    expect(unreadCount(mockNotifications)).toBe(3);
    expect(unreadCount(markAllRead(mockNotifications))).toBe(0);
  });

  it('routes cards to social, backpack or the event', () => {
    const find = (id: string) => mockNotifications.find(n => n.id === id)!;
    expect(notificationTarget(find('n2'))).toEqual({
      section: 'social',
      tab: 'requests',
    });
    expect(notificationTarget(find('n1'))).toEqual({
      section: 'social',
      tab: 'friends',
    });
    expect(notificationTarget(find('n3'))).toEqual({ section: 'backpack' });
    expect(notificationTarget(find('n4'))).toBeNull();
    expect(notificationTarget({ ...find('n4'), eventId: 'e1' })).toEqual({
      eventId: 'e1',
    });
    expect(notificationTarget(find('n5'))).toBeNull();
  });
});

describe('social', () => {
  const ready: SocialState = { status: 'ready', ...mockSocial };
  const received = mockSocial.requests[0].address;

  it('groups friends and requests', () => {
    const g = groupFriends(mockSocial.friends);
    expect([g.online.length, g.offline.length, g.blocked.length]).toEqual([
      2, 2, 2,
    ]);
    const r = groupRequests(mockSocial.requests);
    expect([r.received.length, r.sent.length]).toEqual([2, 1]);
  });

  it('accepting moves the request into friends', () => {
    const next = socialReducer(ready, { type: 'accepted', address: received });
    if (next.status !== 'ready') {
      throw new Error('expected ready');
    }
    expect(next.requests.some(r => r.address === received)).toBe(false);
    expect(next.friends.find(f => f.address === received)?.status).toBe(
      'offline',
    );
  });

  it('removes, sends and unblocks', () => {
    const removed = socialReducer(ready, {
      type: 'removed',
      address: received,
    });
    expect(tabBadges(removed).requests).toBe(1);
    const nearby = mockSocial.nearby[0].address;
    const sent = socialReducer(ready, { type: 'requested', address: nearby });
    expect(tabBadges(sent).nearby).toBe(9);
    const blocked = mockSocial.friends.find(f => f.status === 'blocked')!;
    const un = socialReducer(ready, {
      type: 'unblocked',
      address: blocked.address,
    });
    expect(
      un.status === 'ready' && groupFriends(un.friends).blocked.length,
    ).toBe(1);
  });

  it('handles load states and ignores edits before data', () => {
    expect(socialReducer(ready, { type: 'failed' })).toEqual({
      status: 'error',
    });
    expect(socialReducer(ready, { type: 'guest' })).toEqual({
      status: 'guest',
    });
    expect(socialReducer(ready, { type: 'loading' })).toEqual({
      status: 'loading',
    });
    const loading: SocialState = { status: 'loading' };
    expect(socialReducer(loading, { type: 'removed', address: 'x' })).toBe(
      loading,
    );
    expect(
      socialReducer(loading, { type: 'loaded', data: mockSocial }).status,
    ).toBe('ready');
    expect(tabBadges(loading)).toEqual({ requests: 0, nearby: 0 });
  });
});

describe('drawer state and close zones', () => {
  beforeEach(resetDrawerForTests);

  it('opens on a section and closes', () => {
    const seen: boolean[] = [];
    openMenu('social');
    seen.push(true);
    selectSection('notifications');
    closeMenu();
    expect(seen).toEqual([true]);
  });

  it('leaves the panel and the gamepad out of the close zones', () => {
    const screen = { width: 800, height: 360 };
    const panel = { left: 54, top: 16, right: 416, bottom: 336 };
    const pad = { left: 602, top: 190, right: 800, bottom: 360 };
    const zones = closeZones(screen, panel, pad);
    const hit = (x: number, y: number) =>
      zones.some(
        z =>
          x >= z.left &&
          x < z.left + z.width &&
          y >= z.top &&
          y < z.top + z.height,
      );
    expect(hit(200, 100)).toBe(false);
    expect(hit(700, 300)).toBe(false);
    expect(hit(500, 300)).toBe(true);
    expect(hit(700, 50)).toBe(true);
    expect(hit(10, 200)).toBe(true);
    expect(hit(200, 5)).toBe(true);
    expect(hit(200, 350)).toBe(true);
  });
});

describe('bug report form', () => {
  it('needs a type and a description', () => {
    expect(canSubmit(emptyForm)).toBe(false);
    expect(canSubmit({ ...emptyForm, category: 'chat' })).toBe(false);
    expect(
      canSubmit({ ...emptyForm, category: 'chat', description: '  ' }),
    ).toBe(false);
    expect(
      canSubmit({ ...emptyForm, category: 'chat', description: 'It froze' }),
    ).toBe(true);
  });

  it('caps the description and the screenshots', () => {
    expect(
      withDescription(emptyForm, 'x'.repeat(MAX_DESCRIPTION + 5)).description,
    ).toHaveLength(MAX_DESCRIPTION);
    let form = emptyForm;
    for (let i = 0; i <= MAX_SCREENSHOTS; i++) {
      form = addScreenshot(form, `s${i}`);
    }
    expect(form.screenshots).toEqual(['s0', 's1', 's2']);
    expect(removeScreenshot(form, 1).screenshots).toEqual(['s0', 's2']);
  });
});

describe('share and links', () => {
  it('builds the legacy message with the scene link', () => {
    expect(
      sceneShareMessage({
        title: 'Space Runner',
        creator: '',
        linkQuery: 'realm=spacerunner.dcl.eth',
      }),
    ).toBe(
      '📍 Join Me At Space Runner following this link: https://mobile.dclexplorer.com/open?realm=spacerunner.dcl.eth',
    );
    expect(sceneLink('position=1,2')).toBe(
      'https://mobile.dclexplorer.com/open?position=1,2',
    );
  });

  it('uses a position in Genesis City and the world name elsewhere', () => {
    expect(linkQueryFor(mainRealm(), 3, -4)).toBe('position=3,-4');
    expect(
      linkQueryFor(
        'https://worlds-content-server.decentraland.org/world/foo.dcl.eth',
        0,
        0,
      ),
    ).toBe('realm=foo.dcl.eth');
  });
});

describe('event call to action', () => {
  const live = {
    kind: 'event',
    event: { live: true },
    featured: false,
  } as never;
  const soon = {
    kind: 'event',
    event: { live: false },
    featured: false,
  } as never;
  it('picks jump, remind, set or sign-in', () => {
    expect(ctaFor({ kind: 'place' } as never, false, false)).toBe('jump');
    expect(ctaFor(live, false, false)).toBe('jump');
    expect(ctaFor(soon, false, false)).toBe('remind');
    expect(ctaFor(soon, true, false)).toBe('set');
    expect(ctaFor(soon, false, true)).toBe('signIn');
  });
});

describe('mockApi', () => {
  it('serves data and applies edits', async () => {
    const api = createMockApi();
    expect((await api.social()).requests).toHaveLength(3);
    await api.acceptRequest(mockSocial.requests[0].address);
    expect((await api.social()).requests).toHaveLength(2);
    await api.sendRequest(mockSocial.nearby[0].address);
    expect((await api.social()).nearby).toHaveLength(9);
    await api.unblock(mockSocial.friends[4].address);
    await api.markAllRead();
    const result = await api.notifications();
    expect(result.status === 'ok' && unreadCount(result.items)).toBe(0);
    expect((await api.discover()).events.length).toBeGreaterThan(0);
    expect((await api.scene()).title).toContain('Genesis');
    expect(await api.captureScreenshot()).toMatch(/jpg$/);
    await expect(
      api.submitBugReport({
        category: 'chat',
        description: 'd',
        screenshots: [],
      }),
    ).resolves.toBeUndefined();
    await Promise.all([
      api.rejectRequest('x'),
      api.cancelRequest('x'),
      api.markRead('n1'),
      api.jumpToFriend('x'),
      api.setReminder('e', true),
    ]);
  });

  it('models guests and failures', async () => {
    expect(createMockApi({ guest: true }).isGuest()).toBe(true);
    expect(await createMockApi({ guest: true }).notifications()).toEqual({
      status: 'guest',
    });
    await expect(
      createMockApi({ fail: ['social'] }).social(),
    ).rejects.toThrow();
    await expect(
      createMockApi({ fail: ['discover'] }).discover(),
    ).rejects.toThrow();
    expect(
      await createMockApi({ fail: ['notifications'] }).notifications(),
    ).toEqual({
      status: 'error',
    });
  });
});
