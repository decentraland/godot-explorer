import {
  aboutMode,
  buildPatch,
  canSaveLink,
  draftFromProfile,
  dropdownDirection,
  filledAboutKeys,
  isDirty,
  isValidUrl,
  landscapeAbout,
  resolveUsername,
  shortAddress,
  validateText,
} from '../src/profile/logic';
import { MOCK_PROFILE } from '../src/profile/mockData';
import {
  GENDERS,
  optionLabel,
  ORIENTATIONS,
  PRONOUNS,
} from '../src/profile/options';
import type { Profile } from '../src/profile/types';

const empty: Profile = {
  ...MOCK_PROFILE,
  description: '',
  country: '',
  language: '',
  pronouns: '',
  gender: '',
  sexualOrientation: '',
  relationshipStatus: '',
  employmentStatus: '',
  profession: '',
  realName: '',
  hobbies: '',
};

describe('formatting and validation', () => {
  it('shortens an address to first 5 and last 5', () => {
    expect(shortAddress('0x4e1a5c5eb2A8a7aA7B0a09E04f1b2a1c13B')).toBe(
      '0x4e1...1c13B',
    );
    expect(shortAddress('0x12')).toBe('0x12');
  });

  it('accepts only http(s) URLs with a dot', () => {
    expect(isValidUrl('https://www.instagram.com/ru')).toBe(true);
    expect(isValidUrl('http://a.b')).toBe(true);
    expect(isValidUrl('instagram.com')).toBe(false);
    expect(isValidUrl('https://nodot')).toBe(false);
    expect(isValidUrl('https://a b.com')).toBe(false);
  });

  it('flags limit, url and edge spaces', () => {
    expect(validateText('a'.repeat(16), 15)).toBe('limit');
    expect(validateText('bad', 15, { url: true })).toBe('url');
    expect(validateText('', 15, { url: true })).toBeUndefined();
    expect(validateText(' x', 15, { noEdgeSpaces: true })).toBe('edgeSpaces');
    expect(validateText('x y', 15, { noEdgeSpaces: true })).toBeUndefined();
  });

  it('enables SAVE only for a titled link with a valid URL', () => {
    expect(canSaveLink('', '')).toBe(false);
    expect(canSaveLink('Instagram', '')).toBe(false);
    expect(canSaveLink('Instagram', 'nope')).toBe(false);
    expect(canSaveLink('   ', 'https://a.com')).toBe(false);
    expect(canSaveLink('a'.repeat(16), 'https://a.com')).toBe(false);
    expect(canSaveLink('Instagram', 'https://www.instagram.com/ru')).toBe(true);
  });
});

describe('About me', () => {
  it('hides empty fields and keeps the display order', () => {
    expect(filledAboutKeys(empty)).toEqual([]);
    expect(
      filledAboutKeys({ ...empty, hobbies: 'x', country: 'y', gender: ' ' }),
    ).toEqual(['country', 'hobbies']);
  });

  it('picks the mode from description and data', () => {
    expect(aboutMode(empty)).toBe('none');
    expect(aboutMode({ ...empty, description: 'hi' })).toBe('description');
    expect(aboutMode({ ...empty, country: 'AR' })).toBe('data');
    expect(aboutMode(MOCK_PROFILE)).toBe('both');
  });

  it('landscape both: description only until expanded, then everything', () => {
    const collapsed = landscapeAbout(MOCK_PROFILE, false);
    expect(collapsed).toMatchObject({
      showDescription: true,
      keys: [],
      toggle: 'more',
    });
    const open = landscapeAbout(MOCK_PROFILE, true);
    expect(open.keys).toHaveLength(10);
    expect(open.toggle).toBe('less');
  });

  it('landscape data only: first three fields, toggle only when more exist', () => {
    const three = {
      ...empty,
      country: 'a',
      language: 'b',
      pronouns: 'He / Him',
    };
    expect(landscapeAbout(three, false)).toMatchObject({
      showDescription: false,
      keys: ['country', 'language', 'pronouns'],
      toggle: null,
    });
    const six = { ...three, gender: 'Male', relationshipStatus: 'Single' };
    expect(
      landscapeAbout({ ...six, sexualOrientation: 'Gay' }, false),
    ).toMatchObject({
      keys: ['country', 'language', 'pronouns'],
      toggle: 'more',
    });
    expect(landscapeAbout(six, true).keys).toHaveLength(5);
    expect(landscapeAbout(six, true).toggle).toBe('less');
  });

  it('landscape description only: toggle for long text, none for short', () => {
    expect(
      landscapeAbout({ ...empty, description: 'short' }, false).toggle,
    ).toBeNull();
    expect(
      landscapeAbout({ ...empty, description: 'x'.repeat(200) }, false).toggle,
    ).toBe('more');
    expect(
      landscapeAbout({ ...empty, description: 'x'.repeat(200) }, true).toggle,
    ).toBe('less');
  });

  it('landscape with nothing shows no About block', () => {
    expect(landscapeAbout(empty, false)).toMatchObject({
      mode: 'none',
      showDescription: false,
      toggle: null,
    });
  });
});

describe('edit patch', () => {
  const unchanged = { name: MOCK_PROFILE.name, hasClaimedName: false };

  it('is empty for an untouched draft', () => {
    const patch = buildPatch(
      MOCK_PROFILE,
      draftFromProfile(MOCK_PROFILE),
      unchanged,
    );
    expect(patch).toEqual({});
    expect(isDirty(patch)).toBe(false);
  });

  it('carries only the changed fields, with stable option ids', () => {
    const draft = {
      ...draftFromProfile(MOCK_PROFILE),
      pronouns: 'She / Her',
      links: [{ title: 'Web', url: 'https://decentraland.org' }],
    };
    const patch = buildPatch(MOCK_PROFILE, draft, {
      name: 'Other',
      hasClaimedName: true,
    });
    expect(patch).toEqual({
      name: 'Other',
      hasClaimedName: true,
      pronouns: 'She / Her',
      links: [{ title: 'Web', url: 'https://decentraland.org' }],
    });
    expect(isDirty(patch)).toBe(true);
  });

  it('treats a reordered link list as a change', () => {
    const links = [...MOCK_PROFILE.links].reverse();
    const patch = buildPatch(
      MOCK_PROFILE,
      { ...draftFromProfile(MOCK_PROFILE), links },
      unchanged,
    );
    expect(patch.links).toEqual(links);
  });

  it('resolves the username like the legacy picker', () => {
    const names = ['Name 01', 'Name 02'];
    expect(resolveUsername('unique', 'typed', names, 'Name 02')).toEqual({
      name: 'Name 02',
      hasClaimedName: true,
    });
    expect(resolveUsername('unique', 'typed', names, undefined).name).toBe(
      'Name 01',
    );
    expect(resolveUsername('nonUnique', 'typed', names, 'Name 02')).toEqual({
      name: 'typed',
      hasClaimedName: false,
    });
    expect(
      resolveUsername('nonUnique', 'Name 01', names, undefined).hasClaimedName,
    ).toBe(true);
    expect(resolveUsername('unique', 'typed', [], undefined)).toEqual({
      name: 'typed',
      hasClaimedName: false,
    });
  });
});

describe('options and dropdown placement', () => {
  it('uses the legacy ids with spaces and the gender list, not the orientation one', () => {
    expect(PRONOUNS[0].id).toBe('He / Him');
    expect(GENDERS.map(g => g.id)).toContain('Non-Binary');
    expect(GENDERS.map(g => g.id)).not.toContain('Heterosexual');
    expect(ORIENTATIONS[0].id).toBe('Heterosexual');
  });

  it('labels known ids and passes unknown ones through', () => {
    expect(optionLabel(PRONOUNS, 'She / Her', () => 'translated')).toBe(
      'translated',
    );
    expect(optionLabel(PRONOUNS, 'Newer', () => 'x')).toBe('Newer');
  });

  it('opens upward only when the list would not fit below', () => {
    expect(dropdownDirection(100, 600, 200)).toBe('down');
    expect(dropdownDirection(500, 600, 200)).toBe('up');
  });
});
