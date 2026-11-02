import { act, create } from 'react-test-renderer';
import { BackHandler, Text } from 'react-native';
import { NavProvider, useNav, Nav } from '../src/app/NavProvider';
import { clearScreens, getScreen, registerScreen } from '../src/app/registry';
import { HomeTabs } from '../src/app/HomeTabs';
import { SafeAreaProvider } from 'react-native-safe-area-context';

jest.mock('../src/backpack/BackpackScreen', () => ({
  BackpackScreen: () => null,
}));
jest.mock('../src/discover/DiscoverScreen', () => ({
  DiscoverScreen: () => null,
}));

let nav!: Nav;
function Probe() {
  nav = useNav();
  return null;
}

const metrics = {
  frame: { x: 0, y: 0, width: 360, height: 640 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

function mount(children: React.ReactNode) {
  let tree!: ReturnType<typeof create>;
  act(() => {
    tree = create(
      <SafeAreaProvider initialMetrics={metrics}>
        <NavProvider>
          <Probe />
          {children}
        </NavProvider>
      </SafeAreaProvider>,
    );
  });
  return tree;
}

afterEach(clearScreens);

describe('registry', () => {
  it('returns what was registered', () => {
    const C = () => null;
    registerScreen('profileEdit', C);
    expect(getScreen('profileEdit')).toBe(C);
    expect(getScreen('eventDetail')).toBeUndefined();
  });
});

describe('HomeTabs', () => {
  it('shows the stub for settings, then the registered screen', () => {
    const tree = mount(<HomeTabs onJumpIn={() => {}} />);
    act(() => nav.setTab('settings'));
    expect(JSON.stringify(tree.toJSON())).toContain('Coming soon');
    registerScreen('settings', () => <Text>REAL_SETTINGS</Text>);
    act(() => nav.setTab('profile'));
    act(() => nav.setTab('settings'));
    expect(JSON.stringify(tree.toJSON())).toContain('REAL_SETTINGS');
  });
});

describe('hardware back', () => {
  it('pops a pushed route, then falls through at the root', () => {
    const handlers: Array<() => boolean> = [];
    jest.spyOn(BackHandler, 'addEventListener').mockImplementation(((
      _: string,
      h: () => boolean,
    ) => {
      handlers.push(h);
      return { remove: () => {} };
    }) as never);
    mount(null);
    act(() => nav.push({ name: 'profileEdit' }));
    const press = () => {
      let handled = false;
      act(() => {
        handled = handlers[handlers.length - 1]();
      });
      return handled;
    };
    expect(press()).toBe(true);
    expect(nav.state.stack).toHaveLength(1);
    expect(press()).toBe(false);
  });
});
