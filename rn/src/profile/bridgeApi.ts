import { request } from '../bridge/request';
import type { ProfileData, ProfileFields } from '../bridge/methods';
import type { ProfileApi } from './api';
import type { Profile, ProfileDeployPayload } from './types';

const TAG_LENGTH = 4;

export function fromProfileData(data: ProfileData): Profile {
  return {
    address: data.address,
    name: data.name ?? '',
    tag: data.address.slice(-TAG_LENGTH),
    hasClaimedName: data.has_claimed_name ?? false,
    description: data.description ?? '',
    country: data.country ?? '',
    language: '',
    pronouns: data.pronouns ?? '',
    gender: data.gender ?? '',
    sexualOrientation: data.sexual_orientation ?? '',
    relationshipStatus: data.relationship_status ?? '',
    employmentStatus: data.employment_status ?? '',
    profession: data.profession ?? '',
    realName: data.real_name ?? '',
    hobbies: data.hobbies ?? '',
    links: data.links ?? [],
    equipped: [],
    faceUrl: data.profile_picture_url || undefined,
  };
}

const FIELD_NAMES: Record<keyof ProfileDeployPayload, keyof ProfileFields> = {
  name: 'name',
  hasClaimedName: 'has_claimed_name',
  description: 'description',
  country: 'country',
  pronouns: 'pronouns',
  gender: 'gender',
  sexualOrientation: 'sexual_orientation',
  relationshipStatus: 'relationship_status',
  employmentStatus: 'employment_status',
  profession: 'profession',
  realName: 'real_name',
  hobbies: 'hobbies',
  links: 'links',
};

export function toProfileFields(patch: ProfileDeployPayload): ProfileFields {
  const fields: Record<string, unknown> = {};
  (Object.keys(patch) as Array<keyof ProfileDeployPayload>).forEach(key => {
    fields[FIELD_NAMES[key]] = patch[key];
  });
  return fields as ProfileFields;
}

/** Profile over the request bridge; the mock bridge answers the same calls in tests. */
export function createBridgeProfileApi(): ProfileApi {
  const load = async () => fromProfileData(await request('profile.get'));
  return {
    getProfile: load,
    getNames: async () => (await request('profile.names')).names,
    deployProfile: async patch => {
      await request('profile.deploy', { fields: toProfileFields(patch) });
      return load();
    },
    copyText: text => {
      request('clipboard.copy', { text }).catch(() => {});
    },
  };
}
