import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { fromCoreProfile } from '../src/profile/coreApi';
import { resetProfileStore } from '../src/profile/store';
import { getProfileApi, mockApi, setProfileApi } from '../src/profile/api';
import {
  getProfileState,
  loadProfile,
  saveProfile,
} from '../src/profile/store';
import { createCoreProfileApi } from '../src/profile/coreApi';

jest.useFakeTimers();

async function signIn(method: 'signInGuest' | 'signInEmailVerify') {
  const core = createMockCore();
  setCoreForTests(core);
  const done =
    method === 'signInGuest'
      ? core.signInGuest()
      : core.signInEmailVerify('a@b.c', '123456');
  await jest.runAllTimersAsync();
  await done;
  return core;
}

beforeEach(() => setProfileApi(createCoreProfileApi()));
afterEach(() => {
  setProfileApi(mockApi);
  resetProfileStore();
  setCoreForTests(undefined);
});

describe('core profile mock', () => {
  it('has no name for a guest and a name for an email account', async () => {
    const guest = await signIn('signInGuest');
    expect((await guest.profile(undefined)).name).toBe('');
    const email = await signIn('signInEmailVerify');
    expect((await email.profile(undefined)).name).not.toBe('');
  });

  it('merges deployed updates and bumps the version', async () => {
    const core = await signIn('signInEmailVerify');
    const before = await core.profile(undefined);
    const next = await core.deployProfile({ description: 'new', country: '' });
    expect(next.description).toBe('new');
    expect(next.country).toBe('');
    expect(next.name).toBe(before.name);
    expect(next.version).toBe(before.version + 1);
    expect((await core.profile(undefined)).description).toBe('new');
  });

  it('lists owned names and accepts account deletion', async () => {
    const core = await signIn('signInEmailVerify');
    expect(await core.ownedNames()).toContain('Name 01');
    await expect(core.deleteAccount()).resolves.toBeUndefined();
  });
});

describe('profile view-model mapping', () => {
  it('maps the typed profile to the screen model', async () => {
    const core = await signIn('signInEmailVerify');
    const profile = fromCoreProfile(await core.profile(undefined));
    expect(profile.tag).toBe('0002');
    expect(profile.faceUrl).toBeUndefined();
    expect(profile.equipped).toEqual([]);
    expect(profile.links.length).toBeGreaterThan(0);
  });
});

describe('profile store over the core', () => {
  it('loads, saves optimistically and rolls back on failure', async () => {
    await signIn('signInEmailVerify');
    await loadProfile();
    expect(getProfileState().status).toBe('ready');
    await expect(saveProfile({ description: 'new' })).resolves.toBe(true);
    expect(getProfileState().profile?.description).toBe('new');

    const real = getProfileApi();
    setProfileApi({
      ...real,
      deployProfile: () => Promise.reject(new Error('deploy failed')),
    });
    await expect(saveProfile({ description: 'bad' })).resolves.toBe(false);
    expect(getProfileState().profile?.description).toBe('new');
  });

  it('surfaces a load error', async () => {
    setProfileApi({
      ...createCoreProfileApi(),
      getProfile: () => Promise.reject(new Error('x')),
    });
    await loadProfile();
    expect(getProfileState().status).toBe('error');
  });
});
