import { SettingKey, SettingValue } from '../native';
import type { SettingEntry } from '../native';
import type { SettingsApi } from '../api/settings';

/** The keys the shell lists; the engine-owned ones are not part of the mock. */
const NAMES: Partial<Record<SettingKey, string>> = {
  [SettingKey.SubmitMessageClosesChat]: 'submit_message_closes_chat',
  [SettingKey.Locale]: 'locale',
  [SettingKey.GraphicProfile]: 'graphic_profile',
  [SettingKey.DynamicGraphicsEnabled]: 'dynamic_graphics_enabled',
  [SettingKey.DynamicSkybox]: 'dynamic_skybox',
  [SettingKey.SkyboxTime]: 'skybox_time',
  [SettingKey.ViewDistance]: 'view_distance',
  [SettingKey.ParticleQuality]: 'particle_quality',
  [SettingKey.LimitFps]: 'limit_fps',
  [SettingKey.MaxCacheSize]: 'max_cache_size',
  [SettingKey.AudioGeneralVolume]: 'audio_general_volume',
  [SettingKey.AudioSceneVolume]: 'audio_scene_volume',
  [SettingKey.AudioVoiceChatVolume]: 'audio_voice_chat_volume',
  [SettingKey.AudioUiVolume]: 'audio_ui_volume',
  [SettingKey.AudioMusicVolume]: 'audio_music_volume',
  [SettingKey.AudioAvatarAndEmotesVolume]: 'audio_avatar_and_emotes_volume',
  [SettingKey.AudioMicAmplification]: 'audio_mic_amplification',
};

const KEYS = (Object.keys(NAMES) as unknown as string[]).map(
  k => Number(k) as SettingKey,
);

const volume = () => new SettingValue.Float(100);

/** The ConfigData defaults of the engine. */
function defaultValue(key: SettingKey): SettingValue {
  switch (key) {
    case SettingKey.SubmitMessageClosesChat:
      return new SettingValue.Bool(false);
    case SettingKey.DynamicGraphicsEnabled:
    case SettingKey.DynamicSkybox:
      return new SettingValue.Bool(true);
    case SettingKey.Locale:
      return new SettingValue.Text('');
    case SettingKey.GraphicProfile:
    case SettingKey.ParticleQuality:
    case SettingKey.LimitFps:
      return new SettingValue.Int(3n);
    case SettingKey.SkyboxTime:
      return new SettingValue.Int(43200n);
    case SettingKey.MaxCacheSize:
      return new SettingValue.Int(1n);
    case SettingKey.ViewDistance:
      return new SettingValue.Float(300);
    default:
      return volume();
  }
}

const HALF_GB = 500_000_000n;
const TWO_GB = 2_000_000_000n;

/** Settings held in memory, with the defaults and a fake cache; values do not persist. */
export function createMockSettings(): SettingsApi {
  let entries: SettingEntry[] = KEYS.map(key => ({
    key,
    value: defaultValue(key),
  }));
  let cacheBytes = HALF_GB;

  return {
    settings: () => entries,
    setSetting: (key, value) => {
      const next: SettingEntry = { key, value };
      entries = entries.map(e => (e.key === key ? next : e));
      return [next];
    },
    settingKeyName: key => NAMES[key] ?? '',
    settingKeyFromName: name => KEYS.find(key => NAMES[key] === name),
    cacheSize: () => ({ bytes: cacheBytes, maxBytes: TWO_GB }),
    clearCache: async () => {
      cacheBytes = 0n;
      return cacheBytes;
    },
    version: () => '1.14.0-mock',
  };
}
