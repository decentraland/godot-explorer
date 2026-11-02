import { act, create } from 'react-test-renderer';
import { setCoreForTests } from '../src/core';
import type { CoreApi } from '../src/core';
import { createMockCore } from '../src/core/mock';
import {
  AccessVerdict,
  Control,
  DisconnectReason,
  EngineNotice,
  EntryRefusal,
  ResolveFailure,
  SceneRequest,
  UpdateDecision,
} from '../src/core/native';
import type { ResolvedDestination } from '../src/core/native';
import {
  ModalProvider,
  queueAfter,
  useModals,
} from '../src/modals/ModalProvider';
import type { ModalApi, ModalSpec } from '../src/modals/ModalProvider';
import {
  leaveWorld,
  noticeModal,
  sceneRequestModal,
} from '../src/modals/CoreModals';
import { confirmLeave } from '../src/app/wiring';
import { t } from '../src/i18n';
import { registerModalApi } from '../src/modals/bridge';
import {
  destinationInput,
  entryVerdict,
  mayEnter,
  passwordError,
} from '../src/modals/destination';
import type { Place } from '../src/discover/api';

const resolved = (patch: Partial<ResolvedDestination>): ResolvedDestination =>
  ({
    kind: 0,
    access: AccessVerdict.Allowed,
    ready: true,
    world: '',
    ...patch,
  } as ResolvedDestination);

const place = (patch: Partial<Place> = {}): Place => ({
  id: 'p',
  title: 'Plaza',
  creator: '',
  image: '',
  users: 0,
  likeRate: null,
  location: '1,2',
  realm: 'https://realm/main',
  x: 1,
  y: 2,
  ...patch,
});

const spec = (key: string): ModalSpec => ({ key, title: key, buttons: [] });

describe('queue', () => {
  it('shows modals in the order they were asked for', () => {
    let q: ModalSpec[] = [];
    q = queueAfter(q, spec('a'));
    q = queueAfter(q, spec('b'));
    expect(q.map(m => m.key)).toEqual(['a', 'b']);
  });

  it('replaces a modal that has the same key instead of stacking it', () => {
    let q = [spec('a'), spec('b')];
    q = queueAfter(q, { ...spec('b'), title: 'again' });
    expect(q).toHaveLength(2);
    expect(q[1].title).toBe('again');
    q = queueAfter(q, { ...spec('a'), title: 'now' });
    expect(q[0].title).toBe('now');
    expect(q).toHaveLength(2);
  });
});

describe('entry verdicts', () => {
  it('turns a refusal into the right modal and lets everything else through', () => {
    expect(entryVerdict(resolved({}))).toBe('enter');
    expect(
      entryVerdict(
        resolved({ access: AccessVerdict.NotAllowed, ready: false }),
      ),
    ).toBe('private');
    expect(
      entryVerdict(
        resolved({ access: AccessVerdict.NeedsPassword, ready: false }),
      ),
    ).toBe('password');
    for (const failure of [
      ResolveFailure.InvalidInput,
      ResolveFailure.WorldNotFound,
      ResolveFailure.ParcelOutOfBounds,
      ResolveFailure.PreviewUnreachable,
    ]) {
      expect(entryVerdict(resolved({ failure, ready: false }))).toBe('invalid');
    }
    for (const failure of [
      ResolveFailure.ParcelEmpty,
      ResolveFailure.FetchFailed,
      ResolveFailure.Timeout,
    ]) {
      expect(entryVerdict(resolved({ failure, ready: false }))).toBe('enter');
    }
  });

  it('says why a password was refused', () => {
    expect(passwordError(resolved({}))).toBeNull();
    expect(
      passwordError(
        resolved({ access: AccessVerdict.NeedsPassword, ready: false }),
      ),
    ).toMatch(/didn't work/);
    expect(
      passwordError(
        resolved({ failure: ResolveFailure.RateLimited, ready: false }),
      ),
    ).toMatch(/Too many/);
    expect(
      passwordError(
        resolved({ failure: ResolveFailure.Timeout, ready: false }),
      ),
    ).toMatch(/right now/);
  });

  it('resolves a world by its name and a parcel by its coordinates', () => {
    expect(
      destinationInput(place({ world: true, location: 'a.dcl.eth' })),
    ).toBe('a.dcl.eth');
    expect(destinationInput(place())).toBe('1,2');
  });
});

describe('scene requests', () => {
  it('ask before acting and answer once', () => {
    const answers: [number, boolean][] = [];
    const modal = sceneRequestModal(
      7,
      new SceneRequest.OpenExternalUrl({ url: 'https://x.test' }),
      (id, accepted) => answers.push([id, accepted]),
    );
    expect(modal.body).toContain('https://x.test');
    modal.buttons[1].onPress?.();
    modal.buttons[0].onPress?.();
    modal.onDismiss?.();
    expect(answers).toEqual([
      [7, true],
      [7, false],
      [7, false],
    ]);
  });

  it('name the parcel a teleport goes to', () => {
    const modal = sceneRequestModal(
      1,
      new SceneRequest.Teleport({ parcel: { x: 3, y: -4 }, realm: undefined }),
      () => {},
    );
    expect(modal.title).toContain('3,-4');
  });
});

describe('mayEnter', () => {
  const shown: ModalSpec[] = [];
  const passwords: unknown[] = [];

  function mount(core: Partial<CoreApi>) {
    setCoreForTests({ ...createMockCore(), ...core } as CoreApi);
    let api!: ModalApi;
    function Grab() {
      api = useModals();
      return null;
    }
    act(() => {
      create(
        <ModalProvider>
          <Grab />
        </ModalProvider>,
      );
    });
    const real = api.show;
    api.show = modal => {
      shown.push(modal);
      real(modal);
    };
    return api;
  }

  afterEach(() => {
    shown.length = 0;
    passwords.length = 0;
    setCoreForTests(undefined);
  });

  it('lets the player in when the core says yes or cannot say', async () => {
    mount({ resolveDestination: async () => resolved({}) });
    expect(await mayEnter(place())).toBe(true);
    mount({
      resolveDestination: async () => {
        throw new Error('offline');
      },
    });
    expect(await mayEnter(place())).toBe(true);
  });

  it('refuses a private world with a modal that names it', async () => {
    mount({
      resolveDestination: async () =>
        resolved({ access: AccessVerdict.NotAllowed, ready: false }),
    });
    expect(await mayEnter(place({ title: 'Secret' }))).toBe(false);
    expect(shown[0].key).toBe('private-world');
    expect(shown[0].title).toContain('Secret');
  });

  it('refuses a place that is gone', async () => {
    mount({
      resolveDestination: async () =>
        resolved({ failure: ResolveFailure.WorldNotFound, ready: false }),
    });
    expect(await mayEnter(place())).toBe(false);
    expect(shown[0].key).toBe('invalid-destination');
  });
});

describe('update gate decision', () => {
  it('has the three outcomes the modal switches on', () => {
    expect(UpdateDecision.Proceed).not.toBe(UpdateDecision.Required);
    expect(
      new Control.AnswerSceneRequest({ id: 1, accepted: true }).inner.accepted,
    ).toBe(true);
  });
});

describe('engine notices', () => {
  const actions = () => ({
    home: jest.fn(),
    reconnect: jest.fn(),
    signOut: jest.fn(),
    askPassword: jest.fn(),
  });

  it('say why the entry was refused and send the player back', () => {
    const a = actions();
    const priv = noticeModal(
      new EngineNotice.EntryRefused({
        reason: EntryRefusal.PrivateWorld,
        realm: 'x.dcl.eth',
      }),
      a,
    );
    expect(priv?.key).toBe('private-world');
    priv?.buttons[0].onPress?.();
    expect(a.home).toHaveBeenCalled();
    const gone = noticeModal(
      new EngineNotice.EntryRefused({
        reason: EntryRefusal.NotFound,
        realm: 'x',
      }),
      a,
    );
    expect(gone?.key).toBe('invalid-destination');
  });

  it('asks for the password instead of showing a modal', () => {
    const a = actions();
    const modal = noticeModal(
      new EngineNotice.EntryRefused({
        reason: EntryRefusal.NeedsPassword,
        realm: 'https://w/x',
      }),
      a,
    );
    expect(modal).toBeNull();
    expect(a.askPassword).toHaveBeenCalledWith('https://w/x');
  });

  it('offer a sign in after a duplicate session and a reconnect after a lost room', () => {
    const a = actions();
    const dup = noticeModal(
      new EngineNotice.Disconnected({
        reason: DisconnectReason.DuplicateSession,
      }),
      a,
    );
    expect(dup?.dismissable).toBe(false);
    dup?.buttons[1].onPress?.();
    expect(a.signOut).toHaveBeenCalled();
    const closed = noticeModal(
      new EngineNotice.Disconnected({ reason: DisconnectReason.RoomClosed }),
      a,
    );
    closed?.buttons[1].onPress?.();
    expect(a.reconnect).toHaveBeenCalled();
  });

  it('offer sign in or exit after a platform ban, with the ban copy', () => {
    const a = actions();
    const ban = noticeModal(
      new EngineNotice.Disconnected({ reason: DisconnectReason.PlatformBan }),
      a,
    );
    expect(ban?.title).toBe(t('MODAL_BAN_KICKED_TITLE'));
    expect(ban?.dismissable).toBe(false);
    expect(ban?.buttons[1].label).toBe(t('MODAL_SESSION_ENDED_SIGN_IN_PRIMARY'));
    ban?.buttons[1].onPress?.();
    expect(a.signOut).toHaveBeenCalled();
    ban?.buttons[0].onPress?.();
    expect(a.home).toHaveBeenCalled();
  });

  it('keep the outage modal under a key the recovery notice can withdraw', () => {
    const a = actions();
    const outage = noticeModal(
      new EngineNotice.Disconnected({ reason: DisconnectReason.Other }),
      a,
    );
    expect(outage?.key).toBe('disconnected-recoverable');
    expect(outage?.buttons[1].label).toBe(t('MODAL_SESSION_ENDED_RETRY_PRIMARY'));
    const closed = noticeModal(
      new EngineNotice.Disconnected({ reason: DisconnectReason.RoomClosed }),
      a,
    );
    expect(closed?.key).toBe('disconnected');
    expect(noticeModal(new EngineNotice.DisconnectRecovered(), a)).toBeNull();
  });

  it('keep a ban modal until the player leaves', () => {
    const a = actions();
    const ban = noticeModal(new EngineNotice.Banned(), a);
    expect(ban?.dismissable).toBe(false);
    expect(noticeModal(new EngineNotice.LowMemory(), a)?.buttons).toHaveLength(
      2,
    );
  });
});

describe('leaving', () => {
  afterEach(() => setCoreForTests(undefined));

  it('goes home even when the core says there is no world to leave', () => {
    const reset = jest.fn();
    setCoreForTests({
      ...createMockCore(),
      exitWorld: () => {
        throw new Error('not in a world');
      },
    } as CoreApi);
    expect(() => leaveWorld(reset)).not.toThrow();
    expect(reset).toHaveBeenCalledWith({ name: 'home' });
  });
});

describe('back from the world', () => {
  it('asks before leaving, and leaves on yes only', () => {
    const shown: ModalSpec[] = [];
    registerModalApi({
      show: modal => shown.push(modal),
      dismiss: () => {},
      askPassword: () => {},
      isOpen: () => false,
    });
    const leave = jest.fn();
    confirmLeave(leave);
    expect(leave).not.toHaveBeenCalled();
    expect(shown[0].key).toBe('leave-world');
    shown[0].buttons[0].onPress?.();
    expect(leave).not.toHaveBeenCalled();
    shown[0].buttons[1].onPress?.();
    expect(leave).toHaveBeenCalledTimes(1);
    registerModalApi(null);
  });

  it('leaves at once when no modal host is mounted', () => {
    registerModalApi(null);
    const leave = jest.fn();
    confirmLeave(leave);
    expect(leave).toHaveBeenCalledTimes(1);
  });
});
