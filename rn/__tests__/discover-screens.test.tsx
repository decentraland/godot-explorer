import { ReactElement } from 'react';
import { TextInput } from 'react-native';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { NavProvider, useNav, Nav } from '../src/app/NavProvider';
import { getScreen } from '../src/app/registry';
import { ApiProvider } from '../src/discover/ApiContext';
import { DiscoverScreen } from '../src/discover/DiscoverScreen';
import { forgetAll } from '../src/discover/catalog';
import { setFlags } from '../src/discover/flags';
import { ID } from '../src/discover/galleryIds';
import { createMockApi, MockOptions } from '../src/discover/mockApi';
import { PLACES } from '../src/discover/mockData';
import { StoryScreen } from '../src/discover/StoryScreen';
import { getPreview } from '../src/gallery/registry';
import '../src/discover/gallery';
import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';

const metrics = {
  frame: { x: 0, y: 0, width: 360, height: 800 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

let nav!: Nav;
function Probe() {
  nav = useNav();
  return null;
}

async function mount(node: ReactElement, options?: MockOptions) {
  let tree!: ReactTestRenderer;
  await act(async () => {
    tree = create(
      <SafeAreaProvider initialMetrics={metrics}>
        <ApiProvider api={createMockApi(options)}>
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

const text = (tree: ReactTestRenderer) => JSON.stringify(tree.toJSON());
const noop = () => {};

beforeEach(() => {
  forgetAll();
  jest.useFakeTimers();
});
afterEach(() => jest.useRealTimers());

beforeEach(() => setCoreForTests(createMockCore({ signedIn: true })));
afterEach(() => setCoreForTests(undefined));

describe('every Discover preview renders', () => {
  const ids = Object.values(ID).filter(id => getPreview(id));
  it('registers the designed frames', () =>
    expect(ids.length).toBeGreaterThan(70));
  it.each(ids)('%s', async id => {
    const tree = await mount(getPreview(id)!.render());
    await act(async () => {
      jest.advanceTimersByTime(1000);
    });
    expect(tree.toJSON()).not.toBeNull();
  });
});

describe('DiscoverScreen', () => {
  it('shows the rails once loaded', async () => {
    const tree = await mount(
      <DiscoverScreen onJumpIn={noop} played={[PLACES[0]]} />,
    );
    for (const title of [
      'Jump back in',
      'Popular',
      'Friends',
      'Events',
      'Favorites',
      'My Places',
    ]) {
      expect(text(tree)).toContain(title);
    }
    expect(text(tree)).toContain('Butterfly Prawn Farming');
  });

  it('shows connection lost with a retry that reloads', async () => {
    const tree = await mount(<DiscoverScreen onJumpIn={noop} />, {
      fail: ['banners', 'events', 'places'],
    });
    expect(text(tree)).toContain('Connection lost');
    const retry = tree.root.findByProps({ accessibilityLabel: 'TRY AGAIN' });
    await act(async () => retry.props.onPress());
    expect(text(tree)).toContain('Connection lost');
  });

  it('opens a place sheet route from a card', async () => {
    const tree = await mount(<DiscoverScreen onJumpIn={noop} played={[]} />);
    const card = tree.root.findAllByProps({
      accessibilityLabel: 'Butterfly Prawn Farming',
    })[0];
    await act(async () => card.props.onPress());
    expect(nav.route).toEqual({
      name: 'placeDetail',
      params: { placeId: 'butterfly' },
    });
  });

  it('walks the search flow to results and back', async () => {
    const tree = await mount(<DiscoverScreen onJumpIn={noop} played={[]} />);
    await act(async () =>
      tree.root.findByProps({ accessibilityLabel: 'Search' }).props.onPress(),
    );
    const input = () => tree.root.findByType(TextInput);
    await act(async () => input().props.onChangeText('Casa'));
    await act(async () => {
      jest.advanceTimersByTime(500);
    });
    expect(text(tree)).toContain('Casa Rousto');
    await act(async () => input().props.onSubmitEditing());
    await act(async () => {});
    expect(text(tree)).toContain('Featured');
    await act(async () => input().props.onChangeText('zzzz'));
    await act(async () => input().props.onSubmitEditing());
    await act(async () => {});
    expect(text(tree)).toContain('No result found');
    await act(async () =>
      tree.root.findByProps({ accessibilityLabel: 'Back' }).props.onPress(),
    );
    expect(text(tree)).toContain('Discover');
  });

  it('opens a category and searches inside it', async () => {
    const tree = await mount(<DiscoverScreen onJumpIn={noop} played={[]} />);
    await act(async () =>
      tree.root.findByProps({ accessibilityLabel: 'GAME' }).props.onPress(),
    );
    expect(text(tree)).toContain('Game');
    await act(async () =>
      tree.root.findByProps({ accessibilityLabel: 'Search' }).props.onPress(),
    );
    await act(async () =>
      tree.root.findByType(TextInput).props.onChangeText('casa'),
    );
    await act(async () => {
      jest.advanceTimersByTime(500);
    });
    expect(text(tree)).toContain('Casa');
  });
});

describe('sheet routes', () => {
  it('registers the three sheets', () => {
    for (const name of [
      'placeDetail',
      'eventDetail',
      'friendDetail',
    ] as const) {
      expect(getScreen(name)).toBeDefined();
    }
  });

  it('place sheet loads, toggles state and jumps in', async () => {
    const Place = getScreen('placeDetail')!;
    const tree = await mount(<Place params={{ placeId: 'meta' } as never} />);
    expect(text(tree)).toContain('Meta Residence Tower');
    await act(async () =>
      tree.root.findByProps({ accessibilityLabel: 'Like' }).props.onPress(),
    );
    expect(
      tree.root.findByProps({ accessibilityLabel: 'Like' }).props
        .accessibilityState.selected,
    ).toBe(true);
    await act(async () =>
      tree.root.findByProps({ accessibilityLabel: 'Favorite' }).props.onPress(),
    );
    await act(async () =>
      tree.root.findByProps({ accessibilityLabel: 'JUMP IN' }).props.onPress(),
    );
    expect(nav.route.name).toBe('world');
  });

  it('event sheet toggles the reminder for upcoming events', async () => {
    const Event = getScreen('eventDetail')!;
    const tree = await mount(<Event params={{ eventId: 'crypto' } as never} />);
    expect(text(tree)).toContain('REMIND ME');
    await act(async () =>
      tree.root
        .findByProps({ accessibilityLabel: 'REMIND ME' })
        .props.onPress(),
    );
    expect(text(tree)).toContain('REMINDER SET');
  });

  it('event sheet jumps in when live; friend sheet jumps to their place', async () => {
    const Event = getScreen('eventDetail')!;
    const live = await mount(<Event params={{ eventId: 'chiri' } as never} />);
    expect(text(live)).toContain('JUMP IN');
    const Friend = getScreen('friendDetail')!;
    const friend = await mount(<Friend params={{ address: '0x0' } as never} />);
    await act(async () =>
      friend.root
        .findByProps({ accessibilityLabel: 'JUMP IN' })
        .props.onPress(),
    );
    expect(nav.route.name).toBe('world');
  });

  it('closes itself when the item cannot load', async () => {
    const Place = getScreen('placeDetail')!;
    await mount(<Place params={{ placeId: 'missing' } as never} />);
    expect(nav.state.stack.length).toBe(1);
  });
});

describe('StoryScreen flag', () => {
  it('renders nothing while the flag is off, the stub when on', async () => {
    const off = await mount(
      <StoryScreen place={PLACES[0]} onBack={noop} onJump={noop} />,
    );
    expect(text(off)).not.toContain('Genesis Plaza');
    setFlags({ storyViewer: true });
    const on = await mount(
      <StoryScreen place={PLACES[0]} onBack={noop} onJump={noop} />,
    );
    expect(text(on)).toContain('Genesis Plaza');
    setFlags({ storyViewer: false });
  });
});
