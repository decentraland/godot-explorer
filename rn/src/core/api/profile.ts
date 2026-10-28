import type * as Native from '../native';

/** The profile slice of the generated bindings. */
export type ProfileApi = Pick<
  typeof Native,
  'profile' | 'ownedNames' | 'deployProfile' | 'deleteAccount'
>;
