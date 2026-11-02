import { useSyncExternalStore } from 'react';
import { getProfileApi } from './api';
import type { Profile, ProfileDeployPayload } from './types';

export type ProfileState =
  | { status: 'loading'; profile: null }
  | { status: 'ready'; profile: Profile }
  | { status: 'error'; profile: null };

const LOADING: ProfileState = { status: 'loading', profile: null };
let state: ProfileState = LOADING;
const listeners = new Set<() => void>();

const publish = (next: ProfileState) => {
  state = next;
  listeners.forEach(listener => listener());
};

export function loadProfile(): Promise<void> {
  publish(LOADING);
  return getProfileApi()
    .getProfile()
    .then(profile => publish({ status: 'ready', profile }))
    .catch(() => publish({ status: 'error', profile: null }));
}

/** Shows the patch at once and rolls back to the stored profile if the deploy fails. */
export async function saveProfile(
  patch: ProfileDeployPayload,
): Promise<boolean> {
  const before = state;
  if (before.status !== 'ready') {
    return false;
  }
  publish({ status: 'ready', profile: { ...before.profile, ...patch } });
  try {
    publish({
      status: 'ready',
      profile: await getProfileApi().deployProfile(patch),
    });
    return true;
  } catch {
    publish(before);
    return false;
  }
}

export const getProfileState = (): ProfileState => state;

export function useProfile(): ProfileState {
  return useSyncExternalStore(
    listener => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => state,
  );
}

export function resetProfileStore(): void {
  state = LOADING;
  listeners.clear();
}
