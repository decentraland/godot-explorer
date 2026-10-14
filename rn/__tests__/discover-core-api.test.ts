import { setCoreForTests, type CoreApi } from '../src/core';
import { createMockPlacesWithoutNotifications } from '../src/core/mock/places';
import { Vote, type Friend } from '../src/core/native';
import { forgetAll, remember } from '../src/discover/catalog';
import { createCoreDiscoverApi } from '../src/discover/coreApi';
import { toPlace } from '../src/discover/corePlace';
import { createMockApi } from '../src/discover/mockApi';

const URLS = {
  mainRealm: 'https://realm.test/main',
  worldsContentServer: 'https://worlds.test/world/',
};

const friend = (over: Partial<Friend>): Friend => ({
  address: '0xabcdef1234',
  name: 'Ana',
  hasClaimedName: true,
  profilePictureUrl: '',
  friendshipId: 'f1',
  online: false,
  ...over,
});

let calls: Array<[string, unknown[]]> = [];
const spy = <T extends (...args: never[]) => unknown>(name: string, fn: T) =>
  ((...args: unknown[]) => {
    calls.push([name, args]);
    return fn(...(args as never[]));
  }) as unknown as T;

function install(over: Partial<CoreApi> = {}) {
  const places = createMockPlacesWithoutNotifications();
  const core = {
    ...places,
    urls: () => URLS,
    friends: async () => [
      friend({}),
      friend({ address: '0x9999zzzz', name: '', hasClaimedName: false }),
    ],
    setFavorite: spy('setFavorite', places.setFavorite),
    vote: spy('vote', places.vote),
    setEventReminder: spy('setEventReminder', places.setEventReminder),
    ...over,
  } as unknown as CoreApi;
  setCoreForTests(core);
  return createCoreDiscoverApi(createMockApi());
}

beforeEach(() => {
  calls = [];
  forgetAll();
});
afterEach(() => setCoreForTests(undefined));

describe('discover core api', () => {
  it('sends favorite and vote to the core', async () => {
    const api = install();
    await api.setFavorite('genesis', true);
    await api.setVote('genesis', 'dislike');
    await api.setVote('genesis', null);
    expect(calls).toEqual([
      ['setFavorite', ['genesis', true, false]],
      ['vote', ['genesis', Vote.Down, false]],
      ['vote', ['genesis', Vote.None, false]],
    ]);
  });

  it('flags worlds it has already shown', async () => {
    const api = install();
    const [world] = await api.places('popular', 0).then(page => page.items);
    remember({ places: [{ ...world, id: 'world', world: true }] });
    await api.setFavorite('world', true);
    expect(calls[0]).toEqual(['setFavorite', ['world', true, true]]);
  });

  it('reads the user state of a place', async () => {
    const api = install();
    await api.setFavorite('genesis', true);
    await api.setVote('genesis', 'like');
    expect(await api.placeDetail('genesis')).toMatchObject({
      favorite: true,
      vote: 'like',
    });
  });

  it('keeps the base detail when the service does not know the place', async () => {
    const api = install({ placeUserState: async () => undefined });
    expect((await api.placeDetail('genesis')).favorite).toBe(false);
    const failing = install({
      placeUserState: async () => {
        throw new Error('offline');
      },
    });
    expect((await failing.placeDetail('genesis')).place.id).toBe('genesis');
  });

  it('lists and searches favorites', async () => {
    const api = install();
    await api.setFavorite('genesis', true);
    await api.setFavorite('meta', true);
    const page = await api.places('favorites', 0);
    expect(page.items.map(p => p.id)).toEqual(['genesis', 'meta']);
    expect(page.hasMore).toBe(false);
    const found = await api.search('meta');
    expect(found.favorites.map(p => p.id)).toEqual(['meta']);
  });

  it('keeps public rails on the base api', async () => {
    const page = await install().places('popular', 0);
    expect(page.items.length).toBeGreaterThan(0);
  });

  it('maps friends, tagging unclaimed names', async () => {
    const friends = await install().friends();
    expect(friends).toMatchObject([
      { address: '0xabcdef1234', name: 'Ana', tag: undefined },
      { name: '0x9999', tag: '#zzzz' },
    ]);
    expect((await install().friend('0x9999zzzz'))?.name).toBe('0x9999');
  });

  it('sets and clears event reminders', async () => {
    const api = install();
    await api.setReminder('ev1', true);
    await api.setReminder('ev1', false);
    expect(calls).toEqual([
      ['setEventReminder', ['ev1', true]],
      ['setEventReminder', ['ev1', false]],
    ]);
  });

  it('propagates failures so the sheet can roll back', async () => {
    const api = install({
      setFavorite: async () => {
        throw new Error('denied');
      },
    });
    await expect(api.setFavorite('genesis', true)).rejects.toThrow('denied');
  });
});

describe('toPlace', () => {
  const base = {
    id: 'p',
    title: 'T',
    creator: 'C',
    image: 'i',
    users: 3,
    world: false,
    worldName: '',
    basePosition: '-12,40',
    categories: [],
  };

  it('maps a parcel onto the main realm', () => {
    install();
    expect(toPlace({ ...base, likeRate: 0.5 })).toMatchObject({
      location: '-12,40',
      realm: URLS.mainRealm,
      x: -12,
      y: 40,
      likeRate: 0.5,
      categories: undefined,
    });
  });

  it('maps a world onto its realm from the core urls', () => {
    install();
    expect(
      toPlace({ ...base, world: true, worldName: 'Foo.dcl.eth' }),
    ).toMatchObject({
      location: 'Foo.dcl.eth',
      realm: 'https://worlds.test/world/foo.dcl.eth',
      x: 0,
      likeRate: null,
    });
  });
});

describe('places mock', () => {
  it('serves the credits balance and ignores unknown places', async () => {
    const mock = createMockPlacesWithoutNotifications();
    expect((await mock.creditsBalance()).credits).toBeGreaterThan(0n);
    expect(await mock.placeUserState('nope')).toBeUndefined();
  });
});
