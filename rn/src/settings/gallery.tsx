import '../hud/gallery';
import { useMemo } from 'react';
import { BottomNav } from '../ds';
import { registerAlias, registerPreview } from '../gallery/registry';
import { createMockApi, MockSeed } from './mockApi';
import type { PreviewState } from './SectionBody';
import type { SectionKey } from './sections';
import { SettingsScreen } from './SettingsScreen';

type Cfg = {
  title: string;
  section: SectionKey | null;
  seed?: MockSeed;
  preview?: PreviewState;
  landscape?: boolean;
  /** Pushed variant without the bottom nav. */
  noNav?: boolean;
};

const OLD_VERSION = { short: '1.10.0', full: '1.10.0' };
const noop = () => {};

function Preview({ cfg }: { cfg: Cfg }) {
  const api = useMemo(
    () =>
      createMockApi({
        ...cfg.seed,
        env: { devBuild: false, ...cfg.seed?.env },
      }),
    [cfg.seed],
  );
  return (
    <SettingsScreen
      api={api}
      initialSection={cfg.section}
      preview={cfg.preview}
      footer={
        cfg.noNav ? undefined : (
          <BottomNav active="settings" bottomInset={0} onChange={noop} />
        )
      }
    />
  );
}

const R = 'settings-revamp__';
const P = 'settings-portrait__';
const values = (v: NonNullable<MockSeed['values']>): MockSeed => ({
  values: v,
});
const gfxOff = values({ dynamic_graphics_enabled: false });
const skyOff = values({ dynamic_skybox: false, skybox_time: 21600 });
const chat = values({ submit_message_closes_chat: true });
const inWorld = (env: MockSeed['env'], v = chat): MockSeed => ({
  ...v,
  env: { inWorld: true, ...env },
});
const audio = (general: number, scene: number): MockSeed =>
  values({ audio_general_volume: general, audio_scene_volume: scene });
const cache = (gb: number, bytes = 5e8): MockSeed => ({
  values: { max_cache_size: gb },
  cacheBytes: bytes,
});

const ENTRIES: Record<string, Cfg> = {
  [`${R}settings-graphics__1397-6501`]: {
    title: 'Graphics',
    section: 'graphics',
  },
  [`${R}settings-audio__1397-6829`]: { title: 'Audio', section: 'audio' },
  [`${R}settings-gameplay__1397-6961`]: {
    title: 'Gameplay',
    section: 'gameplay',
  },
  [`${R}settings-language__1397-7138`]: {
    title: 'Language',
    section: 'language',
  },
  [`${R}settings-language__1401-1032`]: {
    title: 'Language',
    section: 'language',
  },
  [`${R}settings-language-dropdown-open__1401-1082`]: {
    title: 'Language: open',
    section: 'language',
    preview: { open: 'list' },
  },
  [`${R}settings-language__1401-4359`]: {
    title: 'Language: Español',
    section: 'language',
    seed: values({ locale: 'es' }),
  },
  [`${R}settings-account-section-menu__1397-6528`]: {
    title: 'Root list',
    section: null,
  },
  [`${R}settings-account-section-menu__1401-1132`]: {
    title: 'Root list (1.10.0)',
    section: null,
    seed: { version: OLD_VERSION },
  },
  [`${R}settings-account-guest-upgrade__1398-7315`]: {
    title: 'Account: guest',
    section: 'account',
  },
  [`${R}settings-account-upgraded__1398-7378`]: {
    title: 'Account: upgraded',
    section: 'account',
    seed: { account: { guest: false, email: 'name@email.com' } },
  },
  [`${R}settings-help__1398-7818`]: {
    title: 'Help & Support: unsupported device',
    section: 'help',
    seed: { env: { deviceUnsupported: true } },
  },
  [`${R}settings-help-temporary-files-dev-tools__1398-8062`]: {
    title: 'Storage',
    section: 'storage',
    seed: cache(0),
  },
  [`${R}settings-language-landscape__1401-4513`]: {
    title: 'Language (landscape)',
    section: 'language',
    landscape: true,
    seed: { version: OLD_VERSION },
  },
  [`${R}settings-language-landscape-dropdown-open__1401-4572`]: {
    title: 'Language open (landscape)',
    section: 'language',
    landscape: true,
    preview: { open: 'list' },
    seed: { version: OLD_VERSION },
  },
  [`${R}settings-language-landscape__1401-4790`]: {
    title: 'Language Español (landscape)',
    section: 'language',
    landscape: true,
    seed: { values: { locale: 'es' }, version: OLD_VERSION },
  },
  [`${P}settings-back__851-10490`]: {
    title: 'Graphics: pushed, no nav',
    section: 'graphics',
    noNav: true,
  },
  [`${P}settings-graphics__348-1584`]: {
    title: 'Graphics: default',
    section: 'graphics',
  },
  [`${P}settings-graphics__290-2035`]: {
    title: 'Graphics: dynamic off',
    section: 'graphics',
    seed: gfxOff,
  },
  [`${P}settings-graphics__348-1986`]: {
    title: 'Graphics: profile open (High)',
    section: 'graphics',
    seed: gfxOff,
    preview: { open: 'profile' },
  },
  [`${P}settings-graphics__718-6559`]: {
    title: 'Graphics: profile open (Medium)',
    section: 'graphics',
    seed: values({ dynamic_graphics_enabled: false, graphic_profile: 2 }),
    preview: { open: 'profile' },
  },
  [`${P}settings-graphics__718-6713`]: {
    title: 'Graphics: Medium',
    section: 'graphics',
    seed: values({ dynamic_graphics_enabled: false, graphic_profile: 2 }),
  },
  [`${P}settings-audio__303-2221`]: { title: 'Audio: 100%', section: 'audio' },
  [`${P}settings-audio__738-7766`]: {
    title: 'Audio: 50% / 0%',
    section: 'audio',
    seed: audio(50, 0),
  },
  [`${P}settings-gameplay__1348-4487`]: {
    title: 'Gameplay: chat off',
    section: 'gameplay',
  },
  [`${P}settings-gameplay__1348-4378`]: {
    title: 'Gameplay: chat on',
    section: 'gameplay',
    seed: chat,
  },
  [`${P}settings-gameplay__1348-4615`]: {
    title: 'Gameplay: camera first person',
    section: 'gameplay',
    seed: inWorld(
      undefined,
      values({ submit_message_closes_chat: true, camera_mode: 'first' }),
    ),
  },
  [`${P}settings-gameplay__1349-4751`]: {
    title: 'Gameplay: camera third person',
    section: 'gameplay',
    seed: inWorld(undefined),
  },
  [`${P}settings-gameplay__1350-4893`]: {
    title: 'Gameplay: camera locked',
    section: 'gameplay',
    seed: inWorld({ cameraLocked: true }),
  },
  [`${P}settings-storage__348-1419`]: {
    title: 'Storage: 0.5 GB of 1 GB',
    section: 'storage',
    seed: cache(0),
  },
  [`${P}settings-storage__412-4560`]: {
    title: 'Storage: empty',
    section: 'storage',
    seed: cache(0, 0),
  },
  [`${P}settings-dev-tools__381-3096`]: {
    title: 'Dev tools',
    section: 'dev',
    seed: {
      env: { devBuild: true },
      values: { raycast_debugger: true, scene_logs: true },
    },
  },
  [`${P}settings-graphics__718-5548`]: {
    title: 'Graphics: custom skybox',
    section: 'graphics',
    seed: skyOff,
  },
  [`${P}settings-graphics__718-5766`]: {
    title: 'Graphics: skybox open (Morning)',
    section: 'graphics',
    seed: skyOff,
    preview: { open: 'skybox' },
  },
  [`${P}settings-graphics__738-7476`]: {
    title: 'Graphics: skybox open (Midday)',
    section: 'graphics',
    seed: values({ dynamic_skybox: false, skybox_time: 43200 }),
    preview: { open: 'skybox' },
  },
  [`${P}settings-graphics__738-6935`]: {
    title: 'Graphics: skybox Midday',
    section: 'graphics',
    seed: values({ dynamic_skybox: false, skybox_time: 43200 }),
  },
  [`${P}settings-gameplay__1348-4266`]: {
    title: 'Gameplay: everything on',
    section: 'gameplay',
    seed: values({
      submit_message_closes_chat: true,
      hide_interface: true,
      hide_player_names: true,
    }),
  },
  [`${P}settings-account-singout__412-4763`]: {
    title: 'Sign out dialog',
    section: 'account',
    preview: { dialog: 'signOut' },
  },
  [`${P}settings-storage__412-4288`]: {
    title: 'Storage: max cache open (1 GB)',
    section: 'storage',
    seed: cache(0),
    preview: { open: 'list' },
  },
  [`${P}settings-storage__738-8796`]: {
    title: 'Storage: max cache open (2 GB)',
    section: 'storage',
    seed: cache(1),
    preview: { open: 'list' },
  },
  [`${P}settings-storage__738-9027`]: {
    title: 'Storage: 2 GB',
    section: 'storage',
    seed: cache(1),
  },
  [`${P}settings-graphics__1099-5103`]: {
    title: 'Graphics: skybox locked by creator',
    section: 'graphics',
    seed: {
      env: { skyboxLocked: true },
      values: { dynamic_graphics_enabled: true },
    },
  },
  [`${P}settings-account-delete__432-1708`]: {
    title: 'Delete account dialog',
    section: 'account',
    preview: { dialog: 'delete' },
  },
};

for (const [id, cfg] of Object.entries(ENTRIES)) {
  registerPreview(id, {
    title: `Settings: ${cfg.title}`,
    orientation: cfg.landscape ? 'landscape' : 'portrait',
    render: () => <Preview cfg={cfg} />,
    settleMs: 600,
  });
}

const SETTINGS_NOTE =
  'Older account tab (report/help/sign out); the settings revamp account section is authoritative';
[`${P}settings-account__303-3251`, `${P}settings-account__738-7985`].forEach(
  id =>
    registerAlias(
      id,
      `${R}settings-account-upgraded__1398-7378`,
      'portrait',
      SETTINGS_NOTE,
    ),
);
registerAlias(
  `${P}thirdperson__851-10734`,
  'hud__hud__609-7021',
  'landscape',
  'In-world HUD behind the settings flow; same screen as the HUD preview',
);
registerAlias(
  `${P}place__969-4359`,
  'hud__hideui__706-1414',
  'landscape',
  'In-world with the UI hidden (Show UI pill)',
);
