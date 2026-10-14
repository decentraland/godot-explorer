import type * as Native from '../native';

/** The settings slice of the generated bindings. */
export type SettingsApi = Pick<
  typeof Native,
  | 'settings'
  | 'setSetting'
  | 'settingKeyName'
  | 'settingKeyFromName'
  | 'cacheSize'
  | 'clearCache'
  | 'version'
>;
