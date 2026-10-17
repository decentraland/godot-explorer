import type { SettingsApi } from './api';
import {
  AccountInfo,
  defaultEnv,
  defaultSettings,
  SettingsEnv,
  SettingsValues,
  VersionInfo,
} from './keys';

export type MockSeed = {
  values?: Partial<SettingsValues>;
  env?: Partial<SettingsEnv>;
  account?: Partial<AccountInfo>;
  cacheBytes?: number;
  version?: VersionInfo;
};

export type MockSettingsApi = SettingsApi & {
  /** Every mutating call, in order, for tests and the gallery. */
  calls: string[];
  /** Fires the subscribers as the engine would on its own change. */
  emitChange: (patch?: Partial<SettingsEnv>) => void;
};

const HALF_GB = 5e8;

/** Deterministic in-memory implementation: the gallery and jest use it. */
export function createMockApi(seed: MockSeed = {}): MockSettingsApi {
  let values: SettingsValues = { ...defaultSettings(), ...seed.values };
  let env: SettingsEnv = { ...defaultEnv(), ...seed.env };
  let cacheBytes = seed.cacheBytes ?? HALF_GB;
  const account: AccountInfo = {
    name: 'Namemaxcharacte',
    tag: '#1234',
    address: '0x4e1c0b2f9d3a7e5c8b6a41d2e0f3c9a8b7d61c13B',
    guest: true,
    email: null,
    ...seed.account,
  };
  const version = seed.version ?? { short: '1.14.0', full: '1.14.0-mock' };
  const listeners = new Set<() => void>();
  const calls: string[] = [];

  return {
    calls,
    getAll: async () => ({ ...values }),
    set: async (key, value, commit = true) => {
      values = { ...values, [key]: value };
      calls.push(`set:${key}=${String(value)}${commit ? '' : ':live'}`);
    },
    getEnv: async () => ({ ...env }),
    getAccount: async () => ({ ...account }),
    getCacheSize: async () => cacheBytes,
    clearCache: async () => {
      cacheBytes = 0;
      calls.push('clearCache');
    },
    getVersion: async () => version,
    copyText: async text => {
      calls.push(`copy:${text}`);
    },
    startEmailUpgrade: async () => {
      calls.push('addEmail');
    },
    signOut: async () => {
      calls.push('signOut');
    },
    deleteAccount: async () => {
      calls.push('deleteAccount');
      return { ok: true };
    },
    openBugReport: async () => {
      calls.push('bugReport');
    },
    openReportContent: async () => {
      calls.push('reportContent');
    },
    devAction: async action => {
      calls.push(`dev:${action}`);
    },
    subscribe: listener => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    emitChange: patch => {
      env = { ...env, ...patch };
      listeners.forEach(l => l());
    },
  };
}
