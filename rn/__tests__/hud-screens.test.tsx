import { ReactElement } from 'react';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { setCoreForTests } from '../src/core';
import { resetCoreEventsForTests } from '../src/core/events';
import { createMockCore } from '../src/core/mock';
import { createMockBus } from '../src/core/mock/emitter';
import { Control, State } from '../src/core/native';
import { ChatRow } from '../src/hud/ChatRow';
import { CoachMarks } from '../src/hud/CoachMarks';
import { Hud } from '../src/hud/Hud';
import { setHudApi } from '../src/hud/activeApi';
import { HudApi, coreHudApi } from '../src/hud/hudApi';
import {
  emitIntent,
  onIntent,
  setMenuOpen,
  setUiHidden,
} from '../src/hud/hudSignals';
import { MOCK_PROFILES } from '../src/hud/mockProfiles';
import { setHudStorage, createMemoryStorage } from '../src/hud/coachMarks';
import { LoadingCover } from '../src/world/LoadingCover';
import { WorldScreen } from '../src/world/WorldScreen';
import { TIPS } from '../src/world/loadingModel';
import type { HudEvent } from '../src/hud/types';

const metrics = {
  frame: { x: 0, y: 0, width: 800, height: 360 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};
const mounted: ReactTestRenderer[] = [];

function mount(node: ReactElement): ReactTestRenderer {
  let tree!: ReactTestRenderer;
  act(() => {
    tree = create(
      <SafeAreaProvider initialMetrics={metrics}>{node}</SafeAreaProvider>,
    );
  });
  mounted.push(tree);
  return tree;
}

const text = (tree: ReactTestRenderer) => JSON.stringify(tree.toJSON());
const noop = () => {};

/** Presses the nearest pressable ancestor of the node showing `label`. */
function pressText(tree: ReactTestRenderer, label: string) {
  let node = tree.root.findAll(n => n.props.children === label)[0];
  while (node && typeof node.props.onPress !== 'function') {
    node = node.parent as typeof node;
  }
  node.props.onPress();
}
const progress = { loaded: 3, total: 584, mbps: 0.1, percent: 10 };

afterEach(() => {
  act(() => mounted.splice(0).forEach(tree => tree.unmount()));
  setMenuOpen(false);
  setUiHidden(false);
});

type Sent = Parameters<HudApi['send']>[0];

function fakeApi(events: HudEvent[] = []): {
  api: HudApi;
  sent: Sent[];
  emit: (e: HudEvent) => void;
} {
  const sent: Sent[] = [];
  const listeners = new Set<(e: HudEvent) => void>();
  const api: HudApi = {
    ...coreHudApi,
    subscribe(listener) {
      listeners.add(listener);
      events.forEach(listener);
      return () => listeners.delete(listener);
    },
    send: command => sent.push(command),
    enterWorld: noop,
    ownProfile: async () => MOCK_PROFILES[0],
    resolveProfile: async address =>
      MOCK_PROFILES.find(p => p.address === address) ?? MOCK_PROFILES[1],
    nearby: async () => MOCK_PROFILES.slice(1),
  };
  return { api, sent, emit: e => listeners.forEach(l => l(e)) };
}

const chat = (
  message: string,
  address = MOCK_PROFILES[1].address,
): HudEvent => ({
  type: 'chat',
  data: { address, message, timestamp: 1 },
});

describe('LoadingCover', () => {
  const base = { progress, onClose: noop, onReload: noop, onStartAnyway: noop };

  it('shows title, creator, tip, progress text and percent', () => {
    const tree = mount(
      <LoadingCover
        {...base}
        phase="loading"
        tip={TIPS[0]}
        place={{ title: 'Genesis Plaza', creator: 'Decentraland Foundation' }}
      />,
    );
    const out = text(tree);
    expect(out).toContain('Genesis Plaza');
    expect(out).toContain('Decentraland Foundation');
    expect(out).toContain('Tip:');
    expect(out).toContain('03/584 resources at 0.10 mb/s');
    expect(out).toContain('10%');
  });

  it('renders each of the four tip icon variants', () => {
    for (const icon of ['bulb', 'dancer', 'friends', 'chat']) {
      const tip = TIPS.find(t => t.icon === icon)!;
      const tree = mount(<LoadingCover {...base} phase="loading" tip={tip} />);
      expect(text(tree)).toContain('Tip:');
    }
  });

  it('falls back to the pre-data title and rotates tips', () => {
    jest.useFakeTimers();
    const tree = mount(<LoadingCover {...base} phase="loading" />);
    expect(text(tree)).toContain('Enjoy your Experience');
    expect(text(tree)).toContain(
      TIPS[0].key === 'TIP_EMOTES' ? 'Emote Wheel' : '',
    );
    act(() => {
      jest.advanceTimersByTime(7000);
    });
    expect(text(tree)).toContain('Add friends');
    jest.useRealTimers();
  });

  it('calls onClose from the X', () => {
    const onClose = jest.fn();
    const tree = mount(
      <LoadingCover {...base} onClose={onClose} phase="loading" />,
    );
    act(() =>
      tree.root
        .findByProps({ accessibilityLabel: 'Cancel loading' })
        .props.onPress(),
    );
    expect(onClose).toHaveBeenCalled();
  });

  it('shows the slow and failed dialogs with their actions', () => {
    const onStartAnyway = jest.fn();
    const slow = mount(
      <LoadingCover {...base} onStartAnyway={onStartAnyway} phase="slow" />,
    );
    expect(text(slow)).toContain('Loading is taking longer than expected');
    expect(text(slow)).toContain('START ANYWAY');
    const failed = mount(<LoadingCover {...base} phase="failed" />);
    expect(text(failed)).toContain('TRY AGAIN');
  });

  it('fades out and unmounts when finished', () => {
    jest.useFakeTimers();
    const onGone = jest.fn();
    const tree = mount(
      <LoadingCover {...base} onGone={onGone} phase="loading" />,
    );
    act(() => {
      tree.update(
        <SafeAreaProvider initialMetrics={metrics}>
          <LoadingCover {...base} onGone={onGone} phase="finished" />
        </SafeAreaProvider>,
      );
    });
    act(() => {
      jest.advanceTimersByTime(600);
    });
    expect(onGone).toHaveBeenCalled();
    jest.useRealTimers();
  });
});

describe('ChatRow', () => {
  const ctx = { names: ['Clara#1f0a'], myName: 'Mateo' };
  const line = {
    id: '1',
    address: MOCK_PROFILES[1].address,
    message: 'see 12,45 and https://decentraland.org',
    timestamp: 1,
  };

  it('renders sender, tag and tappable links', () => {
    const onLink = jest.fn();
    const tree = mount(
      <ChatRow
        line={line}
        profile={MOCK_PROFILES[1]}
        ctx={ctx}
        onLink={onLink}
      />,
    );
    expect(text(tree)).toContain('Clara');
    expect(text(tree)).toContain('#1f0a');
    const coord = tree.root.findAll(
      n => n.props.onPress && n.props.children === '12,45',
    )[0];
    act(() => coord.props.onPress());
    expect(onLink).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'coord', value: '12,45' }),
    );
  });

  it('shortens long text in notification mode and renders system BBCode', () => {
    const long = { ...line, message: 'a'.repeat(120) };
    expect(text(mount(<ChatRow line={long} ctx={ctx} reduce />))).toContain(
      `${'a'.repeat(80)}...`,
    );
    const system = {
      id: 's',
      address: 'system',
      message: '[color=#cfc][b]Welcome[/b][/color]',
      timestamp: 1,
    };
    const out = text(mount(<ChatRow line={system} ctx={ctx} />));
    expect(out).toContain('System');
    expect(out).toContain('Welcome');
    expect(out).not.toContain('[b]');
  });
});

describe('Hud', () => {
  beforeEach(() =>
    setHudStorage(createMemoryStorage({ 'hud.coachMarksSeen': '1' })),
  );

  it('shows incoming chat as click-through notifications that expire after 6.5 s', async () => {
    jest.useFakeTimers();
    const { api, emit } = fakeApi();
    const tree = mount(<Hud api={api} welcome={false} />);
    await act(async () => {});
    await act(async () => emit(chat('hello there')));
    expect(text(tree)).toContain('hello there');
    expect(
      tree.root.findAll(n => n.props.pointerEvents === 'none').length,
    ).toBeGreaterThan(0);
    act(() => {
      jest.advanceTimersByTime(6600);
    });
    expect(text(tree)).not.toContain('hello there');
    jest.useRealTimers();
  });

  it('opens the chat panel, sends a message and closes with a slash command', async () => {
    const { api, sent, emit } = fakeApi();
    const tree = mount(<Hud api={api} welcome={false} initialCoach={null} />);
    await act(async () => {});
    await act(async () => emit(chat('hi')));
    act(() =>
      tree.root.findByProps({ accessibilityLabel: 'Chat' }).props.onPress(),
    );
    expect(text(tree)).toContain('Nearby');
    expect(text(tree)).toContain('Write a message');
    act(() =>
      tree.root
        .findByProps({ accessibilityLabel: 'Write a message' })
        .props.onPress(),
    );
    await act(async () => {});
    const input = tree.root.findByProps({ placeholder: 'Write a message...' });
    act(() => input.props.onChangeText('hola'));
    act(() => input.props.onSubmitEditing());
    expect(sent).toContainEqual(new Control.SendChat({ text: 'hola' }));
    expect(sent).toContainEqual(new Control.ChatOpen({ open: true }));
    act(() => input.props.onChangeText('/help'));
    act(() => input.props.onSubmitEditing());
    expect(text(tree)).not.toContain('Nearby');
  });

  it('suggests mentions while typing an @', async () => {
    const { api } = fakeApi();
    const tree = mount(
      <Hud
        api={api}
        welcome={false}
        initialCoach={null}
        initialUi={{
          chat: 'writing',
          hidden: false,
          menu: false,
        }}
      />,
    );
    await act(async () => {});
    const input = tree.root.findByProps({ placeholder: 'Write a message...' });
    act(() => input.props.onChangeText('hi @cl'));
    act(() =>
      input.props.onSelectionChange({
        nativeEvent: { selection: { start: 6, end: 6 } },
      }),
    );
    expect(text(tree)).toContain('Clara');
    act(() =>
      tree.root
        .findByProps({ accessibilityLabel: 'Clara#1f0a' })
        .props.onPress(),
    );
    expect(
      tree.root.findByProps({ placeholder: 'Write a message...' }).props.value,
    ).toBe('hi @Clara#1f0a ');
  });

  it('asks before opening a tapped link', async () => {
    const { api, sent, emit } = fakeApi();
    const tree = mount(
      <Hud
        api={api}
        welcome={false}
        initialCoach={null}
        initialUi={{ chat: 'open', hidden: false, menu: false }}
      />,
    );
    await act(async () => {});
    await act(async () => emit(chat('meet at 10,20')));
    const coord = tree.root.findAll(
      n => n.props.onPress && n.props.children === '10,20',
    )[0];
    act(() => coord.props.onPress());
    expect(text(tree)).toContain('Teleport to 10,20?');
    act(() => pressText(tree, 'JUMP IN'));
    expect(sent).toContainEqual(
      new Control.Teleport({ parcel: { x: 10, y: 20 } }),
    );
  });

  it('opens the Backpack emotes when the engine reports the wheel Edit button', async () => {
    const { api, emit } = fakeApi();
    const heard: unknown[] = [];
    const off = onIntent(i => heard.push(i));
    mount(<Hud api={api} welcome={false} initialCoach={null} />);
    await act(async () => {});
    await act(async () =>
      emit({ type: 'open_backpack', data: { emotes: true } }),
    );
    off();
    expect(heard).toEqual([{ type: 'edit_emotes' }]);
  });

  it('raises intents for the menu button and shows the restore button when hidden', async () => {
    const { api, sent } = fakeApi();
    const heard: unknown[] = [];
    const off = onIntent(i => heard.push(i));
    const tree = mount(<Hud api={api} welcome={false} initialCoach={null} />);
    await act(async () => {});
    act(() =>
      tree.root.findByProps({ accessibilityLabel: 'Menu' }).props.onPress(),
    );
    expect(heard).toEqual([{ type: 'open_menu' }]);
    act(() => setUiHidden(true));
    expect(
      tree.root.findAllByProps({ accessibilityLabel: 'Chat' }),
    ).toHaveLength(0);
    expect(sent).toContainEqual(new Control.HideUi({ hidden: true }));
    act(() =>
      tree.root
        .findByProps({ accessibilityLabel: 'Show interface' })
        .props.onPress(),
    );
    expect(
      tree.root.findAllByProps({ accessibilityLabel: 'Chat' }).length,
    ).toBeGreaterThan(0);
    act(() => setMenuOpen(true));
    expect(
      tree.root.findAllByProps({ accessibilityLabel: 'Chat' }),
    ).toHaveLength(0);
    off();
    emitIntent({ type: 'edit_emotes' });
  });

  it('reports insets to the engine', async () => {
    const { api, sent } = fakeApi();
    mount(<Hud api={api} welcome={false} initialCoach={null} />);
    await act(async () => {});
    expect(sent.find(c => c.tag === 'SetHudInsets')).toBeDefined();
  });

  it('runs the coach marks once and remembers they were seen', async () => {
    const storage = createMemoryStorage();
    setHudStorage(storage);
    const { api } = fakeApi();
    const tree = mount(<Hud api={api} welcome={false} animate={false} />);
    await act(async () => {});
    expect(text(tree)).toContain('Character movement');
    act(() =>
      tree.root.findByProps({ accessibilityLabel: 'Next tip' }).props.onPress(),
    );
    expect(text(tree)).toContain('Switch camera');
    act(() =>
      tree.root.findByProps({ accessibilityLabel: 'Next tip' }).props.onPress(),
    );
    await act(async () => {});
    expect(text(tree)).not.toContain('Switch camera');
    expect(await storage.getItem('hud.coachMarksSeen')).toBe('1');
  });

  it('skips the tour when already seen', async () => {
    const { api } = fakeApi();
    const tree = mount(<Hud api={api} welcome={false} />);
    await act(async () => {});
    expect(text(tree)).not.toContain('Character movement');
  });
});

describe('pieces', () => {
  it('CoachMarks renders both steps', () => {
    const insets = { top: 16, left: 54, right: 54, bottom: 22 };
    expect(
      text(
        mount(
          <CoachMarks
            step="controls"
            insets={insets}
            onNext={noop}
            animate={false}
          />,
        ),
      ),
    ).toContain('Controls');
    expect(
      text(
        mount(
          <CoachMarks step="camera" insets={insets} onNext={noop} animate />,
        ),
      ),
    ).toContain('Switch camera');
  });
});

describe('WorldScreen', () => {
  const place = {
    id: 'p',
    title: 'Genesis Plaza',
    creator: 'Decentraland',
    image: '',
    users: 1,
    likeRate: null,
    location: '0,0',
    realm: 'r',
    x: 0,
    y: 0,
  };

  const loading = new State.Loading({
    destination: { realm: undefined, parcel: { x: 0, y: 0 } },
    progress: 0,
  });
  const bus = createMockBus(loading);

  beforeEach(() => {
    bus.setState(loading);
    resetCoreEventsForTests();
    setCoreForTests({
      ...createMockCore(),
      state: bus.state,
      addListener: bus.add,
      removeListener: bus.remove,
    });
  });
  afterEach(() => {
    setHudApi(coreHudApi);
    setCoreForTests(undefined);
    resetCoreEventsForTests();
  });

  it('shows the loading cover first', () => {
    const tree = mount(<WorldScreen place={place} onLeave={noop} />);
    expect(text(tree)).toContain('Genesis Plaza');
    expect(text(tree)).toContain('resources at');
  });

  it('follows the core: progress while loading, the HUD once in the world', async () => {
    const { api, emit } = fakeApi();
    setHudApi(api);
    const tree = mount(<WorldScreen place={place} onLeave={noop} />);
    await act(async () =>
      emit({
        type: 'loading_progress',
        data: { loaded: 412, total: 584, mbps: 3.2, percent: 71 },
      }),
    );
    expect(text(tree)).toContain('412/584 resources at 3.20 mb/s');
    expect(text(tree)).toContain('71%');
    expect(
      tree.root.findAllByProps({ accessibilityLabel: 'Chat' }),
    ).toHaveLength(0);
    await act(async () =>
      bus.setState(new State.InWorld({ realm: 'r', parcel: { x: 0, y: 0 } })),
    );
    await act(async () => {});
    expect(
      tree.root.findAllByProps({ accessibilityLabel: 'Chat' }).length,
    ).toBeGreaterThan(0);
  });

  it('offers START ANYWAY after 90 seconds', async () => {
    jest.useFakeTimers();
    const { api } = fakeApi();
    setHudApi(api);
    const tree = mount(<WorldScreen place={place} onLeave={noop} />);
    act(() => {
      jest.advanceTimersByTime(90_000);
    });
    expect(text(tree)).toContain('START ANYWAY');
    await act(async () => pressText(tree, 'START ANYWAY'));
    jest.useRealTimers();
  });
});
