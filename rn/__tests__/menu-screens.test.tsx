import { ReactElement } from 'react';
import { act, create, ReactTestInstance } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { getPreview, previewIds } from '../src/gallery/registry';
import '../src/menu/gallery';
import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { MenuDrawer } from '../src/menu/MenuDrawer';
import { setMenuApi } from '../src/menu/api';
import {
  closeMenu,
  openMenu,
  resetDrawerForTests,
} from '../src/menu/drawerState';
import { createMockApi } from '../src/menu/mockApi';
import '../src/menu/register';
import { getScreen } from '../src/app/registry';
import { NavProvider, useNav } from '../src/app/NavProvider';
import { shareScene } from '../src/menu/share';
import { Share } from 'react-native';

const enterWorld = jest.fn();

const metrics = {
  frame: { x: 0, y: 0, width: 800, height: 360 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

const wrap = (el: ReactElement) => (
  <SafeAreaProvider initialMetrics={metrics}>{el}</SafeAreaProvider>
);

function mount(el: ReactElement) {
  let tree!: ReturnType<typeof create>;
  act(() => {
    tree = create(wrap(el));
  });
  return tree;
}

const settle = () =>
  act(async () => {
    await new Promise(resolve => setTimeout(resolve, 20));
  });

const json = (tree: ReturnType<typeof create>) => JSON.stringify(tree.toJSON());

function press(tree: ReturnType<typeof create>, label: string) {
  const node = tree.root
    .findAll(
      (n: ReactTestInstance) =>
        n.props.accessibilityLabel === label &&
        typeof n.props.onPress === 'function',
    )
    .shift();
  if (!node) {
    throw new Error(`no pressable "${label}"`);
  }
  act(() => node.props.onPress());
}

describe('menu previews', () => {
  const ids = previewIds().filter(
    id => id.startsWith('menu__') || id.startsWith('extra/menu-'),
  );

  it('registers the menu states', () => {
    expect(ids.length).toBeGreaterThanOrEqual(40);
  });

  it.each(ids)('%s renders', async id => {
    const tree = mount(getPreview(id)!.render());
    await settle();
    expect(json(tree).length).toBeGreaterThan(100);
    act(() => tree.unmount());
  });

  it('shows the specific states', async () => {
    const text = async (id: string) => {
      const tree = mount(getPreview(id)!.render());
      await settle();
      const out = json(tree);
      act(() => tree.unmount());
      return out;
    };
    expect(await text('menu__social__933-26017')).toContain('ONLINE (2)');
    expect(await text('menu__social__933-25219')).toContain('No friends yet');
    expect(await text('menu__social-nearby-empty__931-24777')).toContain(
      'No one nearby',
    );
    expect(await text('menu__notifications__426-6356')).toContain('READ ALL');
    expect(await text('extra/menu-notifications-guest')).toContain(
      'Sign in to get notifications!',
    );
    expect(await text('extra/menu-notifications-empty')).toContain(
      'No notifications yet',
    );
    expect(await text('menu__discover-event-set__1525-5303')).toContain(
      'REMINDER SET',
    );
    expect(await text('menu__discover-event-reminder__1525-5237')).toContain(
      'REMIND ME',
    );
    expect(await text('extra/menu-event-guest')).toContain(
      'SIGN IN TO USE REMINDERS',
    );
    expect(await text('extra/menu-bug-sent')).toContain(
      'Bug Report Submitted!',
    );
  });
});

describe('MenuDrawer', () => {
  beforeEach(() => {
    resetDrawerForTests();
    setMenuApi(createMockApi());
    enterWorld.mockClear();
    setCoreForTests({ ...createMockCore(), enterWorld });
  });
  afterAll(() => {
    setMenuApi(null);
    setCoreForTests(undefined);
  });

  it('renders nothing while closed', () => {
    const tree = mount(<MenuDrawer navigate={jest.fn()} />);
    expect(json(tree)).not.toContain('Social');
  });

  it('opens on Discover, loads the scene and closes on a world tap', async () => {
    const tree = mount(<MenuDrawer navigate={jest.fn()} />);
    act(() => openMenu('discover'));
    await settle();
    expect(json(tree)).toContain('Genesis Plaza PlazaPlaza');
    press(tree, 'Close');
    expect(json(tree)).not.toContain('Genesis Plaza PlazaPlaza');
  });

  it('leaves the drawer for the fullscreen pages', async () => {
    const navigate = jest.fn();
    const tree = mount(<MenuDrawer navigate={navigate} />);
    act(() => openMenu('discover'));
    await settle();
    press(tree, 'Settings');
    expect(navigate).toHaveBeenCalledWith('settings');
    act(() => openMenu('discover'));
    press(tree, 'Backpack');
    expect(navigate).toHaveBeenCalledWith('backpack');
    act(() => openMenu('discover'));
    press(tree, 'Profile');
    expect(navigate).toHaveBeenCalledWith('profile');
    act(() => closeMenu());
  });

  it('jumps in from the place modal by entering the place', async () => {
    const tree = mount(<MenuDrawer navigate={jest.fn()} />);
    act(() => openMenu('discover'));
    await settle();
    press(tree, 'Genesis Plaza');
    press(tree, 'JUMP IN');
    expect(enterWorld).toHaveBeenCalledWith({
      realm: undefined,
      parcel: { x: 0, y: 0 },
    });
  });

  it('accepts a friend request and marks notifications read', async () => {
    const tree = mount(<MenuDrawer navigate={jest.fn()} />);
    act(() => openMenu('social'));
    await settle();
    press(tree, 'REQUESTS');
    expect(json(tree)).toContain('RECEIVED (2)');
    press(tree, 'Accept');
    await settle();
    expect(json(tree)).toContain('RECEIVED (1)');

    act(() => openMenu('notifications'));
    await settle();
    press(tree, 'READ ALL');
    await settle();
    const readAll = tree.root.findAll(
      n => n.props.accessibilityLabel === 'READ ALL' && n.props.onPress,
    )[0];
    expect(readAll.props.accessibilityState.disabled).toBe(true);
  });

  it('opens the bug report from the kebab and restores landscape on close', async () => {
    const tree = mount(<MenuDrawer navigate={jest.fn()} />);
    act(() => openMenu('discover'));
    await settle();
    press(tree, 'More options');
    press(tree, 'Report bug');
    await settle();
    expect(json(tree)).toContain('Bug Report');
    press(tree, 'CANCEL');
    expect(json(tree)).not.toContain('Bug Report');
  });

  it('shows the portrait Discover from EXPLORE MORE', async () => {
    const tree = mount(<MenuDrawer navigate={jest.fn()} />);
    act(() => openMenu('discover'));
    await settle();
    press(tree, 'Explore more'.toUpperCase());
    expect(json(tree)).toContain('Last Played');
  });
});

describe('world drawer slot', () => {
  it('registers itself and pushes the fullscreen pages on the route stack', async () => {
    const Slot = getScreen('worldDrawer')!;
    expect(Slot).toBeDefined();
    let nav: ReturnType<typeof useNav> | undefined;
    const Probe = () => {
      nav = useNav();
      return null;
    };
    resetDrawerForTests();
    setMenuApi(createMockApi());
    const tree = mount(
      <NavProvider>
        <Probe />
        <Slot params={undefined as never} />
      </NavProvider>,
    );
    act(() => openMenu('discover'));
    await settle();
    press(tree, 'Settings');
    expect(nav?.route.name).toBe('settings');
  });
});

describe('share', () => {
  it('opens the OS share sheet with the scene message', async () => {
    const spy = jest
      .spyOn(Share, 'share')
      .mockResolvedValue({ action: Share.dismissedAction });
    await shareScene({
      title: 'Plaza',
      creator: '',
      linkQuery: 'position=0,0',
    });
    expect(spy.mock.calls[0][0].message).toContain('Join Me At Plaza');
    spy.mockRejectedValueOnce(new Error('cancelled'));
    await expect(
      shareScene({ title: 'Plaza', creator: '', linkQuery: 'position=0,0' }),
    ).resolves.toBeUndefined();
  });
});
