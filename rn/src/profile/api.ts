import { godotAvailable } from '../GodotBridge';
import { createBridgeProfileApi } from './bridgeApi';
import { MOCK_NAMES, MOCK_PROFILE } from './mockData';
import type { Profile, ProfileDeployPayload } from './types';

/** Everything the profile screens need from the engine. */
export interface ProfileApi {
  getProfile(): Promise<Profile>;
  /** Claimed (minted) names of the account. */
  getNames(): Promise<string[]>;
  deployProfile(patch: ProfileDeployPayload): Promise<Profile>;
  /** Bridge command `copy_to_clipboard {text}` (Godot `DisplayServer.clipboard_set`). */
  copyText(text: string): void;
}

export type MockApiOptions = {
  profile?: Profile;
  names?: string[];
  failDeploy?: boolean;
};

export function createMockApi(options: MockApiOptions = {}): ProfileApi & {
  copied: string[];
  deployed: ProfileDeployPayload[];
} {
  let current = options.profile ?? MOCK_PROFILE;
  const copied: string[] = [];
  const deployed: ProfileDeployPayload[] = [];
  return {
    copied,
    deployed,
    getProfile: () => Promise.resolve(current),
    getNames: () => Promise.resolve(options.names ?? MOCK_NAMES),
    deployProfile: patch => {
      if (options.failDeploy) {
        return Promise.reject(new Error('deploy failed'));
      }
      deployed.push(patch);
      current = { ...current, ...patch };
      return Promise.resolve(current);
    },
    copyText: text => {
      copied.push(text);
    },
  };
}

export const mockApi = createMockApi();

let active: ProfileApi = godotAvailable ? createBridgeProfileApi() : mockApi;

export const getProfileApi = (): ProfileApi => active;

export function setProfileApi(api: ProfileApi): void {
  active = api;
}
