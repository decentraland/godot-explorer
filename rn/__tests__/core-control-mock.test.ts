import { createMockBus } from '../src/core/mock/emitter';
import { createMockControl } from '../src/core/mock/control';
import { createMockScene } from '../src/core/mock/scene';
import { Control, State } from '../src/core/native';
import type { AppListener, ChatLine, HudSnapshot } from '../src/core/native';

const nothing = () => {};
const silent: AppListener = {
  stateChanged: nothing,
  sceneCrashed: nothing,
  authError: nothing,
  openUrl: nothing,
  friendsChanged: nothing,
  settingsChanged: nothing,
  hud: nothing,
  chat: nothing,
  unreadChat: nothing,
  uiState: nothing,
  openBackpack: nothing,
  loadingDetail: nothing,
  avatarChanged: nothing,
};

function setup(initial: State = new State.SignedIn()) {
  const bus = createMockBus(initial);
  const control = createMockControl(bus);
  const scene = createMockScene(bus);
  const states: string[] = [];
  const chat: ChatLine[] = [];
  const hud: HudSnapshot[] = [];
  const unread: number[] = [];
  const progress: number[] = [];
  const ui: boolean[] = [];
  bus.add({
    ...silent,
    stateChanged: s => states.push(s.tag),
    chat: line => chat.push(line),
    hud: snapshot => hud.push(snapshot),
    unreadChat: count => unread.push(count),
    loadingDetail: detail => progress.push(detail.percent),
    uiState: state => ui.push(state.hidden),
  });
  return { bus, control, scene, states, chat, hud, unread, progress, ui };
}

const destination = { realm: undefined, parcel: { x: 12, y: 45 } };

beforeEach(() => jest.useFakeTimers());
afterEach(() => jest.useRealTimers());

describe('mock control: worlds', () => {
  it('loads a world through Loading and into InWorld, with progress up to 100%', () => {
    const { bus, control, states, progress } = setup();
    control.enterWorld(destination);
    expect(bus.state().tag).toBe('Loading');
    jest.advanceTimersByTime(2000);
    expect(bus.state()).toMatchObject({
      tag: 'InWorld',
      inner: { realm: 'mock', parcel: { x: 12, y: 45 } },
    });
    expect(states[0]).toBe('Loading');
    expect(states[states.length - 1]).toBe('InWorld');
    expect(progress).toHaveLength(8);
    expect(progress[7]).toBe(100);
    control.exitWorld();
  });

  it('reports the player a few times a second once inside', () => {
    const { control, hud } = setup();
    control.enterWorld(destination);
    jest.advanceTimersByTime(1200);
    expect(hud).toHaveLength(0);
    jest.advanceTimersByTime(1000);
    expect(hud.length).toBeGreaterThanOrEqual(3);
    expect(hud[0].parcel).toEqual({ x: 12, y: 45 });
    control.exitWorld();
  });

  it('teleports inside the world and ignores the command outside one', () => {
    const { control, hud } = setup();
    control.control(new Control.Teleport({ parcel: { x: 1, y: 1 } }));
    expect(hud).toHaveLength(0);
    control.enterWorld(destination);
    jest.advanceTimersByTime(1300);
    control.control(new Control.Teleport({ parcel: { x: 3, y: 4 } }));
    expect(hud[hud.length - 1].parcel).toEqual({ x: 3, y: 4 });
    control.exitWorld();
  });

  it('replaces the destination when entering again while loading', () => {
    const { bus, control } = setup();
    control.enterWorld(destination);
    jest.advanceTimersByTime(300);
    control.enterWorld({ realm: 'https://w/x', parcel: undefined });
    jest.advanceTimersByTime(2000);
    expect(bus.state()).toMatchObject({
      tag: 'InWorld',
      inner: { realm: 'https://w/x' },
    });
    control.exitWorld();
  });

  it('leaves the world, stops its events and keeps the session', () => {
    const { bus, control, hud } = setup();
    control.enterWorld(destination);
    jest.advanceTimersByTime(2000);
    control.exitWorld();
    const seen = hud.length;
    jest.advanceTimersByTime(5000);
    expect(hud).toHaveLength(seen);
    expect(bus.state().tag).toBe('SignedIn');
  });

  it('refuses what the core refuses', () => {
    const signedOut = setup(new State.SignedOut());
    expect(() => signedOut.control.enterWorld(destination)).toThrow();
    const idle = setup();
    expect(() => idle.control.exitWorld()).toThrow();
    expect(() => idle.control.hideAvatarPreview()).toThrow();
  });

  it('shows and hides the avatar preview', () => {
    const { bus, control } = setup();
    const rect = { x: 0, y: 0, width: 10, height: 10 };
    control.showAvatarPreview(rect);
    expect(bus.state()).toMatchObject({
      tag: 'AvatarPreview',
      inner: { rect },
    });
    control.hideAvatarPreview();
    expect(bus.state().tag).toBe('SignedIn');
  });
});

describe('mock control: chat and UI', () => {
  it('plays the chat script, counting unread while the chat is closed', () => {
    const { control, chat, unread } = setup();
    control.enterWorld(destination);
    jest.advanceTimersByTime(1600 + 2500 * 3);
    expect(chat).toHaveLength(4);
    expect(chat[0].profile?.name).toBe('Clara');
    expect(unread).toEqual([1, 2, 3, 4]);
    control.control(new Control.ChatOpen({ open: true }));
    expect(unread[unread.length - 1]).toBe(0);
    control.exitWorld();
  });

  it('echoes a sent line without counting it unread', () => {
    const { control, chat, unread } = setup();
    control.enterWorld(destination);
    jest.advanceTimersByTime(1300);
    control.control(new Control.SendChat({ text: 'hola' }));
    expect(chat[0].message).toBe('hola');
    expect(chat[0].profile).toBeUndefined();
    expect(unread).toEqual([]);
    control.exitWorld();
  });

  it('mirrors the UI visibility', () => {
    const { control, ui } = setup();
    control.control(new Control.HideUi({ hidden: true }));
    control.control(new Control.HideUi({ hidden: false }));
    expect(ui).toEqual([true, false]);
  });
});

describe('mock scene', () => {
  it('describes Genesis Plaza at rest and the world once inside', async () => {
    const { control, scene } = setup();
    expect(await scene.currentScene()).toMatchObject({
      title: 'Genesis Plaza',
      genesis: true,
    });
    control.enterWorld({ realm: 'https://w/x', parcel: { x: 2, y: 3 } });
    jest.advanceTimersByTime(2000);
    expect(await scene.currentScene()).toMatchObject({
      realm: 'https://w/x',
      parcel: { x: 2, y: 3 },
      genesis: false,
    });
    control.exitWorld();
  });

  it('answers the screenshot and takes analytics without effect', async () => {
    const { scene } = setup();
    expect(await scene.captureScreenshot()).toMatch(/\.png$/);
    scene.track('x', []);
    scene.trackScreen('y', []);
    await scene.setAnalyticsConsent(true);
    await scene.flushAnalytics();
    scene.setAnalyticsFlushInterval(2);
  });
});
