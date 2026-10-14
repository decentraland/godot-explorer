import { core, setCoreForTests } from '../src/core';
import { onCoreEvents, resetCoreEventsForTests } from '../src/core/events';
import { createMockCore } from '../src/core/mock';
import { State } from '../src/core/native';

beforeEach(() => {
  jest.useFakeTimers();
  setCoreForTests(createMockCore());
  resetCoreEventsForTests();
});

afterEach(() => {
  jest.useRealTimers();
  setCoreForTests(undefined);
});

describe('mock core', () => {
  it('falls back to the mock when the native module is absent', () => {
    setCoreForTests(undefined);
    expect(core().state().tag).toBe('SignedOut');
  });

  it('signs in as a guest through the states a shell renders', async () => {
    const seen: string[] = [];
    onCoreEvents({ stateChanged: (s: State) => seen.push(s.tag) });
    const done = core().signInGuest();
    jest.runAllTimers();
    await done;
    expect(seen).toEqual(['SigningIn', 'SignedIn']);
    expect(core().address()).toMatch(/^0x/);
  });

  it('keeps the code step on a wrong email code and signs in with the right one', async () => {
    const errors: string[] = [];
    onCoreEvents({ authError: (code: string) => errors.push(code) });
    const start = core().signInEmailStart('a@b.c');
    jest.runAllTimers();
    await start;
    expect(core().state().tag).toBe('SigningIn');

    const wrong = core().signInEmailVerify('a@b.c', '000000');
    jest.runAllTimers();
    await expect(wrong).rejects.toMatchObject({
      inner: { code: 'invalid_code' },
    });
    expect(errors).toEqual(['invalid_code']);

    const right = core().signInEmailVerify('a@b.c', '123456');
    jest.runAllTimers();
    await right;
    expect(core().state().tag).toBe('SignedIn');
  });

  it('refuses a second sign-in while signed in and forgets the account on sign out', async () => {
    const guest = core().signInGuest();
    jest.runAllTimers();
    await guest;
    await expect(core().signInGuest()).rejects.toBeDefined();
    core().signOut();
    expect(core().state().tag).toBe('SignedOut');
    expect(core().address()).toBeUndefined();
  });

  it('removes a listener', async () => {
    const seen: string[] = [];
    const id = core().addListener({
      stateChanged: s => seen.push(s.tag),
      sceneCrashed: () => {},
      authError: () => {},
      openUrl: () => {},
      friendsChanged: () => {},
      settingsChanged: () => {},
      hud: () => {},
      chat: () => {},
      unreadChat: () => {},
      uiState: () => {},
      openBackpack: () => {},
      loadingDetail: () => {},
      avatarChanged: () => {},
      report: () => {},
      reportTag: () => {},
    });
    core().removeListener(id);
    core().signOut();
    expect(seen).toEqual([]);
  });

  it('hands out state values the generated bindings define', () => {
    expect(new State.SignedIn().tag).toBe('SignedIn');
  });
});
