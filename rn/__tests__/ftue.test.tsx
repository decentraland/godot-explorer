import { ReactElement } from 'react';
import { Text } from 'react-native';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { setCoreForTests } from '../src/core';
import { resetCoreEventsForTests } from '../src/core/events';
import { createMockCore } from '../src/core/mock';
import { createMockBus } from '../src/core/mock/emitter';
import {
  CameraMode,
  ControlsFtueElement,
  SceneEntryFtue,
  State,
} from '../src/core/native';
import type { HudSnapshot } from '../src/core/native';
import { ControlsOverlay, unlockAfterMs } from '../src/ftue/ControlsOverlay';
import {
  dismissControls,
  entryDecided,
  resetFtueForTests,
  showControls,
  useFtue,
} from '../src/ftue/store';
import { useFtueEntry } from '../src/ftue/useFtueEntry';
import { hudInsets } from '../src/hud/layout';
import { Hud } from '../src/hud/Hud';
import { coreHudApi, HudApi } from '../src/hud/hudApi';
import { ModalProvider, useModals } from '../src/modals/ModalProvider';
import type { ModalApi } from '../src/modals/ModalProvider';
import { useModalShowing } from '../src/modals/bridge';

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

function hud(patch: Partial<HudSnapshot> = {}): HudSnapshot {
  return {
    parcel: { x: 0, y: 0 },
    realm: 'r',
    fps: 60,
    position: { x: 0, y: 0, z: 0 },
    sceneBase: { x: -3, y: -2 },
    cameraMode: CameraMode.ThirdPerson,
    joystickVisible: true,
    ...patch,
  };
}

afterEach(() => {
  act(() => mounted.splice(0).forEach(tree => tree.unmount()));
  resetFtueForTests();
  setCoreForTests(undefined);
  resetCoreEventsForTests();
  jest.useRealTimers();
});

describe('ftue store', () => {
  function spyCore() {
    const calls: string[] = [];
    const record =
      (name: string) =>
      (...args: unknown[]) => {
        calls.push(`${name}(${args.join(',')})`);
      };
    setCoreForTests({
      ...createMockCore(),
      controlsFtueDismissed: record('dismissed'),
    });
    return calls;
  }

  it('dismissing the overlay reports what it labelled', () => {
    const calls = spyCore();
    showControls([ControlsFtueElement.Movement]);
    dismissControls(4.2);
    expect(calls).toEqual([`dismissed(${ControlsFtueElement.Movement},4.2)`]);
  });
});

describe('ControlsOverlay', () => {
  it('ignores taps during the lock, then shows the prompt and closes on a tap', () => {
    jest.useFakeTimers();
    const dismissed: number[] = [];
    const tree = mount(
      <ControlsOverlay
        elements={[ControlsFtueElement.Menu, ControlsFtueElement.Movement]}
        insets={hudInsets(metrics.insets)}
        lockSeconds={3}
        onDismiss={s => dismissed.push(s)}
        animate={false}
      />,
    );
    const press = () =>
      tree.root
        .findByProps({
          accessibilityLabel: 'Tap the screen to start exploring',
        })
        .props.onPress();
    expect(text(tree)).toContain('Character movement');
    expect(text(tree)).not.toContain('Chat');
    act(press);
    expect(dismissed).toEqual([]);
    act(() => {
      jest.advanceTimersByTime(unlockAfterMs(2, 3));
    });
    expect(text(tree)).toContain('Tap the screen to start exploring');
    act(press);
    expect(dismissed).toHaveLength(1);
  });
});

describe('modals while the controls overlay is up', () => {
  function Probe({ grab }: { grab: (api: ModalApi) => void }) {
    grab(useModals());
    return <Text>{useModalShowing() ? 'showing' : 'clear'}</Text>;
  }

  it('standard modals wait for it; an urgent one shows at once and removes it', () => {
    let modals!: ModalApi;
    const tree = mount(
      <ModalProvider>
        <Probe grab={api => (modals = api)} />
      </ModalProvider>,
    );
    act(() => showControls([ControlsFtueElement.Menu]));
    act(() =>
      modals.show({ key: 'low-memory', title: 'Low memory', buttons: [] }),
    );
    expect(text(tree)).toContain('clear');
    expect(text(tree)).not.toContain('Low memory');
    act(() =>
      modals.show({
        key: 'scene-crash',
        title: 'Crashed',
        buttons: [],
        urgent: true,
      }),
    );
    expect(text(tree)).toContain('Crashed');
    expect(text(tree)).toContain('showing');
  });
});

describe('scene entry', () => {
  function Entry() {
    useFtueEntry(true);
    const { controls, decided } = useFtue();
    return <Text>{JSON.stringify({ controls, decided })}</Text>;
  }

  function inWorldCore(answer: SceneEntryFtue) {
    const bus = createMockBus(
      new State.InWorld({ realm: 'r', parcel: { x: 0, y: 0 } }),
    );
    const blocked: boolean[] = [];
    setCoreForTests({
      ...createMockCore(),
      state: bus.state,
      addListener: bus.add,
      removeListener: bus.remove,
      ftueSceneEntered: b => {
        blocked.push(b);
        return answer;
      },
    });
    return { bus, blocked };
  }

  it('asks the core once the engine reported the scene, and shows what it answered', async () => {
    const { bus, blocked } = inWorldCore(
      new SceneEntryFtue.Controls({ elements: [ControlsFtueElement.Chat] }),
    );
    const tree = mount(<Entry />);
    await act(async () => {});
    expect(blocked).toEqual([]);
    await act(async () => bus.emit(l => l.hud(hud())));
    await act(async () => bus.emit(l => l.hud(hud())));
    expect(blocked).toHaveLength(1);
    expect(text(tree)).toContain('\\"decided\\":true');
  });
});

describe('Hud', () => {
  const quietApi: HudApi = {
    ...coreHudApi,
    subscribe: () => () => {},
    send: () => {},
    ownProfile: async () => ({
      address: '0x1',
      name: 'Me',
      tag: '',
      claimed: true,
      color: '#fff',
      own: true,
    }),
  };

  it('holds the welcome chat line until the entry is decided and the overlay closed', async () => {
    setCoreForTests(createMockCore());
    const tree = mount(<Hud api={quietApi} animate={false} />);
    await act(async () => {});
    expect(text(tree)).not.toContain('Welcome to Decentraland');
    act(() => {
      showControls([ControlsFtueElement.Menu]);
      entryDecided();
    });
    expect(text(tree)).toContain('Tap the screen');
    expect(text(tree)).not.toContain('Welcome to Decentraland');
    act(() => dismissControls(1));
    await act(async () => {});
    expect(text(tree)).toContain('Welcome to Decentraland');
  });
});
