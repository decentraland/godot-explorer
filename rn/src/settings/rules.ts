import { SKYBOX_ON_DYNAMIC_OFF, SettingKey, SettingsValues } from './keys';

/** Other keys that change with `key` (legacy: turning Dynamic Skybox off selects Morning). */
export function dependentChanges<K extends SettingKey>(
  key: K,
  value: SettingsValues[K],
): Partial<SettingsValues> {
  return key === 'dynamic_skybox' && value === false
    ? { skybox_time: SKYBOX_ON_DYNAMIC_OFF }
    : {};
}
