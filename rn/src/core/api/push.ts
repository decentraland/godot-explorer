import type * as Native from '../native';

/** How this install can be reached by push: the OS owns the token and the permission, the core
 * decides when the registration is reported. */
export type PushApi = Pick<typeof Native, 'setPushRegistration'>;
