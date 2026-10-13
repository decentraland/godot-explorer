import { registerMockHandler } from '../bridge/mockHandlers';
import type { SettingValue } from '../bridge/methods';
import { defaultSettings } from './keys';

/** Fakes the engine's settings.* and clipboard methods on the mock bridge. */
export function registerSettingsMockHandlers(
  seed: Record<string, SettingValue> = {},
): { values: Record<string, SettingValue> } {
  const state = {
    values: { ...defaultSettings(), ...seed } as Record<string, SettingValue>,
  };
  let cacheBytes = 5e8;
  registerMockHandler('settings.get_all', () => ({ ...state.values }));
  registerMockHandler(
    'settings.set',
    (p: { key: string; value: SettingValue }) => {
      state.values = { ...state.values, [p.key]: p.value };
      return { key: p.key, value: p.value };
    },
  );
  registerMockHandler('settings.cache_size', () => ({
    bytes: cacheBytes,
    max_bytes: 2e9,
  }));
  registerMockHandler('settings.clear_cache', () => {
    cacheBytes = 0;
    return { bytes: 0 };
  });
  registerMockHandler('settings.version', () => ({ version: '1.14.0-mock' }));
  registerMockHandler('clipboard.copy', () => ({}));
  registerMockHandler('account.sign_out', () => ({}));
  registerMockHandler('account.delete', () => ({}));
  return state;
}
