import { ReactElement } from 'react';
import { BackHandler, Dimensions } from 'react-native';
import { act, create, ReactTestInstance } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import type { CoreApi } from '../src/core';
import { LinkRow, OutlinedButton } from '../src/ds';
import { en } from '../src/i18n/en';
import { createMockApi, MockSeed } from '../src/settings/mockApi';
import { SettingsScreen } from '../src/settings/SettingsScreen';
import type { SectionKey } from '../src/settings/sections';
import type { PreviewState } from '../src/settings/SectionBody';
import { setLocale } from '../src/i18n';

const metrics = {
  frame: { x: 0, y: 0, width: 360, height: 640 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

type Tree = ReturnType<typeof create>;
const mounted: Tree[] = [];

beforeEach(() => {
  const urls = {
    help: 'https://decentraland.org/help',
    privacyPolicy: 'https://decentraland.org/privacy/',
    contentPolicy: 'https://decentraland.org/content/',
    termsOfService: 'https://decentraland.org/terms/',
    devRealms: ['main', 'https://realm.test'],
  };
  setCoreForTests({
    ...createMockCore(),
    urls: () => urls,
  } as unknown as CoreApi);
});

afterEach(() => {
  setCoreForTests(undefined);
  mounted.splice(0).forEach(t => act(() => t.unmount()));
});

async function mount(
  seed: MockSeed = {},
  section: SectionKey | null = null,
  extra: {
    preview?: PreviewState;
    onSignedOut?: () => void;
    onClose?: () => void;
  } = {},
) {
  const api = createMockApi({ env: { devBuild: false }, ...seed });
  let tree!: Tree;
  const element: ReactElement = (
    <SafeAreaProvider initialMetrics={metrics}>
      <SettingsScreen api={api} initialSection={section} {...extra} />
    </SafeAreaProvider>
  );
  await act(async () => {
    tree = create(element);
  });
  mounted.push(tree);
  return { api, tree };
}

const text = (tree: Tree) => JSON.stringify(tree.toJSON());

const host = (tree: Tree, props: Record<string, unknown>) =>
  tree.root.findAll(
    (n: ReactTestInstance) =>
      typeof n.type === 'string' &&
      Object.entries(props).every(([k, v]) => n.props[k] === v),
  );

/** Presses the first (or, for dialog buttons, the last) pressable with this label. */
function press(tree: Tree, label: string, last = false) {
  const all = tree.root
    .findAllByProps({ accessibilityLabel: label })
    .filter((n: ReactTestInstance) => typeof n.props.onPress === 'function');
  const node = last ? all[all.length - 1] : all[0];
  if (!node) {
    throw new Error(`no pressable "${label}"`);
  }
  return act(async () => {
    node.props.onPress();
  });
}

const portrait = () =>
  Dimensions.set({
    window: { width: 360, height: 640, scale: 1, fontScale: 1 },
  });
const landscape = () =>
  Dimensions.set({
    window: { width: 900, height: 400, scale: 1, fontScale: 1 },
  });

beforeEach(portrait);

afterEach(() => setLocale('en'));

describe('portrait master-detail', () => {
  it('lists the seven sections and the version, with no Dev Tools', async () => {
    const { tree } = await mount();
    const out = text(tree);
    for (const key of [
      'SETTINGS_GRAPHICS',
      'SETTINGS_AUDIO',
      'SETTINGS_GAMEPLAY',
      'SETTINGS_LANGUAGE',
      'SETTINGS_ACCOUNT',
      'SETTINGS_HELP_SUPPORT',
      'SETTINGS_STORAGE',
    ] as const) {
      expect(out).toContain(en[key]);
    }
    expect(out).not.toContain('Dev Tools');
    expect(out).toContain('Version 1.14.0');
    expect(tree.root.findAllByType(LinkRow)).toHaveLength(7);
  });

  it('opens a detail, shows its title, and goes back', async () => {
    const { tree } = await mount();
    await press(tree, 'Audio');
    expect(text(tree)).toContain('VOLUME');
    await press(tree, 'Back');
    expect(text(tree)).toContain('Version 1.14.0');
  });

  it('handles the hardware back button inside a detail', async () => {
    let handler: (() => boolean) | undefined;
    jest.spyOn(BackHandler, 'addEventListener').mockImplementation(((
      _: string,
      cb: () => boolean,
    ) => {
      handler = cb;
      return { remove: jest.fn() };
    }) as never);
    const { tree } = await mount({}, 'storage');
    expect(text(tree)).toContain('TEMPORARY FILES');
    await act(async () => {
      expect(handler?.()).toBe(true);
    });
    expect(text(tree)).toContain('Version 1.14.0');
    jest.restoreAllMocks();
  });

  it('shows Dev Tools in a dev build and mounts its section', async () => {
    const root = await mount({ env: { devBuild: true } });
    expect(text(root.tree)).toContain('Dev Tools');
    expect(text(root.tree)).toContain('1.14.0-mock');
    await press(root.tree, 'Dev Tools');
    expect(text(root.tree)).toContain('Raycast Debugger');
  });

  it('copies the full version and toasts', async () => {
    const { api, tree } = await mount();
    await press(tree, 'Copy version');
    expect(api.calls).toContain('copy:1.14.0-mock');
  });
});

describe('Graphics', () => {
  it('disables the profile while Dynamic Graphics is on and persists a toggle', async () => {
    const { api, tree } = await mount({}, 'graphics');
    const profile = tree.root.findAllByProps({
      accessibilityLabel: 'Graphic Profiles',
    })[0];
    expect(profile.props.accessibilityState.disabled).toBe(true);
    await press(tree, 'Dynamic Graphics');
    expect(api.calls).toContain('set:dynamic_graphics_enabled=false');
  });

  it('picks a profile when dynamic graphics is off', async () => {
    const { api, tree } = await mount(
      { values: { dynamic_graphics_enabled: false } },
      'graphics',
      { preview: { open: 'profile' } },
    );
    await press(tree, 'Medium');
    expect(api.calls).toContain('set:graphic_profile=2');
  });

  it('turning Dynamic Skybox off selects Morning and shows the preview', async () => {
    const { api, tree } = await mount({}, 'graphics');
    expect(host(tree, { testID: 'skybox-preview' })).toHaveLength(0);
    await press(tree, 'Dynamic Skybox');
    expect(api.calls).toEqual(
      expect.arrayContaining([
        'set:dynamic_skybox=false',
        'set:skybox_time=21600',
      ]),
    );
    expect(host(tree, { testID: 'skybox-preview' })).toHaveLength(1);
  });

  it('shows the creator lock banner and disables the skybox controls', async () => {
    const { tree } = await mount({ env: { skyboxLocked: true } }, 'graphics');
    expect(text(tree)).toContain('The skybox is set by the creator');
    const toggle = tree.root.findAllByProps({
      accessibilityLabel: 'Dynamic Skybox',
    })[0];
    expect(toggle.props.accessibilityState.disabled).toBe(true);
  });
});

describe('Audio and Gameplay', () => {
  it('renders the five volume sliders only', async () => {
    const { tree } = await mount(
      { values: { audio_general_volume: 50, audio_scene_volume: 0 } },
      'audio',
    );
    const out = text(tree);
    expect(out).toContain('50%');
    expect(out).toContain('0%');
    expect(out).not.toContain('Voice Chat');
    expect(host(tree, { accessibilityRole: 'adjustable' })).toHaveLength(5);
  });

  it('sets a volume live then commits it', async () => {
    const { api, tree } = await mount({}, 'audio');
    const slider = tree.root.findAllByProps({
      accessibilityLabel: 'General',
    })[0];
    await act(async () => {
      slider.props.onAccessibilityAction({
        nativeEvent: { actionName: 'decrement' },
      });
    });
    expect(api.calls).toContain('set:audio_general_volume=99:live');
  });

  it('disables the interface children until Hide Interface is on', async () => {
    const { api, tree } = await mount({}, 'gameplay');
    const child = () =>
      tree.root.findAllByProps({
        accessibilityLabel: 'Hide “View Profile”',
      })[0];
    expect(child().props.accessibilityState.disabled).toBe(true);
    await press(tree, 'Hide Interface');
    expect(api.calls).toContain('set:hide_interface=true');
    expect(child().props.accessibilityState.disabled).toBe(false);
  });

  it('shows the camera group only in the world, locked with a banner', async () => {
    const outside = await mount({}, 'gameplay');
    expect(text(outside.tree)).not.toContain('Camera Mode');
    const inside = await mount(
      { env: { inWorld: true, cameraLocked: true } },
      'gameplay',
    );
    expect(text(inside.tree)).toContain('Camera Mode');
    expect(text(inside.tree)).toContain(
      'The camera mode is set by the creator',
    );
  });
});

describe('Language, Help, Storage', () => {
  it('changes the language from the open list', async () => {
    const { api, tree } = await mount({}, 'language', {
      preview: { open: 'list' },
    });
    expect(text(tree)).toContain('Português (Brasil)');
    await press(tree, 'Español');
    expect(api.calls).toContain('set:locale=es');
  });

  it('shows the unsupported banner and routes the rows', async () => {
    const { api, tree } = await mount(
      { env: { deviceUnsupported: true } },
      'help',
    );
    expect(text(tree)).toContain('Device no longer supported');
    await press(tree, 'Report Bug');
    await press(tree, 'Report Content');
    expect(api.calls).toEqual(['bugReport', 'reportContent']);
    const rows = tree.root.findAllByType(LinkRow);
    expect(rows).toHaveLength(7);
  });

  it('opens external links through Linking', async () => {
    const { Linking } = jest.requireActual('react-native');
    const spy = jest.spyOn(Linking, 'openURL').mockResolvedValue(true);
    const { tree } = await mount({}, 'help');
    await press(tree, 'Discord');
    await press(tree, 'Term Of Service');
    expect(spy).toHaveBeenCalledWith(expect.stringContaining('discord.com'));
    expect(spy).toHaveBeenCalledWith('https://decentraland.org/terms/');
    spy.mockRestore();
  });

  it('shows the cache and cleans it, then disables the button', async () => {
    const { api, tree } = await mount(
      { values: { max_cache_size: 0 } },
      'storage',
    );
    expect(text(tree)).toContain('0.5 GB');
    await press(tree, 'CLEAN CACHE');
    expect(api.calls).toContain('clearCache');
    expect(text(tree)).toContain('0 MB');
    expect(tree.root.findByType(OutlinedButton).props.disabled).toBe(true);
  });

  it('commits a new max cache size', async () => {
    const { api, tree } = await mount({}, 'storage', {
      preview: { open: 'list' },
    });
    await press(tree, '4 GB');
    expect(api.calls).toContain('set:max_cache_size=2');
  });
});

describe('Account', () => {
  it('shows the guest card and starts the email upgrade', async () => {
    const { api, tree } = await mount({}, 'account');
    const out = text(tree);
    expect(out).toContain('Unlock the full experience');
    expect(out).toContain('#1234');
    await press(tree, 'ADD EMAIL');
    expect(api.calls).toContain('addEmail');
  });

  it('shows the email field for upgraded accounts', async () => {
    const { tree } = await mount(
      { account: { guest: false, email: 'name@email.com', tag: '' } },
      'account',
    );
    const out = text(tree);
    expect(out).toContain('name@email.com');
    expect(out).not.toContain('Unlock the full experience');
  });

  it('confirms sign out through the dialog', async () => {
    const onSignedOut = jest.fn();
    const { api, tree } = await mount({}, 'account', { onSignedOut });
    await press(tree, 'SIGN OUT');
    await press(tree, 'SIGN OUT', true);
    expect(api.calls).toContain('signOut');
    expect(onSignedOut).toHaveBeenCalled();
  });

  it('cancels a dialog without side effects', async () => {
    const { api, tree } = await mount({}, 'account', {
      preview: { dialog: 'signOut' },
    });
    expect(text(tree)).toContain('signed out of your account');
    await press(tree, 'CANCEL');
    expect(api.calls).toEqual([]);
  });

  it('deletes after confirmation and reports a failure', async () => {
    const onSignedOut = jest.fn();
    const { api, tree } = await mount({}, 'account', {
      preview: { dialog: 'delete' },
      onSignedOut,
    });
    expect(text(tree)).toContain('cannot be undone');
    await press(tree, 'DELETE');
    expect(api.calls).toContain('deleteAccount');
    expect(onSignedOut).toHaveBeenCalled();

    const failing = createMockApi();
    failing.deleteAccount = async () => ({ ok: false, error: 'x' });
    let t2!: Tree;
    await act(async () => {
      t2 = create(
        <SafeAreaProvider initialMetrics={metrics}>
          <SettingsScreen
            api={failing}
            initialSection="account"
            preview={{ dialog: 'delete' }}
          />
        </SafeAreaProvider>,
      );
    });
    mounted.push(t2);
    await press(t2, 'DELETE');
    expect(text(t2)).toContain('Could not delete account');
  });

  it('opens the delete dialog from the link', async () => {
    const { tree } = await mount({}, 'account');
    await press(tree, 'Delete your account?');
    expect(text(tree)).toContain('Delete account');
  });
});

describe('landscape split', () => {
  beforeEach(landscape);

  it('shows the list and the selected section together, no title', async () => {
    const { tree } = await mount({}, null);
    const out = text(tree);
    expect(out).toContain('Graphics');
    expect(out).toContain('PERFORMANCE');
    expect(out).toContain('Version 1.14.0');
    const selected = tree.root
      .findAllByType(LinkRow)
      .filter(r => r.props.selected);
    expect(selected).toHaveLength(1);
  });

  it('swaps the right pane on tap and closes with the chevron', async () => {
    const onClose = jest.fn();
    const { tree } = await mount({}, 'language', { onClose });
    expect(text(tree)).toContain('App Language');
    await press(tree, 'Audio');
    expect(text(tree)).toContain('VOLUME');
    await press(tree, 'Back');
    expect(onClose).toHaveBeenCalled();
  });
});

describe('Dev Tools and camera', () => {
  it('drives the dev actions through the api', async () => {
    const { api, tree } = await mount({ env: { devBuild: true } }, 'dev');
    await press(tree, 'Scene Logs');
    await press(tree, 'TEST NOTIFICATIONS');
    await press(tree, 'CONNECT');
    expect(api.calls).toEqual(
      expect.arrayContaining([
        'set:scene_logs=true',
        'dev:test_notification',
        'set:websocket_url=http://127.0.0.1:8000/',
        'dev:connect_websocket',
      ]),
    );
  });

  it('switches the camera mode in the world', async () => {
    const { api, tree } = await mount({ env: { inWorld: true } }, 'gameplay');
    await press(tree, 'Camera Mode');
    await press(tree, 'First person');
    expect(api.calls).toContain('set:camera_mode=first');
  });
});
