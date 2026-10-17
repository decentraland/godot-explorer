import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { SettingsApi } from './api';
import {
  AccountInfo,
  defaultEnv,
  defaultSettings,
  SettingKey,
  SettingsEnv,
  SettingsValues,
  VersionInfo,
} from './keys';
import { dependentChanges } from './rules';

const NO_ACCOUNT: AccountInfo = {
  name: '',
  tag: '',
  address: '',
  guest: true,
  email: null,
};

export type SettingsModel = {
  values: SettingsValues;
  env: SettingsEnv;
  account: AccountInfo;
  version: VersionInfo;
  cacheBytes: number;
  /** Applies at once; `commit` false leaves persistence to a later call (slider drag). */
  set: <K extends SettingKey>(
    key: K,
    value: SettingsValues[K],
    commit?: boolean,
  ) => void;
  clearCache: () => Promise<void>;
  api: SettingsApi;
};

/** Loads everything once, keeps it in state, and reloads when the engine reports a change. */
export function useSettings(
  api: SettingsApi,
  onSaveError?: () => void,
): SettingsModel {
  const [values, setValues] = useState<SettingsValues>(defaultSettings);
  const [env, setEnv] = useState<SettingsEnv>(defaultEnv);
  const [account, setAccount] = useState<AccountInfo>(NO_ACCOUNT);
  const [version, setVersion] = useState<VersionInfo>({ short: '', full: '' });
  const [cacheBytes, setCacheBytes] = useState(0);
  const alive = useRef(true);

  const load = useCallback(async () => {
    const [v, e, a, ver, c] = await Promise.all([
      api.getAll(),
      api.getEnv(),
      api.getAccount(),
      api.getVersion(),
      api.getCacheSize(),
    ]);
    if (alive.current) {
      setValues(v);
      setEnv(e);
      setAccount(a);
      setVersion(ver);
      setCacheBytes(c);
    }
  }, [api]);

  useEffect(() => {
    alive.current = true;
    load().catch(() => {});
    const off = api.subscribe(() => load().catch(() => {}));
    return () => {
      alive.current = false;
      off();
    };
  }, [api, load]);

  const set = useCallback<SettingsModel['set']>(
    (key, value, commit = true) => {
      const extra = dependentChanges(key, value);
      setValues(prev => ({ ...prev, [key]: value, ...extra }));
      const writes = [
        api.set(key, value, commit),
        ...(Object.entries(extra) as [SettingKey, never][]).map(([k, v]) =>
          api.set(k, v, commit),
        ),
      ];
      Promise.all(writes).catch(() => {
        onSaveError?.();
        load().catch(() => {});
      });
    },
    [api, load, onSaveError],
  );

  const clearCache = useCallback(async () => {
    await api.clearCache();
    const bytes = await api.getCacheSize();
    if (alive.current) {
      setCacheBytes(bytes);
    }
  }, [api]);

  return useMemo(
    () => ({ values, env, account, version, cacheBytes, set, clearCache, api }),
    [values, env, account, version, cacheBytes, set, clearCache, api],
  );
}
