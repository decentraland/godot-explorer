import {
  cacheFraction,
  formatCacheSize,
  isSplitLayout,
  shortenAddress,
} from '../src/settings/format';
import {
  CACHE_SIZES_GB,
  defaultSettings,
  HIDDEN_KEYS,
  SETTING_DEFAULTS,
} from '../src/settings/keys';
import { createMockApi } from '../src/settings/mockApi';
import { dependentChanges } from '../src/settings/rules';
import { sectionsFor } from '../src/settings/sections';
import { getSettingsApi, setSettingsApi } from '../src/settings/api';

describe('key table', () => {
  it('follows the legacy defaults', () => {
    const d = defaultSettings();
    expect(CACHE_SIZES_GB[d.max_cache_size]).toBe(2);
    expect(d.graphic_profile).toBe(3);
    expect(d.dynamic_graphics_enabled).toBe(true);
    expect(d.skybox_time).toBe(43200);
    expect(d.audio_general_volume).toBe(100);
  });

  it('keeps the hidden keys in the table', () => {
    HIDDEN_KEYS.forEach(k => expect(k in SETTING_DEFAULTS).toBe(true));
  });

  it('returns a fresh object each time', () => {
    expect(defaultSettings()).not.toBe(defaultSettings());
  });
});

describe('format', () => {
  it('shows zero and small caches in MB and bigger ones in GB', () => {
    expect(formatCacheSize(0)).toBe('0 MB');
    expect(formatCacheSize(5e7)).toBe('50 MB');
    expect(formatCacheSize(5e8)).toBe('0.5 GB');
    expect(formatCacheSize(2.25e9)).toBe('2.3 GB');
  });

  it('computes the share of the cap, clamped', () => {
    expect(cacheFraction(5e8, 1)).toBeCloseTo(0.5);
    expect(cacheFraction(5e9, 1)).toBe(1);
    expect(cacheFraction(-1, 1)).toBe(0);
    expect(cacheFraction(1, 0)).toBe(0);
  });

  it('shortens long addresses only', () => {
    expect(shortenAddress('0x4e1c0b2f9d3a7e5c8b6a41d2e0f3c9a8b7d61c13B')).toBe(
      '0x4e1…1c13B',
    );
    expect(shortenAddress('0x1234')).toBe('0x1234');
  });

  it('splits only wide landscape windows', () => {
    expect(isSplitLayout(400, 800)).toBe(false);
    expect(isSplitLayout(800, 400)).toBe(true);
    expect(isSplitLayout(500, 400)).toBe(false);
  });
});

describe('rules and sections', () => {
  it('selects Morning when Dynamic Skybox turns off', () => {
    expect(dependentChanges('dynamic_skybox', false)).toEqual({
      skybox_time: 21600,
    });
    expect(dependentChanges('dynamic_skybox', true)).toEqual({});
    expect(dependentChanges('locale', 'es')).toEqual({});
  });

  it('adds Dev Tools only outside production', () => {
    expect(sectionsFor(false).map(s => s.key)).toEqual([
      'graphics',
      'audio',
      'gameplay',
      'language',
      'account',
      'help',
      'storage',
    ]);
    expect(sectionsFor(true).pop()?.key).toBe('dev');
  });
});

describe('mock api', () => {
  it('stores values and records calls', async () => {
    const api = createMockApi({ values: { locale: 'es' } });
    expect((await api.getAll()).locale).toBe('es');
    await api.set('audio_ui_volume', 40, false);
    await api.set('audio_ui_volume', 40);
    expect((await api.getAll()).audio_ui_volume).toBe(40);
    expect(api.calls).toEqual([
      'set:audio_ui_volume=40:live',
      'set:audio_ui_volume=40',
    ]);
  });

  it('clears the cache and reports the account and version', async () => {
    const api = createMockApi();
    expect(await api.getCacheSize()).toBe(5e8);
    await api.clearCache();
    expect(await api.getCacheSize()).toBe(0);
    expect((await api.getAccount()).guest).toBe(true);
    expect((await api.getVersion()).short).toBe('1.14.0');
    expect(await api.deleteAccount()).toEqual({ ok: true, deleted: true });
  });

  it('notifies subscribers until they unsubscribe', async () => {
    const api = createMockApi();
    const listener = jest.fn();
    const off = api.subscribe(listener);
    api.emitChange({ skyboxLocked: true });
    expect((await api.getEnv()).skyboxLocked).toBe(true);
    off();
    api.emitChange();
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it('records the remaining actions', async () => {
    const api = createMockApi();
    await api.copyText('v');
    await api.signOut();
    await api.startEmailUpgrade();
    await api.openBugReport();
    await api.openReportContent();
    await api.devAction('test_notification');
    expect(api.calls).toEqual([
      'copy:v',
      'signOut',
      'addEmail',
      'bugReport',
      'reportContent',
      'dev:test_notification',
    ]);
  });
});

describe('active api', () => {
  afterEach(() => setSettingsApi(null));

  it('is swappable and falls back to a mock', () => {
    const fallback = getSettingsApi();
    expect(getSettingsApi()).toBe(fallback);
    const other = createMockApi();
    setSettingsApi(other);
    expect(getSettingsApi()).toBe(other);
  });
});
