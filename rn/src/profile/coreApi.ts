import { core } from '../core';
import type { Profile as CoreProfile } from '../core/native';
import type { ProfileApi } from './api';
import type { Profile } from './types';

const TAG_LENGTH = 4;

export function fromCoreProfile(data: CoreProfile): Profile {
  return {
    address: data.address,
    name: data.name,
    tag: data.address.slice(-TAG_LENGTH),
    hasClaimedName: data.hasClaimedName,
    description: data.description,
    country: data.country,
    language: '',
    pronouns: data.pronouns,
    gender: data.gender,
    sexualOrientation: data.sexualOrientation,
    relationshipStatus: data.relationshipStatus,
    employmentStatus: data.employmentStatus,
    profession: data.profession,
    realName: data.realName,
    hobbies: data.hobbies,
    links: data.links,
    equipped: [],
    faceUrl: data.profilePictureUrl || undefined,
  };
}

/** Profile over the typed core; the core mock answers the same calls without the engine. */
export function createCoreProfileApi(): ProfileApi {
  return {
    getProfile: async () => fromCoreProfile(await core().profile(undefined)),
    getNames: () => core().ownedNames(),
    deployProfile: async patch =>
      fromCoreProfile(await core().deployProfile(patch)),
    // The core has no clipboard capability yet; see the migration notes.
    copyText: () => {},
  };
}
