import { Platform } from 'react-native';
import { forgetAll, recallPlace } from '../src/discover/catalog';
import { createMockApi } from '../src/discover/mockApi';
import { createPublicApi } from '../src/discover/publicApi';

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
        ? [{ id: 'e1', name: 'Party', x: 1, y: 2 }]
        : [place('a'), place('b')],
    }),
  })) as unknown as typeof fetch;
  const api = createPublicApi(fetchImpl);

  it('maps the public services', async () => {
    expect((await api.banners())[0].place.id).toBe('a');
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

  it('fails when the service does', async () => {
    const bad = createPublicApi((async () => ({
      ok: false,
      status: 500,
    })) as unknown as typeof fetch);
    await expect(bad.banners()).rejects.toThrow();
  });
});
