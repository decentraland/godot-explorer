import { ReactElement } from 'react';
import { NativeModules, Platform } from 'react-native';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { HomeTabs } from '../src/app/HomeTabs';
import { NavProvider } from '../src/app/NavProvider';
import { EntryFlow } from '../src/flow/EntryFlow';

const metrics = {
  frame: { x: 0, y: 0, width: 400, height: 900 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

const mounted: ReactTestRenderer[] = [];

function mount(node: ReactElement): ReactTestRenderer {
  let renderer!: ReactTestRenderer;
  act(() => {
    renderer = create(
      <SafeAreaProvider initialMetrics={metrics}>{node}</SafeAreaProvider>,
    );
  });
  mounted.push(renderer);
  return renderer;
}

describe('screens render', () => {
  afterEach(() => {
    act(() => mounted.splice(0).forEach(tree => tree.unmount()));
  });

  beforeEach(() => {
    jest.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('offline'));
  });

  it('shows the connection lost state when the catalog cannot load', async () => {
    const tree = mount(
      <NavProvider>
        <HomeTabs onJumpIn={() => {}} />
      </NavProvider>,
    );
    await act(async () => {});
    expect(JSON.stringify(tree.toJSON())).toContain('Connection lost');
  });

  it('starts the entry flow on the splash screen', () => {
    const tree = mount(<EntryFlow onDone={() => {}} />);
    expect(tree.toJSON()).not.toBeNull();
  });

  it('asks for the notification permission at the Play as Guest tap', async () => {
    const os = Platform.OS;
    Platform.OS = 'android';
    const push = {
      hasPermission: jest.fn().mockResolvedValue(false),
      requestPermission: jest.fn().mockResolvedValue(true),
    };
    NativeModules.DclPush = push;
    jest.useFakeTimers();
    try {
      const tree = mount(<EntryFlow onDone={() => {}} />);
      await act(async () => {
        jest.advanceTimersByTime(1500);
      });
      await act(async () => {});
      await act(async () =>
        tree.root
          .findByProps({ accessibilityLabel: 'PLAY AS GUEST' })
          .props.onPress(),
      );
      expect(push.requestPermission).toHaveBeenCalledTimes(1);
    } finally {
      jest.useRealTimers();
      Platform.OS = os;
      delete NativeModules.DclPush;
    }
  });

  it('plays as guest again after backing out of Avatar Create', async () => {
    jest.useFakeTimers();
    try {
      const tree = mount(<EntryFlow onDone={() => {}} />);
      const playAsGuest = async () => {
        await act(async () =>
          tree.root
            .findByProps({ accessibilityLabel: 'PLAY AS GUEST' })
            .props.onPress(),
        );
        await act(async () => {
          jest.advanceTimersByTime(500);
        });
        await act(async () => {});
      };
      await act(async () => {
        jest.advanceTimersByTime(1500);
      });
      await act(async () => {});
      await playAsGuest();
      expect(JSON.stringify(tree.toJSON())).toContain('Choose your look');
      await act(async () =>
        tree.root.findByProps({ accessibilityLabel: 'Back' }).props.onPress(),
      );
      expect(JSON.stringify(tree.toJSON())).toContain('PLAY AS GUEST');
      await playAsGuest();
      expect(JSON.stringify(tree.toJSON())).toContain('Choose your look');
    } finally {
      jest.useRealTimers();
    }
  });
});
