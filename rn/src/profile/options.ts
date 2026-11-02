import type { I18nKey } from '../i18n';

export type OptionId = { id: string; label: I18nKey };

/** `id` is what gets published and read by other clients: never translate it. */
export const PRONOUNS: OptionId[] = [
  { id: 'He / Him', label: 'PROFILE_PRONOUN_HE_HIM' },
  { id: 'She / Her', label: 'PROFILE_PRONOUN_SHE_HER' },
  { id: 'They / Them', label: 'PROFILE_PRONOUN_THEY_THEM' },
  { id: 'Ze / Zir', label: 'PROFILE_PRONOUN_ZE_ZIR' },
  { id: 'Xe / Xem', label: 'PROFILE_PRONOUN_XE_XEM' },
  { id: 'Ze / Hir', label: 'PROFILE_PRONOUN_ZE_HIR' },
  { id: 'Per / Per', label: 'PROFILE_PRONOUN_PER_PER' },
  { id: 'Other', label: 'PROFILE_PRONOUN_OTHER' },
];

export const GENDERS: OptionId[] = [
  { id: 'Female', label: 'PROFILE_GENDER_FEMALE' },
  { id: 'Male', label: 'PROFILE_GENDER_MALE' },
  { id: 'Transgender', label: 'PROFILE_GENDER_TRANSGENDER' },
  { id: 'Neutral', label: 'PROFILE_GENDER_NEUTRAL' },
  { id: 'Non-Binary', label: 'PROFILE_GENDER_NON_BINARY' },
  { id: 'Fluid', label: 'PROFILE_GENDER_FLUID' },
  { id: 'Agender', label: 'PROFILE_GENDER_AGENDER' },
  { id: 'Pangender', label: 'PROFILE_GENDER_PANGENDER' },
  { id: 'Queer', label: 'PROFILE_GENDER_QUEER' },
  { id: 'Two-Spirit', label: 'PROFILE_GENDER_TWO_SPIRIT' },
  { id: 'Other', label: 'PROFILE_GENDER_OTHER' },
];

export const ORIENTATIONS: OptionId[] = [
  { id: 'Heterosexual', label: 'PROFILE_ORIENTATION_HETEROSEXUAL' },
  { id: 'Bisexual', label: 'PROFILE_ORIENTATION_BISEXUAL' },
  { id: 'Lesbian', label: 'PROFILE_ORIENTATION_LESBIAN' },
  { id: 'Gay', label: 'PROFILE_ORIENTATION_GAY' },
  { id: 'Asexual', label: 'PROFILE_ORIENTATION_ASEXUAL' },
  { id: 'Queer', label: 'PROFILE_ORIENTATION_QUEER' },
  { id: 'Polysexual', label: 'PROFILE_ORIENTATION_POLYSEXUAL' },
  { id: 'Pansexual', label: 'PROFILE_ORIENTATION_PANSEXUAL' },
  { id: 'Other', label: 'PROFILE_ORIENTATION_OTHER' },
];

export const RELATIONSHIPS: OptionId[] = [
  { id: 'Single', label: 'PROFILE_RELATIONSHIP_SINGLE' },
  { id: 'In a relationship', label: 'PROFILE_RELATIONSHIP_IN_A_RELATIONSHIP' },
  { id: 'Engaged', label: 'PROFILE_RELATIONSHIP_ENGAGED' },
  { id: 'Married', label: 'PROFILE_RELATIONSHIP_MARRIED' },
  { id: "It's complicated", label: 'PROFILE_RELATIONSHIP_IT_S_COMPLICATED' },
  {
    id: 'In an open relationship',
    label: 'PROFILE_RELATIONSHIP_IN_AN_OPEN_RELATIONSHIP',
  },
  { id: 'Widowed', label: 'PROFILE_RELATIONSHIP_WIDOWED' },
  { id: 'Separated', label: 'PROFILE_RELATIONSHIP_SEPARATED' },
  { id: 'Divorced', label: 'PROFILE_RELATIONSHIP_DIVORCED' },
  { id: 'Other', label: 'PROFILE_RELATIONSHIP_OTHER' },
];

/** Display text for a stored value; unknown values (written by a newer client) pass through. */
export function optionLabel(
  options: OptionId[],
  id: string,
  translate: (key: I18nKey) => string,
): string {
  const match = options.find(o => o.id === id);
  return match ? translate(match.label) : id;
}
