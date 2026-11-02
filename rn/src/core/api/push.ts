import type * as Native from '../native';

/** How this install can be reached by push and what local notifications are due: the OS owns the
 * token, the permission and the alarms; the core decides. */
export type PushApi = Pick<
  typeof Native,
  'setPushRegistration' | 'day1Notification'
>;
