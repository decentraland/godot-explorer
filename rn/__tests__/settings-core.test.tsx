import { act, create } from 'react-test-renderer';
import { setCoreForTests } from '../src/core';
import type { CoreApi } from '../src/core';
import { resetCoreEventsForTests } from '../src/core/events';
import { createMockCore } from '../src/core/mock';
import { createMockSettings } from '../src/core/mock/settings';
import { SettingKey, SettingValue } from '../src/core/native';
import type { AppListener, SettingEntry } from '../src/core/native';
import { createCoreSettingsApi } from '../src/settings/coreApi';
import { SettingsModel, useSettings } from '../src/settings/useSettings';
import { devRealms, helpUrl, legalLinks } from '../src/settings/links';

const URLS = {
  help: 'https://decentraland.zone/help',
  privacyPolicy: 'https://decentraland.zone/privacy/',
  contentPolicy: 'https://decentraland.zone/content/',
  termsOfService: 'https://decentraland.zone/terms/',
  devRealms: ['main', 'https://realm.test'],
} as CoreApi['urls'] extends () => infer U ? U : never;

const PROFILE = {
  address: '0x4e1c0b2f9d3a7e5c8b6a41d2e0f3c9a8b7d61c13B',
  name: 'Ann',
  hasClaimedName: false,
} as Awaited<ReturnType<CoreApi['profile']>>;

type Harness = {
  core: CoreApi;
  setCalls: Array<[SettingKey, SettingValue, boolean]>;
  emit: (entry: SettingEntry) => void;
};

function setup(overrides: Partial<CoreApi> = {}): Harness {
  const listeners = new Set<Partial<AppListener>>();
  const settings = createMockSettings();
  const setCalls: Harness['setCalls'] = [];
  const core = {
    ...createMockCore(),
    ...settings,
    setSetting: (key: SettingKey, value: SettingValue, commit: boolean) => {
      setCalls.push([key, value, commit]);
      return settings.setSetting(key, value, commit);
    },
    urls: () => URLS,
    profile: async () => PROFILE,
    signOut: jest.fn(),
    addListener: (l: AppListener) => {
      listeners.add(l);
      return BigInt(listeners.size);
    },
    removeListener: () => {},
    ...overrides,
  } as CoreApi;
  setCoreForTests(core);
  resetCoreEventsForTests();
  return {
    core,
    setCalls,
    emit: entry => listeners.forEach(l => l.settingsChanged?.(entry)),
  };
}

afterEach(() => {
  setCoreForTests(undefined);
  resetCoreEventsForTests();
});

describe('core settings api', () => {
  it('lays engine values over the shell defaults and drops empty text', async () => {
    const h = setup();
    h.core.setSetting(
      SettingKey.AudioGeneralVolume,
      new SettingValue.Float(40),
      true,
    );
    h.core.setSetting(SettingKey.MaxCacheSize, new SettingValue.Int(2n), true);
    const all = await createCoreSettingsApi().getAll();
    expect(all.audio_general_volume).toBe(40);
    expect(all.max_cache_size).toBe(2);
    expect(all.locale).toBe('en');
    expect(all.camera_mode).toBe('third');
    expect('view_distance' in all).toBe(false);
  });

  it('sends engine keys typed and keeps session flags local', async () => {
    const h = setup();
    const api = createCoreSettingsApi();
    await api.set('audio_music_volume', 12, false);
    await api.set('dynamic_skybox', false);
    await api.set('locale', 'es');
    await api.set('hide_player_names', true);
    expect(h.setCalls).toHaveLength(3);
    expect(h.setCalls[0][0]).toBe(SettingKey.AudioMusicVolume);
    expect(h.setCalls[0][1]).toEqual(new SettingValue.Int(12n));
    expect(h.setCalls[0][2]).toBe(false);
    expect(h.setCalls[1][1]).toEqual(new SettingValue.Bool(false));
    expect(h.setCalls[2][1]).toEqual(new SettingValue.Text('es'));
    const all = await api.getAll();
    expect(all.audio_music_volume).toBe(12);
    expect(all.locale).toBe('es');
    expect(all.hide_player_names).toBe(true);
  });

  it('reports cache, version, account and sign-out from the core', async () => {
    const h = setup();
    const api = createCoreSettingsApi();
    expect(await api.getCacheSize()).toBe(5e8);
    await api.clearCache();
    expect(await api.getCacheSize()).toBe(0);
    expect((await api.getVersion()).full).toBe('1.14.0-mock');
    expect(await api.getAccount()).toMatchObject({ name: 'Ann', tag: '#c13B' });
    await api.signOut();
    expect(h.core.signOut).toHaveBeenCalled();
  });

  it('turns a failed account deletion into a result', async () => {
    setup({
      deleteAccount: async () => {
        throw new Error('nope');
      },
    });
    expect(await createCoreSettingsApi().deleteAccount()).toEqual({
      ok: false,
      error: 'nope',
    });
  });

  it('notifies subscribers on settingsChanged until they unsubscribe', () => {
    const h = setup();
    const listener = jest.fn();
    const off = createCoreSettingsApi().subscribe(listener);
    h.emit({ key: SettingKey.Locale, value: new SettingValue.Text('es') });
    expect(listener).toHaveBeenCalledTimes(1);
    off();
    h.emit({ key: SettingKey.Locale, value: new SettingValue.Text('en') });
    expect(listener).toHaveBeenCalledTimes(1);
  });
});

describe('settings model over the core', () => {
  function mountModel(onError: () => void) {
    const ref: { m: SettingsModel | null } = { m: null };
    const api = createCoreSettingsApi();
    function Probe() {
      ref.m = useSettings(api, onError);
      return null;
    }
    return {
      ref,
      mount: () =>
        act(async () => {
          create(<Probe />);
        }),
    };
  }

  it('updates optimistically and rolls back with an error callback on failure', async () => {
    const h = setup();
    const onError = jest.fn();
    const { ref, mount } = mountModel(onError);
    await mount();
    await act(async () => {
      ref.m?.set('audio_ui_volume', 30);
    });
    expect(ref.m?.values.audio_ui_volume).toBe(30);
    expect(onError).not.toHaveBeenCalled();

    (h.core as { setSetting: unknown }).setSetting = () => {
      throw new Error('nope');
    };
    await act(async () => {
      ref.m?.set('audio_ui_volume', 5);
    });
    expect(onError).toHaveBeenCalledTimes(1);
    expect(ref.m?.values.audio_ui_volume).toBe(30);
  });

  it('reloads when the core announces a change', async () => {
    const h = setup();
    const { ref, mount } = mountModel(jest.fn());
    await mount();
    h.core.setSetting(
      SettingKey.GraphicProfile,
      new SettingValue.Int(1n),
      true,
    );
    await act(async () => {
      h.emit({
        key: SettingKey.GraphicProfile,
        value: new SettingValue.Int(1n),
      });
    });
    expect(ref.m?.values.graphic_profile).toBe(1);
  });
});

describe('links from the core urls', () => {
  it('reads help, legal and dev realms from core().urls()', () => {
    setup();
    expect(helpUrl()).toBe(URLS.help);
    expect(legalLinks().map(l => l.url)).toEqual([
      URLS.privacyPolicy,
      URLS.contentPolicy,
      URLS.termsOfService,
    ]);
    expect(devRealms()).toEqual(URLS.devRealms);
  });
});
