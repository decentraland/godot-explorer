/** Legacy `config/...` keys of settings.cfg with the types and defaults godot/src/config/config_data.gd uses. */
export const SETTING_DEFAULTS = {
  graphic_profile: 3,
  dynamic_graphics_enabled: true,
  dynamic_skybox: true,
  skybox_time: 43200,
  audio_general_volume: 100,
  audio_scene_volume: 100,
  audio_ui_volume: 100,
  audio_music_volume: 100,
  audio_avatar_and_emotes_volume: 100,
  // Kept so the engine value survives, not shown in the UI.
  audio_voice_chat_volume: 100,
  audio_mic_amplification: 100,
  submit_message_closes_chat: false,
  show_own_nametag: false,
  locale: 'en',
  /** Index into CACHE_SIZES_GB; legacy default is 2 GB. */
  max_cache_size: 1,
  // Session flags: not persisted by the engine.
  camera_mode: 'third' as CameraMode,
  hide_interface: false,
  hide_player_names: false,
  hide_view_profile: true,
  hide_interaction_prompts: true,
  hide_scene_interface: true,
  // Dev tools.
  scene_logs: false,
  raycast_debugger: false,
  scene_processing: true,
  realm: '',
  websocket_url: 'http://127.0.0.1:8000/',
  process_tick_quota: 100,
} as const;

export type CameraMode = 'first' | 'third';

type Widen<T> = T extends boolean
  ? boolean
  : T extends number
  ? number
  : T extends string
  ? string
  : T;

export type SettingKey = keyof typeof SETTING_DEFAULTS;
export type SettingsValues = {
  [K in SettingKey]: K extends 'camera_mode'
    ? CameraMode
    : Widen<(typeof SETTING_DEFAULTS)[K]>;
};

/** Hidden in the UI (revamp dropped them) but still in the table. */
export const HIDDEN_KEYS: SettingKey[] = [
  'audio_voice_chat_volume',
  'audio_mic_amplification',
  'hide_scene_interface',
];

export const CACHE_SIZES_GB = [1, 2, 4] as const;

/** 1..3 = Low, Medium, High; Very Low and Custom are not offered. */
export const GRAPHIC_PROFILES = [1, 2, 3] as const;

/** Seconds of day for the Custom Skybox list, in display order. */
export const SKYBOX_TIMES = [86400, 64800, 43200, 21600] as const;
export const SKYBOX_ON_DYNAMIC_OFF = 21600;

export const LOCALES = ['en', 'es', 'pt_BR'] as const;

export const defaultSettings = (): SettingsValues => ({
  ...SETTING_DEFAULTS,
});

/** Runtime state the engine owns: locks set by scenes and which screens exist. */
export type SettingsEnv = {
  inWorld: boolean;
  skyboxLocked: boolean;
  cameraLocked: boolean;
  deviceUnsupported: boolean;
  /** Dev Tools and the pseudo-locale only exist outside production. */
  devBuild: boolean;
  /** In a world, where Gameplay offers to replay the tutorial. */
  tutorialReplay: boolean;
};

export const defaultEnv = (): SettingsEnv => ({
  inWorld: false,
  skyboxLocked: false,
  cameraLocked: false,
  deviceUnsupported: false,
  devBuild: typeof __DEV__ !== 'undefined' && __DEV__,
  tutorialReplay: false,
});

export type AccountInfo = {
  name: string;
  /** `#1234`, empty for claimed names. */
  tag: string;
  address: string;
  guest: boolean;
  email: string | null;
};

export type VersionInfo = { short: string; full: string };
