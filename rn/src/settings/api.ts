import { createCoreSettingsApi } from './coreApi';
import type {
  AccountInfo,
  SettingKey,
  SettingsEnv,
  SettingsValues,
  VersionInfo,
} from './keys';

export type DevAction = 'test_notification' | 'connect_websocket';

/** What the Settings screens need from the engine. */
export interface SettingsApi {
  getAll(): Promise<SettingsValues>;
  /** `commit` false applies the value without persisting it (sliders while dragging). */
  set<K extends SettingKey>(
    key: K,
    value: SettingsValues[K],
    commit?: boolean,
  ): Promise<void>;
  getEnv(): Promise<SettingsEnv>;
  getAccount(): Promise<AccountInfo>;
  /** Bytes in the cache folder. */
  getCacheSize(): Promise<number>;
  clearCache(): Promise<void>;
  getVersion(): Promise<VersionInfo>;
  copyText(text: string): Promise<void>;
  /** Starts the guest-to-email upgrade flow (OTP, owned by the engine). */
  startEmailUpgrade(): Promise<void>;
  signOut(): Promise<void>;
  deleteAccount(): Promise<{ ok: boolean; error?: string }>;
  openBugReport(): Promise<void>;
  openReportContent(): Promise<void>;
  devAction(action: DevAction): Promise<void>;
  /** Engine-driven changes (profile picked by dynamic graphics, scene locks) call the listener. */
  subscribe(listener: () => void): () => void;
}

let active: SettingsApi | null = null;

export function setSettingsApi(api: SettingsApi | null): void {
  active = api;
}

/** The registered implementation, else the core's (the native binding, or its mock without the engine). */
export function getSettingsApi(): SettingsApi {
  if (!active) {
    active = createCoreSettingsApi();
  }
  return active;
}
