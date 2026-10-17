import { ReactElement } from 'react';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { NavProvider, useNav } from '../src/app/NavProvider';
import { clearScreens, getScreen } from '../src/app/registry';
import { ToastProvider } from '../src/ds';
import { createMockApi, mockApi, setProfileApi } from '../src/profile/api';
import { ProfileEditRoute, ProfileTab } from '../src/profile/ProfileScreens';
import {
  getProfileState,
  loadProfile,
  resetProfileStore,
} from '../src/profile/store';
import { previewIds } from '../src/gallery/registry';

jest.mock('../src/GodotBridge', () => ({
  sendCommand: jest.fn(),
  startEngine: jest.fn(),
  setOrientation: jest.fn(),
  subscribe: jest.fn(() => () => {}),
}));

const metrics = {
  frame: { x: 0, y: 0, width: 400, height: 900 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};
const mounted: ReactTestRenderer[] = [];
const flush = () =>
  act(async () => {
    await Promise.resolve();
  });

async function mount(node: ReactElement) {
  let tree!: ReactTestRenderer;
  await act(async () => {
    tree = create(
      <SafeAreaProvider initialMetrics={metrics}>
        <ToastProvider>
          <NavProvider>{node}</NavProvider>
        </ToastProvider>
      </SafeAreaProvider>,
    );
  });
  mounted.push(tree);
  return tree;
}

const press = async (tree: ReactTestRenderer, label: string) => {
  const nodes = tree.root
    .findAllByProps({ accessibilityLabel: label })
    .filter(n => typeof n.props.onPress === 'function');
  await act(async () => nodes[nodes.length - 1].props.onPress());
};

afterEach(() => {
  act(() => mounted.splice(0).forEach(tree => tree.unmount()));
  setProfileApi(mockApi);
  resetProfileStore();
});

describe('registration', () => {
  it('plugs the profile tab and edit route into the registry', () => {
    clearScreens();
    require('../src/profile');
    expect(getScreen('profile')).toBeDefined();
    expect(getScreen('profileEdit')).toBeDefined();
  });
});

describe('ProfileTab', () => {
  it('loads the profile, copies the address and opens the editor', async () => {
    const api = createMockApi();
    setProfileApi(api);
    let nav!: ReturnType<typeof useNav>;
    const Probe = () => {
      nav = useNav();
      return null;
    };
    const tree = await mount(
      <>
        <Probe />
        <ProfileTab params={undefined as never} />
      </>,
    );
    await flush();
    expect(JSON.stringify(tree.toJSON())).toContain('Namemaxcharacte');
    await press(tree, 'Copy address');
    expect(api.copied).toEqual([MOCK_ADDRESS]);
    await press(tree, 'Copy name');
    expect(api.copied[1]).toBe('Namemaxcharacte#1234');
    await press(tree, 'EDIT PROFILE');
    expect(nav.route.name).toBe('profileEdit');
  });

  it('offers a retry when the load fails', async () => {
    const api = createMockApi();
    setProfileApi({ ...api, getProfile: () => Promise.reject(new Error('x')) });
    const tree = await mount(<ProfileTab params={undefined as never} />);
    await flush();
    expect(JSON.stringify(tree.toJSON())).toContain('TRY AGAIN');
    setProfileApi(api);
    await press(tree, 'TRY AGAIN');
    await flush();
    expect(JSON.stringify(tree.toJSON())).toContain('Namemaxcharacte');
  });
});

const MOCK_ADDRESS = '0x4e1a5c5eb2A8a7aA7B0a09E04f1b2a1c13B';

describe('ProfileEditRoute', () => {
  it('renders nothing before the profile is loaded', async () => {
    const tree = await mount(<ProfileEditRoute />);
    expect(JSON.stringify(tree.toJSON())).not.toContain('Edit Profile');
  });

  it('saves a patch, pops the route and deploys', async () => {
    const api = createMockApi();
    setProfileApi(api);
    await loadProfile();
    let nav!: ReturnType<typeof useNav>;
    const Probe = () => {
      nav = useNav();
      return null;
    };
    const tree = await mount(
      <>
        <Probe />
        <ProfileEditRoute />
      </>,
    );
    await act(async () => nav.push({ name: 'profileEdit' }));
    const description = tree.root
      .findAll(
        n =>
          n.props.accessibilityLabel === 'Description' && n.props.onChangeText,
      )
      .at(0);
    await act(async () => description?.props.onChangeText('Hi'));
    await press(tree, 'SAVE');
    await flush();
    expect(nav.route.name).toBe('home');
    expect(api.deployed).toEqual([{ description: 'Hi' }]);
    expect(getProfileState().profile?.description).toBe('Hi');
  });
});

describe('gallery previews', () => {
  it('registers every non-duplicate Figma state of the area', () => {
    require('../src/profile/gallery');
    const ids = previewIds().filter(
      id => id.startsWith('profile__') || id === 'menu__profile__533-5757',
    );
    expect(ids).toHaveLength(39);
  });

  it('every preview renders', () => {
    const { getPreview } = require('../src/gallery/registry');
    for (const id of previewIds().filter(
      i => i.startsWith('profile__') || i === 'menu__profile__533-5757',
    )) {
      let tree!: ReactTestRenderer;
      act(() => {
        tree = create(
          <SafeAreaProvider initialMetrics={metrics}>
            {getPreview(id).render()}
          </SafeAreaProvider>,
        );
      });
      expect(tree.toJSON()).not.toBeNull();
      act(() => tree.unmount());
    }
  });
});
