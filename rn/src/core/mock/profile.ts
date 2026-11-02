import { core } from '..';
import { AccountDeletion } from '../native';
import type { Profile, ProfileUpdate } from '../native';
import type { ProfileApi } from '../api/profile';

const GUEST_PREFIX = '0xguest';
const MOCK_NAMES = ['Name 01', 'Name 02', 'Name'];

const SAMPLE: Omit<Profile, 'address' | 'name' | 'version'> = {
  hasClaimedName: false,
  description:
    "Visit my personal web. I love building and making friends in Decentraland. When I'm not decorating LAND, I like to play games and hang out in the districts!",
  country: 'Argentina',
  pronouns: 'He / Him',
  gender: 'Male',
  sexualOrientation: 'Bisexual',
  relationshipStatus: 'Single',
  employmentStatus: 'Working',
  profession: 'Dev',
  realName: 'George Pascal',
  hobbies: 'Drawing',
  links: [
    { title: 'Instagram', url: 'https://www.instagram.com/decentraland' },
    { title: 'GitHub', url: 'https://github.com/decentraland' },
    { title: 'Youtube', url: 'https://www.youtube.com/@decentraland' },
    { title: 'Web', url: 'https://decentraland.org' },
    { title: 'Discord', url: 'https://discord.gg/decentraland' },
  ],
  profilePictureUrl: '',
};

/** Guests and signed-out sessions have no name yet; any other account answers with one. */
function sampleFor(address: string): Profile {
  const named = address !== '' && !address.startsWith(GUEST_PREFIX);
  return {
    ...SAMPLE,
    address,
    version: 1,
    name: named ? 'Namemaxcharacte' : '',
  };
}

const applied = (current: Profile, update: ProfileUpdate): Profile => {
  const defined = Object.fromEntries(
    Object.entries(update).filter(([, value]) => value !== undefined),
  );
  return { ...current, ...defined, version: current.version + 1 };
};

/** Profiles kept in memory per address, for builds without the engine library. */
export function createMockProfile(): ProfileApi {
  const byAddress = new Map<string, Profile>();
  const own = () => core().address() ?? '';
  const read = (address: string) =>
    byAddress.get(address) ?? sampleFor(address);

  return {
    profile: async address => read(address || own()),
    ownedNames: async () => [...MOCK_NAMES],
    deployProfile: async update => {
      const address = own();
      const next = applied(read(address), update);
      byAddress.set(address, next);
      return next;
    },
    deleteAccount: async () => AccountDeletion.Requested,
  };
}
