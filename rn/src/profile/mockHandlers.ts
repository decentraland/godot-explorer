import { registerMockHandler } from '../bridge/mockHandlers';
import type { ProfileData, ProfileFields } from '../bridge/methods';

const MOCK_DATA: ProfileData = {
  address: '0x4e1a5c5eb2A8a7aA7B0a09E04f1b2a1c13B',
  name: 'Namemaxcharacte',
  has_claimed_name: false,
  description: 'Hello from the mock bridge.',
  country: 'Argentina',
  links: [{ title: 'Web', url: 'https://decentraland.org' }],
};

/** Fakes profile.* on the mock bridge; `failDeploy` makes deploy reject. */
export function registerProfileMockHandlers(
  options: { data?: Partial<ProfileData>; failDeploy?: boolean } = {},
): { data: ProfileData } {
  const state = { data: { ...MOCK_DATA, ...options.data } };
  registerMockHandler('profile.get', () => ({ ...state.data }));
  registerMockHandler('profile.names', () => ({ names: ['Name 01', 'Name'] }));
  registerMockHandler('profile.deploy', (p: { fields: ProfileFields }) => {
    if (options.failDeploy) {
      throw new Error('deploy failed');
    }
    state.data = { ...state.data, ...p.fields };
    return {};
  });
  registerMockHandler('clipboard.copy', () => ({}));
  return state;
}
