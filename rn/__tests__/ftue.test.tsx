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
  TutorialStep,
} from '../src/core/native';
import type { FtueTuning, HudSnapshot } from '../src/core/native';
import { ControlsOverlay, unlockAfterMs } from '../src/ftue/ControlsOverlay';
import {
  answerWelcome,
  dismissControls,
  entryDecided,
  replayTutorial,
  resetFtueForTests,
  showControls,
  skipTutorial,
  tutorialFact,
  useFtue,
} from '../src/ftue/store';
import { MENU_OPEN_MS, reduceRun, startRun } from '../src/ftue/tutorial';
import type { Fact, Run } from '../src/ftue/tutorial';
import { useFtueEntry } from '../src/ftue/useFtueEntry';
import { hudInsets } from '../src/hud/layout';
import { Hud } from '../src/hud/Hud';
import { coreHudApi, HudApi } from '../src/hud/hudApi';
import { ModalProvider, useModals } from '../src/modals/ModalProvider';
import type { ModalApi } from '../src/modals/ModalProvider';
import { useModalShowing } from '../src/modals/bridge';

const TUNING: FtueTuning = {
  overlayLockSeconds: 3,
  overlayTapThrough: false,
  moveMeters: 1,
  cameraDegrees: 30,
  socialSeconds: 3,
  stepDelaySeconds: 2,
  chatSeconds: 3,
};

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
    cameraYaw: 0,
    cameraPitch: 0,
    cameraMode: CameraMode.ThirdPerson,
    cameraModeLocked: false,
    joystickVisible: true,
    ...patch,
  };
}

const at = (x: number, z: number, yaw = 0): HudSnapshot =>
  hud({ position: { x, y: 0, z }, cameraYaw: yaw });

function play(run: Run, ...facts: Fact[]): Run {
  return facts.reduce((r, fact) => reduceRun(r, fact, TUNING), run);
}

const onStep = (step: TutorialStep): Run => ({ ...startRun(false), step });

afterEach(() => {
  act(() => mounted.splice(0).forEach(tree => tree.unmount()));
  resetFtueForTests();
  setCoreForTests(undefined);
  resetCoreEventsForTests();
  jest.useRealTimers();
});

describe('tutorial steps', () => {
  it('walking a metre ends the move step, the next one starts after the pause', () => {
    let run = play(
      startRun(false),
      { type: 'hud', hud: at(0, 0), now: 0 },
      { type: 'hud', hud: at(0, 0.6), now: 250 },
      { type: 'hud', hud: at(50, 50), now: 500 },
    );
    expect(run.moved).toBeCloseTo(0.6);
    run = play(run, { type: 'hud', hud: at(50, 50.5), now: 750 });
    expect(run.next).toEqual({ step: TutorialStep.Camera, at: 2750 });
    expect(play(run, { type: 'tick', now: 2000 }).step).toBe(TutorialStep.Move);
    expect(play(run, { type: 'tick', now: 2750 }).step).toBe(
      TutorialStep.Camera,
    );
  });

  it('turning the camera counts the short way round', () => {
    const run = play(
      onStep(TutorialStep.Camera),
      { type: 'hud', hud: at(0, 0, 350), now: 0 },
      { type: 'hud', hud: at(0, 0, 10), now: 250 },
    );
    expect(run.rotated).toBeCloseTo(20);
    expect(
      play(run, { type: 'hud', hud: at(0, 0, 25), now: 500 }).next?.step,
    ).toBe(TutorialStep.Pinch);
  });

  it('pinch needs first and then third person, and is skipped when a scene locks the camera', () => {
    const first = hud({ cameraMode: CameraMode.FirstPerson });
    const run = play(
      onStep(TutorialStep.Pinch),
      { type: 'hud', hud: hud(), now: 0 },
      { type: 'hud', hud: first, now: 250 },
    );
    expect(run.next).toBeUndefined();
    expect(play(run, { type: 'hud', hud: hud(), now: 500 }).next?.step).toBe(
      TutorialStep.Chat,
    );

    const locked = play(
      onStep(TutorialStep.Camera),
      { type: 'hud', hud: hud({ cameraModeLocked: true }), now: 0 },
      {
        type: 'hud',
        hud: hud({ cameraModeLocked: true, cameraYaw: 40 }),
        now: 250,
      },
      { type: 'tick', now: 5000 },
    );
    expect(locked.step).toBe(TutorialStep.Chat);
  });

  it('a chat closed within the guard is asked again; later it moves on to the menu', () => {
    const opened = play(onStep(TutorialStep.Chat), {
      type: 'chat',
      open: true,
      now: 0,
    });
    expect(opened).toMatchObject({ engaged: true, guardUntil: 3000 });
    const early = play(opened, { type: 'chat', open: false, now: 1000 });
    expect(early).toMatchObject({ step: TutorialStep.Chat, engaged: false });
    const late = play(
      opened,
      { type: 'tick', now: 3000 },
      { type: 'chat', open: false, now: 3500 },
    );
    expect(late.step).toBe(TutorialStep.Menu);
  });

  it('the menu opened from the chat skips the menu step', () => {
    const run = play(
      onStep(TutorialStep.Chat),
      { type: 'chat', open: true, now: 0 },
      { type: 'tick', now: 3000 },
      { type: 'menu', open: true, social: false, now: 4000 },
      { type: 'chat', open: false, now: 4000 },
    );
    expect(run.step).toBe(TutorialStep.Social);
  });

  it('menu, social and backpack lead to the equip step; equipping and leaving completes it', () => {
    let run = play(onStep(TutorialStep.Menu), {
      type: 'menu',
      open: true,
      social: false,
      now: 0,
    });
    run = play(run, { type: 'tick', now: MENU_OPEN_MS });
    expect(run.step).toBe(TutorialStep.Social);
    run = play(run, { type: 'menu', open: true, social: true, now: 1000 });
    expect(run).toMatchObject({ engaged: true, next: { at: 4000 } });
    run = play(
      run,
      { type: 'tick', now: 4000 },
      { type: 'backpack', open: true },
    );
    expect(run.step).toBe(TutorialStep.Equip);
    expect(play(run, { type: 'reenter' }).step).toBe(TutorialStep.Backpack);
    expect(play(run, { type: 'backpack', open: false }).ended).toBe('skipped');
    expect(
      play(run, { type: 'equipped' }, { type: 'backpack', open: false }).ended,
    ).toBe('completed');
  });
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
      tutorialWelcomeAnswered: record('welcome'),
      tutorialStepEntered: record('step'),
      tutorialSkipped: record('skipped'),
      tutorialCompleted: record('completed'),
      tutorialReplayRequested: record('replay'),
      controlsFtueDismissed: record('dismissed'),
    });
    return calls;
  }

  it('START runs the steps from the first one; SKIP during a step settles it', () => {
    const calls = spyCore();
    answerWelcome(true);
    tutorialFact({ type: 'hud', hud: at(0, 0), now: 0 });
    skipTutorial();
    expect(calls).toEqual([
      'welcome(true)',
      `step(${TutorialStep.Move},false)`,
      `skipped(${TutorialStep.Move},false)`,
    ]);
  });

  it('Replay starts a run without the welcome card and says so', () => {
    const calls = spyCore();
    replayTutorial();
    expect(calls).toEqual(['replay()', `step(${TutorialStep.Move},true)`]);
  });

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
    const { controls, welcome, decided } = useFtue();
    return <Text>{JSON.stringify({ controls, welcome, decided })}</Text>;
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

  it('a welcome answer from the core shows the card', async () => {
    const { bus } = inWorldCore(new SceneEntryFtue.Tutorial());
    const tree = mount(<Entry />);
    await act(async () => bus.emit(l => l.hud(hud())));
    expect(text(tree)).toContain('\\"welcome\\":true');
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
