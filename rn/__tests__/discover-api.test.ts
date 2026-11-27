import { createElement } from 'react';
import { Platform } from 'react-native';
import { act, create } from 'react-test-renderer';
import { forgetAll, recallPlace } from '../src/discover/catalog';
import { createMockApi } from '../src/discover/mockApi';
import { createPublicApi } from '../src/discover/publicApi';
import { fetchNewInDecentraland } from '../src/discover/newInDecentraland';
import { recordPlayed, usePlayed } from '../src/discover/history';
import { PLACES } from '../src/discover/mockData';

beforeEach(forgetAll);

describe('mock api', () => {
  it('serves rails and remembers what it showed', async () => {
    const api = createMockApi();
    const page = await api.places('popular', 0);
    expect(page.items.length).toBeGreaterThan(0);
    expect(recallPlace(page.items[0].id)).toBeDefined();
    expect((await api.banners()).length).toBe(3);
    expect((await api.events()).some(e => e.live)).toBe(true);
    expect((await api.places('popular', 6)).items.length).toBeGreaterThan(0);
  });

  it('keeps votes, favorites and reminders per instance', async () => {
    const api = createMockApi();
    await api.setFavorite('meta', true);
    await api.setVote('meta', 'like');
    await api.setReminder('crypto', true);
    expect(await api.placeDetail('meta')).toMatchObject({
      favorite: true,
      vote: 'like',
    });
    expect((await api.eventDetail('crypto')).reminder).toBe(true);
  });

  it('can fail, break images and return empty rails', async () => {
    await expect(
      createMockApi({ fail: ['events'] }).events(),
    ).rejects.toThrow();
    const first = (
      await createMockApi({ imageFail: true }).places('popular', 0)
    ).items[0];
    expect(first.image).toBe('mock:error');
    expect(
      (await createMockApi({ empty: ['popular'] }).places('popular', 0)).items,
    ).toEqual([]);
    expect(await createMockApi({ noFriends: true }).friends()).toEqual([]);
  });

  it('searches by text and by coordinates, and suggests', async () => {
    const api = createMockApi();
    expect((await api.search('casa')).featured.length).toBeGreaterThan(0);
    expect((await api.search('44,-48')).featured[0].title).toMatch(/Golf/);
    expect((await api.search('zzzz')).featured).toEqual([]);
    expect((await api.suggest('Ca')).map(s => s.text)).toContain('Casa');
    expect((await api.suggest('44,-48'))[0].kind).toBe('coordinates');
    expect(
      (await api.categoryPlaces('game', 0, 'casa')).items.length,
    ).toBeGreaterThan(0);
  });

  it('rejects unknown ids and never settles when hanging', async () => {
    await expect(createMockApi().placeDetail('nope')).rejects.toThrow();
    await expect(createMockApi().eventDetail('nope')).rejects.toThrow();
    let settled = false;
    createMockApi({ hang: true })
      .banners()
      .then(() => (settled = true));
    await Promise.resolve();
    expect(settled).toBe(false);
  });
});

describe('public api', () => {
  const place = (id: string, extra = {}) => ({
    id,
    title: `Place ${id}`,
    base_position: '1,2',
    ...extra,
  });
  const fetchImpl = jest.fn(async (url: string) => ({
    ok: true,
    json: async () => ({
      data: url.includes('/events')
        ? [{ id: 'e1', name: 'Party', x: 1, y: 2, approved: true }]
        : [place('a'), place('b')],
    }),
  })) as unknown as typeof fetch;
  const api = createPublicApi(fetchImpl);

  it('maps the public services', async () => {
    expect((await api.banners()).map(b => b.place.id).sort()).toEqual([
      'a',
      'b',
    ]);
    expect((await api.events())[0].id).toBe('e1');
    expect((await api.places('popular', 0)).items).toHaveLength(2);
    expect((await api.places('favorites', 0)).items).toEqual([]);
    expect(await api.friends()).toEqual([]);
    expect((await api.search('casa')).events).toHaveLength(1);
    expect((await api.search('1,2')).mostActive).toEqual([]);
    expect((await api.suggest('cas'))[0]).toMatchObject({ kind: 'keyword' });
    expect((await api.suggest('1,2'))[0].kind).toBe('coordinates');
    expect((await api.categoryPlaces('game', 0, 'x')).items).toHaveLength(2);
  });

  it('applies user state locally and resolves from the cache', async () => {
    await api.places('popular', 0);
    await api.setFavorite('a', true);
    await api.setVote('a', 'dislike');
    expect(await api.placeDetail('a')).toMatchObject({
      favorite: true,
      vote: 'dislike',
    });
    await api.events();
    await api.setReminder('e1', true);
    expect((await api.eventDetail('e1')).reminder).toBe(true);
    expect(await api.friend('0x')).toBeNull();
  });

  it('reads the ftue list from the destinations service, bare arrays too', async () => {
    const urls: string[] = [];
    const ftue = createPublicApi((async (url: string) => {
      urls.push(url);
      return {
        ok: true,
        json: async () => [
          place('solo', { world: true, world_name: 'solo.dcl.eth' }),
        ],
      };
    }) as unknown as typeof fetch);
    const found = await ftue.ftuePlaces();
    expect(found.map(p => p.location)).toEqual(['solo.dcl.eth']);
    expect(urls[0]).toMatch(
      /\/destinations\/\?offset=0&tag=ftue&sdk=7&tag=allowed_ios$/,
    );
    Platform.OS = 'android';
    await ftue.ftuePlaces();
    Platform.OS = 'ios';
    expect(urls[1]).toMatch(/\/destinations\/\?offset=0&tag=ftue&sdk=7$/);
  });

  it('swaps the featured banners for the season tag, falling back when it is empty', async () => {
    const urls: string[] = [];
    const respond = (seasonal: unknown[]) =>
      createPublicApi((async (url: string) => {
        urls.push(url);
        return {
          ok: true,
          json: async () => ({
            data: url.includes('tag=season') ? seasonal : [place('f')],
          }),
        };
      }) as unknown as typeof fetch);
    expect(
      (await respond([place('s')]).banners(true)).map(b => b.place.id),
    ).toEqual(['s']);
    expect(urls.some(u => u.includes('only_highlighted'))).toBe(false);
    expect((await respond([]).banners(true)).map(b => b.place.id)).toEqual([
      'f',
    ]);
    urls.length = 0;
    await respond([place('s')]).banners();
    expect(urls.every(u => !u.includes('tag=season'))).toBe(true);
  });

  it('lists everything through mobile-bff, tagged for its iOS block list', async () => {
    const urls: string[] = [];
    const bff = createPublicApi((async (url: string) => {
      urls.push(url);
      return { ok: true, json: async () => ({ data: [] }) };
    }) as unknown as typeof fetch);
    const run = async () => {
      urls.length = 0;
      await bff.banners();
      await bff.events();
      await bff.places('popular', 12);
      await bff.categoryPlaces('game', 0, 'casa');
      await bff.search('casa');
      await bff.search('1,-2');
      await bff.suggest('cas');
      return [...urls];
    };
    const ios = await run();
    expect(ios).toEqual([
      'https://mobile-bff.decentraland.org/destinations/?offset=0&tag=featured&sdk=7&tag=allowed_ios',
      'https://mobile-bff.decentraland.org/events/?sdk=7&tag=allowed_ios',
      'https://mobile-bff.decentraland.org/destinations/?order_by=most_active&offset=12&limit=12&sdk=7&tag=allowed_ios',
      'https://mobile-bff.decentraland.org/destinations/?categories=game&offset=0&search=casa&limit=12&sdk=7&tag=allowed_ios',
      'https://mobile-bff.decentraland.org/destinations/?offset=0&search=casa&only_highlighted=true&limit=12&sdk=7&tag=allowed_ios',
      'https://mobile-bff.decentraland.org/destinations/?offset=0&search=casa&order_by=most_active&limit=12&sdk=7&tag=allowed_ios',
      'https://mobile-bff.decentraland.org/events/?sdk=7&tag=allowed_ios&search=casa',
      'https://mobile-bff.decentraland.org/destinations/?only_places=true&pointer=1,-2&sdk=7&tag=allowed_ios',
      'https://mobile-bff.decentraland.org/destinations/?search=cas&limit=8&sdk=7&tag=allowed_ios',
    ]);
    Platform.OS = 'android';
    const android = await run();
    Platform.OS = 'ios';
    expect(android).toEqual(
      ios.map(url => url.replace('&tag=allowed_ios', '')),
    );
  });

  it('lists approved events only, live and trending first, then by next start', async () => {
    const event = (id: string, extra = {}) => ({
      id,
      name: id,
      approved: true,
      ...extra,
    });
    const listed = createPublicApi((async () => ({
      ok: true,
      json: async () => ({
        data: [
          event('later', { next_start_at: '2026-12-02T10:00:00Z' }),
          event('undated'),
          event('hidden', { approved: false, live: true, trending: true }),
          event('live', { live: true }),
          event('soon', { next_start_at: '2026-12-01T10:00:00Z' }),
          event('hot', { live: true, trending: true }),
        ],
      }),
    })) as unknown as typeof fetch);
    expect((await listed.events()).map(e => e.id)).toEqual([
      'hot',
      'live',
      'soon',
      'later',
      'undated',
    ]);
  });

  it('fails when the service does', async () => {
    const bad = createPublicApi((async () => ({
      ok: false,
      status: 500,
    })) as unknown as typeof fetch);
    await expect(bad.banners()).rejects.toThrow();
  });
});

describe('New in Decentraland', () => {
  const NOW = Date.parse('2026-11-25T12:00:00Z');
  const ago = (days: number) => new Date(NOW - days * 86400000).toISOString();
  const row = (id: string, extra: Record<string, unknown> = {}) => ({
    id,
    title: id,
    base_position: '0,0',
    ...extra,
  });

  it('joins the 7-day window of the newest with the most active deployed in it', async () => {
    const urls: string[] = [];
    // A full first page: places then worlds, each newest first, so a stale place sits mid-page.
    const first = [
      ...Array.from({ length: 60 }, (_, i) =>
        row(`p${i}`, { created_at: ago(1) }),
      ),
      row('old-place', { created_at: ago(30) }),
      ...Array.from({ length: 39 }, (_, i) =>
        row(`w${i}`, {
          world: true,
          world_name: `w${i}.dcl.eth`,
          created_at: ago(2),
        }),
      ),
    ];
    const pages: Record<string, unknown[]> = {
      'offset=0&order_by=created_at': first,
      'offset=100&order_by=created_at': [
        row('busy-new', { created_at: ago(6), user_count: 4 }),
        row('stale', { created_at: ago(8), user_count: 50 }),
      ],
      'order_by=most_active': [
        row('busy-new', {
          created_at: ago(6),
          deployed_at: ago(1),
          user_count: 4,
        }),
        row('redeployed', {
          created_at: ago(90),
          deployed_at: ago(3),
          user_count: 9,
        }),
        row('liked', {
          created_at: ago(90),
          deployed_at: ago(3),
          user_count: 4,
          like_score: 0.9,
        }),
        row('untouched', {
          created_at: ago(90),
          deployed_at: ago(40),
          user_count: 99,
        }),
      ],
    };
    const found = await fetchNewInDecentraland(async url => {
      urls.push(url);
      const key = Object.keys(pages).find(k => url.includes(k));
      return { data: key ? pages[key] : [] };
    }, NOW);
    expect(urls).toEqual([
      'https://mobile-bff.decentraland.org/destinations/?offset=0&order_by=created_at&order=desc&limit=100&sdk=7&tag=allowed_ios',
      'https://mobile-bff.decentraland.org/destinations/?offset=100&order_by=created_at&order=desc&limit=100&sdk=7&tag=allowed_ios',
      'https://mobile-bff.decentraland.org/destinations/?offset=0&order_by=most_active&limit=100&sdk=7&tag=allowed_ios',
    ]);
    const ids = found.map(p => p.id);
    expect(ids.slice(0, 3)).toEqual(['redeployed', 'liked', 'busy-new']);
    expect(ids).toHaveLength(102);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).not.toContain('old-place');
    expect(ids).not.toContain('stale');
    expect(ids).not.toContain('untouched');
    expect(ids).toContain('w38');
  });

  it('stops paging at a short page and fails with the service', async () => {
    const urls: string[] = [];
    await fetchNewInDecentraland(async url => {
      urls.push(url);
      return { data: [row('a', { created_at: ago(0) })] };
    }, NOW);
    expect(urls).toHaveLength(2);
    await expect(
      fetchNewInDecentraland(async () => {
        throw new Error('down');
      }, NOW),
    ).rejects.toThrow('down');
  });

  it('is the whole window in one page of the public api', async () => {
    const api = createPublicApi((async () => ({
      ok: true,
      json: async () => ({
        data: [row('n', { created_at: new Date().toISOString() })],
      }),
    })) as unknown as typeof fetch);
    const page = await api.places('new', 0);
    expect(page).toEqual({
      items: [expect.objectContaining({ id: 'n' })],
      hasMore: false,
    });
    expect(recallPlace('n')).toBeDefined();
  });
});

describe('Jump back in', () => {
  it('keeps the six most recent places, newest first, once each', () => {
    const place = (id: string) => ({ ...PLACES[0], id, title: id });
    ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'a'].forEach(id =>
      recordPlayed(place(id)),
    );
    let played: string[] = [];
    function Probe() {
      played = usePlayed().map(p => p.id);
      return null;
    }
    act(() => {
      create(createElement(Probe));
    });
    expect(played).toEqual(['a', 'g', 'f', 'e', 'd', 'c']);
  });
});
