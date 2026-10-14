import { core } from '../core';
import { onCoreEvents } from '../core/events';
import { CoreError } from '../core/native';

export type AuthResult = { hasProfile: boolean; name: string };

/** The stable `auth_error` code behind a failed core call, if it carried one. */
export function authCode(error: unknown): string | undefined {
  if (error instanceof CoreError.SignIn) {
    return error.inner.code;
  }
  return undefined;
}

export const startGuest = () => core().signInGuest();

export const sendEmailCode = (email: string) =>
  core().signInEmailStart(email);

export const verifyEmailCode = (email: string, code: string) =>
  core().signInEmailVerify(email, code);

/** Google and Apple: the core asks the shell to open the page (see `watchOpenUrl`). */
export const startBrowserSignIn = (provider: 'google' | 'apple') =>
  core().signInBrowser(provider);

export const cancelSignIn = () => core().cancelSignIn();

/** Resolves once the app state says the user is signed in, rejects when a step fails. */
export function whenSignedIn(): Promise<void> {
  return new Promise((resolve, reject) => {
    const done = (fn: () => void) => {
      off();
      fn();
    };
    const off = onCoreEvents({
      stateChanged: state => state.tag === 'SignedIn' && done(resolve),
      authError: code => done(() => reject(new Error(code))),
    });
    if (core().state().tag === 'SignedIn') {
      done(resolve);
    }
  });
}
