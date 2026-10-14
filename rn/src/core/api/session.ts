import type * as Native from '../native';

/** Signing in and out, and the app state. A slice of the generated bindings, so the mock fails
 * `tsc` when a Rust signature changes. */
export type SessionApi = Pick<
  typeof Native,
  | 'state'
  | 'address'
  | 'isGuest'
  | 'installs'
  | 'addListener'
  | 'removeListener'
  | 'signInGuest'
  | 'signInEmailStart'
  | 'signInEmailVerify'
  | 'signInBrowser'
  | 'completeBrowser'
  | 'cancelSignIn'
  | 'signInWalletBegin'
  | 'signInWalletComplete'
  | 'signOut'
  | 'handleDeepLink'
>;
