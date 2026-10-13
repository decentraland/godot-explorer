import { subscribe } from '../GodotBridge';
import { request } from '../bridge/request';
import type { SettingValue } from '../bridge/methods';
import type { SettingsApi } from './api';
import {
  defaultEnv,
  defaultSettings,
  SETTING_DEFAULTS,
  SettingKey,
  SettingsValues,
} from './keys';

/** Keys the engine persists and applies; the rest (session flags, dev tools) live in this process. */
const ENGINE_KEYS = new Set<SettingKey>([
  'graphic_profile',
  'dynamic_graphics_enabled',
  'dynamic_skybox',
  'skybox_time',
  'audio_general_volume',
  'audio_scene_volume',
  'audio_ui_volume',
  'audio_music_volume',
  'audio_avatar_and_emotes_volume',
  'audio_voice_chat_volume',
  'audio_mic_amplification',
  'submit_message_closes_chat',
  'locale',
  'max_cache_size',
]);

const GUEST_TAG_LENGTH = 4;

/** Settings over the request bridge; same code runs against the mock bridge handlers. */
export function createBridgeSettingsApi(): SettingsApi {
  let local: Partial<SettingsValues> = {};

  return {
    getAll: async () => {
      const remote = await request('settings.get_all');
      const values: Record<string, SettingValue> = { ...defaultSettings() };
      Object.keys(remote).forEach(key => {
        if (key in SETTING_DEFAULTS) {
          values[key] = remote[key];
        }
      });
      return { ...values, ...local } as SettingsValues;
    },
    set: async (key, value, commit = true) => {
      if (!ENGINE_KEYS.has(key)) {
        local = { ...local, [key]: value };
        return;
      }
      await request('settings.set', { key, value, commit });
    },
    getEnv: async () => defaultEnv(),
    getAccount: async () => {
      const p = await request('profile.get');
      return {
        name: p.name ?? '',
        tag: p.has_claimed_name ? '' : `#${p.address.slice(-GUEST_TAG_LENGTH)}`,
        address: p.address,
        guest: false,
        email: null,
      };
    },
    getCacheSize: async () => (await request('settings.cache_size')).bytes,
    clearCache: async () => {
      await request('settings.clear_cache');
    },
    getVersion: async () => {
      const { version } = await request('settings.version');
      return { short: version, full: version };
    },
    copyText: async text => {
      await request('clipboard.copy', { text });
    },
    startEmailUpgrade: async () => {},
    signOut: async () => {
      await request('account.sign_out');
    },
    deleteAccount: async () => {
      try {
        await request('account.delete');
        return { ok: true };
      } catch (e) {
        return { ok: false, error: e instanceof Error ? e.message : String(e) };
      }
    },
    openBugReport: async () => {},
    openReportContent: async () => {},
    devAction: async () => {},
    subscribe: listener =>
      subscribe(event => {
        if (event.type === 'settings_changed') {
          listener();
        }
      }),
  };
}
