import { act, create } from 'react-test-renderer';
import type { GodotCommand } from '../src/GodotBridge';
import type { GodotEvent } from '../src/events';
import { createMockBridge } from '../src/godot/mockBridge';
import { clearMockHandlers } from '../src/bridge/mockHandlers';
import { resetRequests } from '../src/bridge/request';
import { createBridgeSettingsApi } from '../src/settings/bridgeApi';
import { registerSettingsMockHandlers } from '../src/settings/mockHandlers';
import { SettingsModel, useSettings } from '../src/settings/useSettings';
import { createBridgeProfileApi } from '../src/profile/bridgeApi';
import { registerProfileMockHandlers } from '../src/profile/mockHandlers';
import {
  getProfileState,
  loadProfile,
  resetProfileStore,
  saveProfile,
} from '../src/profile/store';
import { mockApi, setProfileApi } from '../src/profile/api';

// The real request layer talks to the mock bridge, which answers from the registered handlers.
let mockListeners: Array<(event: GodotEvent) => void> = [];
let mockBridge = createMockBridge();
jest.mock('../src/GodotBridge', () => ({
  godotAvailable: false,
  sendCommand: (command: GodotCommand) => mockBridge.sendCommand(command),
  subscribe: (listener: (event: GodotEvent) => void) => {
    mockListeners.push(listener);
    const off = mockBridge.subscribe(listener);
    return () => {
      mockListeners = mockListeners.filter(l => l !== listener);
      off();
    };
  },
}));

beforeEach(() => {
  clearMockHandlers();
  resetRequests();
  mockListeners = [];
  mockBridge = createMockBridge();
});

afterEach(() => {
  setProfileApi(mockApi);
  resetProfileStore();
});

function mountSettings(onError: () => void) {
  const api = createBridgeSettingsApi();
  const ref: { m: SettingsModel | null } = { m: null };
  function Probe() {
    ref.m = useSettings(api, onError);
    return null;
  }
  return {
    ref,
    api,
    mount: () =>
      act(async () => {
        create(<Probe />);
      }),
  };
}

describe('bridge settings api', () => {
  it('loads engine values over defaults and the version, cache and account', async () => {
    registerSettingsMockHandlers({
      audio_general_volume: 40,
      view_distance: 9,
    });
    registerProfileMockHandlers();
    const api = createBridgeSettingsApi();
    const all = await api.getAll();
    expect(all.audio_general_volume).toBe(40);
    expect(all.camera_mode).toBe('third');
    expect('view_distance' in all).toBe(false);
    expect(await api.getCacheSize()).toBe(5e8);
    expect((await api.getVersion()).full).toBe('1.14.0-mock');
    expect((await api.getAccount()).tag).toBe('#c13B');
  });

  it('persists engine keys and keeps session flags local', async () => {
    const engine = registerSettingsMockHandlers();
    const api = createBridgeSettingsApi();
    await api.set('audio_music_volume', 12);
    await api.set('hide_player_names', true);
    expect(engine.values.audio_music_volume).toBe(12);
    expect(engine.values.hide_player_names).toBe(false);
    const all = await api.getAll();
    expect(all.audio_music_volume).toBe(12);
    expect(all.hide_player_names).toBe(true);
  });

  it('updates optimistically and rolls back with an error callback on failure', async () => {
    const engine = registerSettingsMockHandlers();
    registerProfileMockHandlers();
    const failing = jest.fn(() => {
      throw new Error('nope');
    });
    const onError = jest.fn();
    const { ref, mount } = mountSettings(onError);
    await mount();
    expect(ref.m?.values.audio_ui_volume).toBe(100);

    await act(async () => {
      ref.m?.set('audio_ui_volume', 30);
    });
    expect(ref.m?.values.audio_ui_volume).toBe(30);
    expect(engine.values.audio_ui_volume).toBe(30);
    expect(onError).not.toHaveBeenCalled();

    const { registerMockHandler } = require('../src/bridge/mockHandlers');
    registerMockHandler('settings.set', failing);
    await act(async () => {
      ref.m?.set('audio_ui_volume', 5);
    });
    for (let i = 0; i < 20 && ref.m?.values.audio_ui_volume !== 30; i += 1) {
      await act(async () => {
        await new Promise(resolve => setTimeout(resolve, 5));
      });
    }
    expect(onError).toHaveBeenCalledTimes(1);
    expect(ref.m?.values.audio_ui_volume).toBe(30);
  });

  it('notifies subscribers on settings_changed only', () => {
    const api = createBridgeSettingsApi();
    const listener = jest.fn();
    api.subscribe(listener);
    mockListeners.forEach(l => l({ type: 'ready', data: {} } as GodotEvent));
    expect(listener).not.toHaveBeenCalled();
    mockListeners.forEach(l =>
      l({ type: 'settings_changed', data: { key: 'locale', value: 'es' } }),
    );
    expect(listener).toHaveBeenCalledTimes(1);
  });
});

describe('bridge profile api', () => {
  it('maps the engine profile and deploys camelCase patches as snake_case fields', async () => {
    const engine = registerProfileMockHandlers();
    const api = createBridgeProfileApi();
    const profile = await api.getProfile();
    expect(profile.name).toBe('Namemaxcharacte');
    expect(profile.tag).toBe('c13B');
    expect(await api.getNames()).toEqual(['Name 01', 'Name']);
    const next = await api.deployProfile({
      realName: 'Ann',
      sexualOrientation: 'x',
    });
    expect(engine.data.real_name).toBe('Ann');
    expect(next.realName).toBe('Ann');
    expect(next.sexualOrientation).toBe('x');
  });

  it('store: optimistic save succeeds, and failure rolls back', async () => {
    registerProfileMockHandlers();
    setProfileApi(createBridgeProfileApi());
    await loadProfile();
    expect(getProfileState().profile?.description).toBe(
      'Hello from the mock bridge.',
    );
    await expect(saveProfile({ description: 'new' })).resolves.toBe(true);
    expect(getProfileState().profile?.description).toBe('new');

    clearMockHandlers();
    registerProfileMockHandlers({ failDeploy: true });
    await expect(saveProfile({ description: 'bad' })).resolves.toBe(false);
    expect(getProfileState().profile?.description).toBe('new');
  });

  it('store: load error surfaces as error status', async () => {
    setProfileApi(createBridgeProfileApi());
    await loadProfile();
    expect(getProfileState().status).toBe('error');
  });
});
