import {
  createMockApi,
  getProfileApi,
  mockApi,
  setProfileApi,
} from '../src/profile/api';
import { MOCK_PROFILE } from '../src/profile/mockData';
import {
  getProfileState,
  loadProfile,
  resetProfileStore,
  saveProfile,
} from '../src/profile/store';

afterEach(() => {
  setProfileApi(mockApi);
  resetProfileStore();
});

describe('mock api', () => {
  it('serves the profile and names and merges deployed patches', async () => {
    const api = createMockApi();
    await expect(api.getNames()).resolves.toEqual([
      'Name 01',
      'Name 02',
      'Name',
    ]);
    const next = await api.deployProfile({ description: 'new' });
    expect(next.description).toBe('new');
    expect((await api.getProfile()).description).toBe('new');
    expect(api.deployed).toEqual([{ description: 'new' }]);
    expect(MOCK_PROFILE.description).not.toBe('new');
  });

  it('records copied text and can fail deploys', async () => {
    const api = createMockApi({ failDeploy: true });
    api.copyText('0xabc');
    expect(api.copied).toEqual(['0xabc']);
    await expect(api.deployProfile({})).rejects.toThrow();
  });

  it('is swappable', () => {
    const other = createMockApi();
    setProfileApi(other);
    expect(getProfileApi()).toBe(other);
  });
});

describe('profile store', () => {
  it('goes loading -> ready', async () => {
    setProfileApi(createMockApi());
    const pending = loadProfile();
    expect(getProfileState().status).toBe('loading');
    await pending;
    expect(getProfileState()).toMatchObject({ status: 'ready' });
  });

  it('reports a failed load', async () => {
    setProfileApi({
      ...createMockApi(),
      getProfile: () => Promise.reject(new Error('x')),
    });
    await loadProfile();
    expect(getProfileState().status).toBe('error');
  });

  it('shows a patch at once and keeps the deployed result', async () => {
    setProfileApi(createMockApi());
    await loadProfile();
    const saving = saveProfile({ profession: 'Artist' });
    expect(getProfileState().profile?.profession).toBe('Artist');
    await expect(saving).resolves.toBe(true);
    expect(getProfileState().profile?.profession).toBe('Artist');
  });

  it('rolls back when the deploy fails', async () => {
    setProfileApi(createMockApi({ failDeploy: true }));
    await loadProfile();
    const before = getProfileState().profile?.profession;
    await expect(saveProfile({ profession: 'Artist' })).resolves.toBe(false);
    expect(getProfileState().profile?.profession).toBe(before);
  });

  it('refuses to save before the profile is loaded', async () => {
    await expect(saveProfile({ profession: 'x' })).resolves.toBe(false);
  });
});
