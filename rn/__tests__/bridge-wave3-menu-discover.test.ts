import type { GodotCommand } from '../src/GodotBridge';
import type { GodotEvent } from '../src/events';
import { createMockBridge } from '../src/godot/mockBridge';
import {
  clearMockHandlers,
  registerMockHandler,
} from '../src/bridge/mockHandlers';
import { resetRequests } from '../src/bridge/request';
import { createBridgeMenuApi, GuestError } from '../src/menu/bridgeApi';
import { createMockApi as createMockMenuApi } from '../src/menu/mockApi';
import { registerMenuMockHandlers } from '../src/menu/mockHandlers';
import { parseNotification } from '../src/menu/bridgeNotifications';
import { mockSocial } from '../src/menu/mockData';
import { createBridgeDiscoverApi } from '../src/discover/bridgeApi';
import { createMockApi as createMockDiscoverApi } from '../src/discover/mockApi';
import { registerDiscoverMockHandlers } from '../src/discover/mockHandlers';
import { forgetAll } from '../src/discover/catalog';

let mockListeners: Array<(event: GodotEvent) => void> = [];
let mockBridge = createMockBridge();
const mockSent: GodotCommand[] = [];

jest.mock('../src/GodotBridge', () => ({
  godotAvailable: false,
  sendCommand: (command: GodotCommand) => {
    mockSent.push(command);
    mockBridge.sendCommand(command);
  },
  subscribe: (listener: (event: GodotEvent) => void) => {
    mockListeners.push(listener);
    return () => {
      mockListeners = mockListeners.filter(item => item !== listener);
    };
  },
}));

const methods = () =>
  mockSent.flatMap(c => (c.type === 'req' ? [c.method] : []));

beforeEach(() => {
  mockSent.length = 0;
  clearMockHandlers();
  resetRequests();
  forgetAll();
  mockBridge = createMockBridge();
  mockListeners = [];
  mockBridge.subscribe(event => mockListeners.forEach(l => l(event)));
});

describe('menu bridge api', () => {
  beforeEach(registerMenuMockHandlers);

  it('serves the discover feed from the public places and events APIs', async () => {
    const place = {
      id: 'p1',
      title: 'Real Place',
      contact_name: 'Ana',
      image: 'https://img/p1.png',
      user_count: 7,
      like_rate: 0.8,
      base_position: '10,20',
    };
    const event = {
      id: 'e1',
      name: 'Real Event',
      user_name: 'Bo',
      image: 'https://img/e1.png',
      live: true,
      total_attendees: 3,
      start_at: '2026-10-12T12:00:00Z',
      x: 1,
      y: 2,
    };
    const fetchImpl = jest.fn(async (url: string) => ({
      ok: true,
      json: async () =>
        url.includes('/events') ? { data: [event] } : { data: [place] },
    })) as unknown as typeof fetch;
    const feed = await createBridgeMenuApi(
      createMockMenuApi(),
      fetchImpl,
    ).discover();
    expect(feed.featured[0]).toMatchObject({
      title: 'Real Place',
      users: 7,
      image: 'https://img/p1.png',
    });
    expect(feed.events[0]).toMatchObject({ id: 'e1', attendees: 3 });
    expect(feed.featuredEvents).toEqual([]);
  });

  it('builds the scene header from scene.current', async () => {
    registerMockHandler('scene.current', () => ({
      title: 'Plaza',
      creator: 'Ana',
      parcel: { x: 3, y: -4 },
      realm: 'https://realm-provider-ea.decentraland.org/main',
      genesis: true,
    }));
    const scene = await createBridgeMenuApi(createMockMenuApi()).scene();
    expect(scene).toEqual({
      title: 'Plaza',
      creator: 'Ana',
      linkQuery: 'position=3,-4',
    });
  });

  it('links a world by its realm name', async () => {
    registerMockHandler('scene.current', () => ({
      title: 'Mine',
      creator: '',
      parcel: { x: 0, y: 0 },
      realm: 'https://worlds-content-server.decentraland.org/world/foo.dcl.eth',
      genesis: false,
    }));
    const scene = await createBridgeMenuApi(createMockMenuApi()).scene();
    expect(scene.linkQuery).toBe('realm=foo.dcl.eth');
  });

  it('falls back to a neutral header when scene.current fails', async () => {
    registerMockHandler('scene.current', () => {
      throw new Error('no scene');
    });
    const scene = await createBridgeMenuApi(createMockMenuApi()).scene();
    expect(scene).toEqual({
      title: 'Decentraland',
      creator: '',
      linkQuery: 'position=0,0',
    });
  });

  it('maps friends, requests and nearby from the bridge', async () => {
    const api = createBridgeMenuApi(createMockMenuApi());
    const social = await api.social();
    expect(social.friends).toHaveLength(mockSocial.friends.length);
    expect(
      social.requests.filter(r => r.direction === 'received'),
    ).toHaveLength(2);
    expect(social.nearby).toHaveLength(mockSocial.nearby.length);
    expect(social.friends[0].claimed).toBe(true);
  });

  it('accepting a request removes it server side', async () => {
    const api = createBridgeMenuApi(createMockMenuApi());
    const [first] = (await api.social()).requests;
    await api.acceptRequest(first.address);
    const after = await api.social();
    expect(
      after.requests.find(r => r.address === first.address),
    ).toBeUndefined();
    expect(methods()).toContain('friends.accept');
  });

  it('rejects when the bridge rejects', async () => {
    registerMockHandler('friends.send', () => {
      throw new Error('boom');
    });
    const api = createBridgeMenuApi(createMockMenuApi());
    await expect(api.sendRequest('0x1')).rejects.toThrow('boom');
  });

  it('parses the notification list and marks one read', async () => {
    const api = createBridgeMenuApi(createMockMenuApi());
    const result = await api.notifications();
    expect(result.status).toBe('ok');
    if (result.status !== 'ok') {
      return;
    }
    const unread = result.items.filter(n => !n.read);
    expect(unread.length).toBeGreaterThan(1);
    await api.markRead(unread[0].id);
    const after = await api.notifications();
    expect(
      after.status === 'ok' &&
        after.items.find(n => n.id === unread[0].id)?.read,
    ).toBe(true);
  });

  it('marks every unread notification read', async () => {
    const api = createBridgeMenuApi(createMockMenuApi());
    await api.markAllRead();
    const after = await api.notifications();
    expect(after.status === 'ok' && after.items.every(n => n.read)).toBe(true);
  });

  it('reports guest and error states', async () => {
    const api = createBridgeMenuApi(createMockMenuApi());
    registerMockHandler('notifications.list', () => {
      throw new Error('User not authenticated');
    });
    expect(await api.notifications()).toEqual({ status: 'guest' });
    registerMockHandler('notifications.list', () => {
      throw new Error('Fetch error');
    });
    expect(await api.notifications()).toEqual({ status: 'error' });
  });

  it('rejects social with GuestError when the engine has no social service', async () => {
    const api = createBridgeMenuApi(createMockMenuApi());
    registerMockHandler('friends.list', () => {
      throw new Error('Social service not initialized');
    });
    await expect(api.social()).rejects.toBeInstanceOf(GuestError);
  });

  it('parses service notification shapes', () => {
    expect(
      parseNotification({
        id: 'a',
        type: 'events_started',
        read: false,
        timestamp: '1700000000000',
        metadata: {
          title: 'Party',
          link: 'https://decentraland.org/jump/events?id=ev1',
          image: 'i',
        },
      }),
    ).toMatchObject({
      type: 'event',
      eventId: 'ev1',
      timestamp: 1700000000000,
      subject: 'Party',
    });
    expect(
      parseNotification({
        id: 'b',
        type: 'social_service_friendship_request',
        metadata: { sender: { name: 'Ana', profileImageUrl: 'u' } },
      }),
    ).toMatchObject({ type: 'friendship_request', subject: 'Ana', image: 'u' });
    expect(parseNotification({ type: 'x' })).toBeNull();
    expect(parseNotification({ id: 'c', type: 'item_sold' })?.type).toBe(
      'other',
    );
  });
});

describe('discover bridge api', () => {
  beforeEach(registerDiscoverMockHandlers);
  const make = () => createBridgeDiscoverApi(createMockDiscoverApi());

  it('sends favorite and vote through places.*', async () => {
    const api = make();
    await api.setFavorite('genesis', true);
    await api.setVote('genesis', 'dislike');
    await api.setVote('genesis', null);
    const reqs = mockSent.filter(c => c.type === 'req') as Array<{
      method: string;
      params: unknown;
    }>;
    expect(reqs.map(r => r.params)).toEqual([
      { place_id: 'genesis', value: true, is_world: false },
      { place_id: 'genesis', vote: 'down', is_world: false },
      { place_id: 'genesis', vote: 'none', is_world: false },
    ]);
  });

  it('reads the user state of a place from a signed fetch', async () => {
    const api = make();
    await api.banners();
    await api.setFavorite('genesis', true);
    await api.setVote('genesis', 'like');
    const detail = await api.placeDetail('genesis');
    expect(detail.favorite).toBe(true);
    expect(detail.vote).toBe('like');
  });

  it('lists favorites with a signed fetch', async () => {
    const api = make();
    await api.setFavorite('genesis', true);
    const page = await api.places('favorites', 0);
    expect(page.items.map(p => p.id)).toEqual(['genesis']);
    expect(methods()).toContain('fetch.signed');
  });

  it('keeps public rails on the base api', async () => {
    const page = await make().places('popular', 0);
    expect(page.items.length).toBeGreaterThan(0);
  });

  it('maps friends from friends.list', async () => {
    registerMenuMockHandlers();
    const friends = await make().friends();
    expect(friends).toHaveLength(mockSocial.friends.length);
    expect(friends[0]).toMatchObject({ place: null });
  });

  it('propagates failures so the sheet can roll back', async () => {
    registerMockHandler('places.favorite', () => {
      throw new Error('denied');
    });
    await expect(make().setFavorite('genesis', true)).rejects.toThrow('denied');
  });

  it('posts and deletes event attendance', async () => {
    const api = make();
    await api.setReminder('ev1', true);
    await api.setReminder('ev1', false);
    const verbs = (
      mockSent.filter(c => c.type === 'req') as Array<{
        params: { method: string };
      }>
    ).map(r => r.params.method);
    expect(verbs).toEqual(['POST', 'DELETE']);
  });
});
