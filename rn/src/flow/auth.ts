import { core } from '../core';
import { onCoreEvents } from '../core/events';
import { Control, CoreError } from '../core/native';

export type AuthResult = { hasProfile: boolean; name: string };

/** The stable `auth_error` code behind a failed core call, if it carried one. */
export function authCode(error: unknown): string | undefined {
  if (error instanceof CoreError.SignIn) {
    return error.inner.code;
  }
  return undefined;
}

export const startGuest = () => core().signInGuest();

export const sendEmailCode = (email: string) => core().signInEmailStart(email);

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

/** Who just signed in: a profile with a name means a returning account (welcome back). */
export async function welcome(): Promise<AuthResult> {
  const profile = await core().profile(undefined);
  const name = profile.name.trim();
  return { hasProfile: name.length > 0, name };
}

/** Terms the player already accepted in this version (or an older run of the same one). */
export const termsAccepted = () =>
  core().termsAcceptedVersion() >= core().termsVersion();

/** Records the acceptance and lets analytics start (nothing is sent before this). */
export async function acceptTerms(): Promise<void> {
  core().acceptTerms(core().termsVersion());
  await core().setAnalyticsConsent(true);
}

export const setAvatarName = (name: string) =>
  core().control(new Control.SetName({ name }));
