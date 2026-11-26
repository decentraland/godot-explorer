import { ReactElement } from 'react';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { NavProvider, Nav, useNav } from '../src/app/NavProvider';
import { getScreen } from '../src/app/registry';
import type { Route } from '../src/app/routes';
import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import type { Property } from '../src/core/native';
import { ApiProvider } from '../src/discover/ApiContext';
import { forgetAll } from '../src/discover/catalog';
import { DiscoverScreen } from '../src/discover/DiscoverScreen';
import { discoverCarousels, HomeRails } from '../src/discover/metrics';
import { createMockApi } from '../src/discover/mockApi';
import { BANNERS, EVENTS, FRIENDS, PLACES } from '../src/discover/mockData';
import '../src/discover/routes';

const metrics = {
  frame: { x: 0, y: 0, width: 360, height: 800 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

const trackScreen = jest.fn();
const trackClick = jest.fn();
const trackDiscover = jest.fn();

let nav!: Nav;
function Probe() {
  nav = useNav();
  return null;
}

async function mount(node: ReactElement) {
  let tree!: ReactTestRenderer;
  await act(async () => {
    tree = create(
      <SafeAreaProvider initialMetrics={metrics}>
        <ApiProvider api={createMockApi()}>
          <NavProvider>
            <Probe />
            {node}
          </NavProvider>
        </ApiProvider>
      </SafeAreaProvider>,
    );
  });
  await act(async () => {});
  return tree;
}

// The typed properties as the JSON object the core sends in `extra_properties`.
const json = (props: Property[]) =>
  Object.fromEntries(
    props.map(({ key, value }) => {
      const inner = (value as { inner?: { value: unknown } }).inner;
      const raw = inner ? inner.value : null;
      return [key, typeof raw === 'bigint' ? Number(raw) : raw];
    }),
  );

const press = async (tree: ReactTestRenderer, label: string) =>
  act(async () =>
    tree.root.findAllByProps({ accessibilityLabel: label })[0].props.onPress(),
  );

beforeEach(() => {
  forgetAll();
  jest.useFakeTimers();
  [trackScreen, trackClick, trackDiscover].forEach(mock => mock.mockClear());
  setCoreForTests({
    ...createMockCore({ signedIn: true }),
    trackScreen,
    trackClick,
    trackDiscover,
  });
});
afterEach(() => {
  jest.useRealTimers();
  setCoreForTests(undefined);
});

describe('DISCOVER snapshot', () => {
  const ready = <T,>(items: T[]) => ({
    status: 'ready' as const,
    items,
    hasMore: false,
  });
  const rails = (over: Partial<HomeRails> = {}): HomeRails => ({
    banners: ready(BANNERS),
    played: [PLACES[0]],
    popular: ready(PLACES.slice(3, 5)),
    friends: ready([{ ...FRIENDS[0], place: null }, FRIENDS[1]]),
    fresh: ready([]),
    events: ready(EVENTS.slice(0, 1)),
    favorites: { status: 'error' },
    mine: { status: 'loading' },
    ...over,
  });

  it('lists every rail with items under its 1.x key, in slot order', () => {
    const carousels = discoverCarousels(rails());
    expect(carousels.map(c => c.key)).toEqual([
      'featured',
      'last_visited',
      'most_active',
      'friends',
      'events',
    ]);
    expect(carousels[0].items).toEqual(
      BANNERS.map(b => ({ id: b.place.id, world: b.place.world === true })),
    );
    expect(carousels[2].items.map(i => i.id)).toEqual([
      PLACES[3].id,
      PLACES[4].id,
    ]);
    // A friend's slot is the place they are in, empty when they are in none.
    expect(carousels[3].items).toEqual([
      { id: '', world: false },
      { id: FRIENDS[1].place!.id, world: FRIENDS[1].place!.world === true },
    ]);
    expect(carousels[4].items[0].id).toBe(EVENTS[0].id);
  });

  it('is sent from the home once its rails have settled', async () => {
    await mount(<DiscoverScreen onJumpIn={() => {}} played={[PLACES[0]]} />);
    expect(trackDiscover).toHaveBeenCalled();
    const [location, carousels] = trackDiscover.mock.calls.at(-1);
    expect(location).toBe('pre_game');
    expect(carousels.map((c: { key: string }) => c.key)).toEqual([
      'featured',
      'last_visited',
      'most_active',
      'friends',
      'new_in_decentraland',
      'events',
      'favorites',
      'my_places',
    ]);
  });
});

describe('PLACE_DETAILS and PLACE_DETAIL_CLICK', () => {
  it('carry the rail and slot a card was opened from', async () => {
    const home = await mount(
      <DiscoverScreen onJumpIn={() => {}} played={[]} />,
    );
    await press(home, PLACES[4].title);
    const route = nav.route as Extract<Route, { name: 'placeDetail' }>;
    expect(route.params).toEqual({
      placeId: PLACES[4].id,
      origin: { carousel: 'most_active', position: 1 },
    });

    const Sheet = getScreen('placeDetail')!;
    const sheet = await mount(<Sheet params={route.params as never} />);
    const views = trackScreen.mock.calls.filter(c => c[0] === 'PLACE_DETAILS');
    expect(views).toHaveLength(1);
    expect(json(views[0][1])).toEqual({
      place_id: PLACES[4].id,
      place_title: PLACES[4].title,
      orientation: 'portrait',
      carousel: 'most_active',
      position: 1,
    });

    await press(sheet, 'Share');
    await press(sheet, 'JUMP IN');
    expect(trackClick.mock.calls.map(c => [c[0], c[1], json(c[2])])).toEqual(
      ['SHARE', 'JUMP_IN'].map(button => [
        button,
        'PLACE_DETAIL_CLICK',
        {
          place_id: PLACES[4].id,
          carousel: 'most_active',
          position: 1,
        },
      ]),
    );
  });

  it('logs a sheet a link opened as deeplink with no slot', async () => {
    const Sheet = getScreen('placeDetail')!;
    await mount(<Sheet params={{ placeId: 'meta' } as never} />);
    const view = trackScreen.mock.calls.find(c => c[0] === 'PLACE_DETAILS')!;
    expect(json(view[1])).toMatchObject({
      place_id: 'meta',
      carousel: 'deeplink',
      position: null,
    });
  });

  it('logs a friend sheet under friends with the place they are in', async () => {
    const home = await mount(
      <DiscoverScreen onJumpIn={() => {}} played={[]} />,
    );
    await press(home, FRIENDS[1].name);
    const route = nav.route as Extract<Route, { name: 'friendDetail' }>;
    expect(route.params.origin).toEqual({ carousel: 'friends', position: 1 });
    const Sheet = getScreen('friendDetail')!;
    const sheet = await mount(<Sheet params={route.params as never} />);
    const view = trackScreen.mock.calls.find(c => c[0] === 'PLACE_DETAILS')!;
    expect(json(view[1])).toMatchObject({
      place_id: FRIENDS[1].place!.id,
      carousel: 'friends',
      position: 1,
    });
    await press(sheet, 'JUMP IN');
    expect(json(trackClick.mock.calls[0][2])).toEqual({
      place_id: FRIENDS[1].place!.id,
      carousel: 'friends',
      position: 1,
    });
  });
});
