import type { Rarity } from '../ds';

export type ProfileLink = { title: string; url: string };

export type EquippedItem = {
  id: string;
  name: string;
  rarity: Rarity;
  thumbnail?: string;
  isEmote: boolean;
};

/** Own profile as the profile screens read it; option fields hold the legacy stable ids ("He / Him"). */
export type Profile = {
  address: string;
  name: string;
  /** Last 4 characters of the address; hidden when `hasClaimedName`. */
  tag: string;
  hasClaimedName: boolean;
  description: string;
  country: string;
  language: string;
  pronouns: string;
  gender: string;
  sexualOrientation: string;
  relationshipStatus: string;
  employmentStatus: string;
  profession: string;
  realName: string;
  hobbies: string;
  links: ProfileLink[];
  equipped: EquippedItem[];
  faceUrl?: string;
  /** Guests cannot change their username. */
  guest?: boolean;
};

/** What `deployProfile` sends: only the fields that changed. */
export type ProfileDeployPayload = Partial<
  Pick<
    Profile,
    | 'name'
    | 'hasClaimedName'
    | 'description'
    | 'country'
    | 'pronouns'
    | 'gender'
    | 'sexualOrientation'
    | 'relationshipStatus'
    | 'employmentStatus'
    | 'profession'
    | 'realName'
    | 'hobbies'
    | 'links'
  >
>;

/** The fields the Edit Profile form changes; `name` is the text-field value (non-unique tab). */
export type EditDraft = Pick<
  Profile,
  | 'name'
  | 'description'
  | 'country'
  | 'pronouns'
  | 'gender'
  | 'sexualOrientation'
  | 'relationshipStatus'
  | 'employmentStatus'
  | 'profession'
  | 'realName'
  | 'hobbies'
  | 'links'
>;

export type UsernameTab = 'unique' | 'nonUnique';
