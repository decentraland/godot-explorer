import type {
  EditDraft,
  Profile,
  ProfileDeployPayload,
  ProfileLink,
  UsernameTab,
} from './types';

export const MAX_LINKS = 5;
export const LIMITS = {
  username: 15,
  description: 300,
  field: 30,
  linkTitle: 15,
} as const;

const URL_PATTERN = /^https?:\/\/[^\s]+\.[^\s]+$/;
const EDGE_SPACES = /(^\s)|(\s$)/;

/** `0x4e1...1c13B`: first 5 and last 5 characters. */
export const shortAddress = (address: string): string =>
  address.length > 13
    ? `${address.slice(0, 5)}...${address.slice(-5)}`
    : address;

export const isValidUrl = (url: string): boolean => URL_PATTERN.test(url);

export type FieldError = 'limit' | 'url' | 'edgeSpaces';

export function validateText(
  value: string,
  max: number,
  rules: { url?: boolean; noEdgeSpaces?: boolean } = {},
): FieldError | undefined {
  if (value.length > max) {
    return 'limit';
  }
  if (rules.url && value.length > 0 && !isValidUrl(value)) {
    return 'url';
  }
  if (rules.noEdgeSpaces && EDGE_SPACES.test(value)) {
    return 'edgeSpaces';
  }
  return undefined;
}

export const canSaveLink = (title: string, url: string): boolean =>
  title.trim().length > 0 &&
  !validateText(title, LIMITS.linkTitle) &&
  url.length > 0 &&
  !validateText(url, Number.MAX_SAFE_INTEGER, { url: true });

export type AboutKey =
  | 'country'
  | 'language'
  | 'pronouns'
  | 'gender'
  | 'relationshipStatus'
  | 'sexualOrientation'
  | 'employmentStatus'
  | 'profession'
  | 'realName'
  | 'hobbies';

/** Display order of the About grid. */
export const ABOUT_ORDER: AboutKey[] = [
  'country',
  'language',
  'pronouns',
  'gender',
  'relationshipStatus',
  'sexualOrientation',
  'employmentStatus',
  'profession',
  'realName',
  'hobbies',
];

/** Keys with a value, in display order; empty fields are hidden. */
export const filledAboutKeys = (profile: Profile): AboutKey[] =>
  ABOUT_ORDER.filter(key => profile[key].trim().length > 0);

export type AboutMode = 'none' | 'description' | 'data' | 'both';

export function aboutMode(profile: Profile): AboutMode {
  const description = profile.description.trim().length > 0;
  const data = filledAboutKeys(profile).length > 0;
  if (description && data) {
    return 'both';
  }
  if (description) {
    return 'description';
  }
  return data ? 'data' : 'none';
}

const COLLAPSED_DATA_COUNT = 3;
const LONG_DESCRIPTION = 140;

export type LandscapeAbout = {
  mode: AboutMode;
  showDescription: boolean;
  keys: AboutKey[];
  /** Which toggle to draw, if any. */
  toggle: 'more' | 'less' | null;
};

/** Landscape About: description and data are shown together only when expanded. */
export function landscapeAbout(
  profile: Profile,
  expanded: boolean,
): LandscapeAbout {
  const mode = aboutMode(profile);
  const all = filledAboutKeys(profile);
  switch (mode) {
    case 'both':
      return {
        mode,
        showDescription: true,
        keys: expanded ? all : [],
        toggle: expanded ? 'less' : 'more',
      };
    case 'description':
      return {
        mode,
        showDescription: true,
        keys: [],
        toggle: expanded
          ? 'less'
          : profile.description.length > LONG_DESCRIPTION
          ? 'more'
          : null,
      };
    case 'data':
      return {
        mode,
        showDescription: false,
        keys: expanded ? all : all.slice(0, COLLAPSED_DATA_COUNT),
        toggle:
          all.length > COLLAPSED_DATA_COUNT
            ? expanded
              ? 'less'
              : 'more'
            : null,
      };
    default:
      return { mode, showDescription: false, keys: [], toggle: null };
  }
}

export const draftFromProfile = (p: Profile): EditDraft => ({
  name: p.name,
  description: p.description,
  country: p.country,
  pronouns: p.pronouns,
  gender: p.gender,
  sexualOrientation: p.sexualOrientation,
  relationshipStatus: p.relationshipStatus,
  employmentStatus: p.employmentStatus,
  profession: p.profession,
  realName: p.realName,
  hobbies: p.hobbies,
  links: p.links,
});

const sameLinks = (a: ProfileLink[], b: ProfileLink[]): boolean =>
  a.length === b.length &&
  a.every((l, i) => l.title === b[i].title && l.url === b[i].url);

/** The name the form publishes: the picked minted name on the unique tab, else the typed one. */
export function resolveUsername(
  tab: UsernameTab,
  typed: string,
  names: readonly string[],
  pickedName: string | undefined,
): { name: string; hasClaimedName: boolean } {
  if (tab === 'unique' && names.length > 0) {
    return { name: pickedName ?? names[0], hasClaimedName: true };
  }
  return { name: typed, hasClaimedName: names.includes(typed) };
}

/** Changed fields only, like the legacy `_async_save_profile`. */
export function buildPatch(
  original: Profile,
  draft: EditDraft,
  username: { name: string; hasClaimedName: boolean },
): ProfileDeployPayload {
  const patch: ProfileDeployPayload = {};
  if (username.name !== original.name) {
    patch.name = username.name;
  }
  if (username.hasClaimedName !== original.hasClaimedName) {
    patch.hasClaimedName = username.hasClaimedName;
  }
  const textKeys = [
    'description',
    'country',
    'pronouns',
    'gender',
    'sexualOrientation',
    'relationshipStatus',
    'employmentStatus',
    'profession',
    'realName',
    'hobbies',
  ] as const;
  for (const key of textKeys) {
    if (draft[key] !== original[key]) {
      patch[key] = draft[key];
    }
  }
  if (!sameLinks(draft.links, original.links)) {
    patch.links = draft.links;
  }
  return patch;
}

export const isDirty = (patch: ProfileDeployPayload): boolean =>
  Object.keys(patch).length > 0;

/** Whether a dropdown list of `listHeight` fits below a field; otherwise it opens upward. */
export function dropdownDirection(
  fieldBottom: number,
  limit: number,
  listHeight: number,
): 'up' | 'down' {
  return fieldBottom + listHeight > limit ? 'up' : 'down';
}
