import { core } from '../core';
import { onCoreEvents } from '../core/events';
import { SettingKey as CoreKey, SettingValue } from '../core/native';
import type { SettingEntry } from '../core/native';
import type { SettingsApi } from './api';
import {
  defaultEnv,
  defaultSettings,
  SettingKey,
  SettingsValues,
} from './keys';

const GUEST_TAG_LENGTH = 4;

const toCoreValue = (value: unknown): SettingValue => {
  if (typeof value === 'boolean') {
    return new SettingValue.Bool(value);
  }
  if (typeof value === 'string') {
    return new SettingValue.Text(value);
  }
  const n = Number(value);
  return Number.isInteger(n)
    ? new SettingValue.Int(BigInt(n))
    : new SettingValue.Float(n);
};

const fromCoreValue = (value: SettingValue): boolean | number | string => {
  const [inner] = value.inner;
  return typeof inner === 'bigint' ? Number(inner) : inner;
};

/** Engine values by UI key; keys the shell does not know and empty text (follow the device) are dropped. */
export function entriesToValues(
  entries: SettingEntry[],
): Partial<SettingsValues> {
  const known = defaultSettings();
  const values: Record<string, boolean | number | string> = {};
  entries.forEach(({ key, value }) => {
    const name = core().settingKeyName(key);
    const v = fromCoreValue(value);
    if (name in known && v !== '') {
      values[name] = v;
    }
  });
  return values as Partial<SettingsValues>;
}

/** Settings over the core. Session flags and dev tools have no core key and live in this process. */
export function createCoreSettingsApi(): SettingsApi {
  let local: Partial<SettingsValues> = {};
  const engineKey = (key: SettingKey): CoreKey | undefined =>
    core().settingKeyFromName(key);

  return {
    getAll: async () => ({
      ...defaultSettings(),
      ...entriesToValues(core().settings()),
      ...local,
    }),
    set: async (key, value, commit = true) => {
      const coreKey = engineKey(key);
      if (coreKey === undefined) {
        local = { ...local, [key]: value };
        return;
      }
      core().setSetting(coreKey, toCoreValue(value), commit);
    },
    getEnv: async () => defaultEnv(),
    getAccount: async () => {
      const p = await core().profile(undefined);
      return {
        name: p.name,
        tag: p.hasClaimedName ? '' : `#${p.address.slice(-GUEST_TAG_LENGTH)}`,
        address: p.address,
        guest: false,
        email: null,
      };
    },
    getCacheSize: async () => Number(core().cacheSize().bytes),
    clearCache: async () => {
      await core().clearCache();
    },
    getVersion: async () => {
      const version = core().version();
      return { short: version, full: version };
    },
    // No clipboard or support-flow functions in the core yet.
    copyText: async () => {},
    startEmailUpgrade: async () => {},
    signOut: async () => {
      core().signOut();
    },
    deleteAccount: async () => {
      try {
        await core().deleteAccount();
        return { ok: true };
      } catch (e) {
        return { ok: false, error: e instanceof Error ? e.message : String(e) };
      }
    },
    openBugReport: async () => {},
    openReportContent: async () => {},
    devAction: async () => {},
    subscribe: listener => onCoreEvents({ settingsChanged: () => listener() }),
  };
}
