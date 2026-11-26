import { ReactElement } from 'react';
import { Keyboard, NativeModules, Platform } from 'react-native';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { HomeTabs } from '../src/app/HomeTabs';
import { NavProvider } from '../src/app/NavProvider';
import { EntryFlow } from '../src/flow/EntryFlow';
import { core } from '../src/core';
import { signInWithWallet, WalletError } from '../src/flow/wallet';

jest.mock('../src/flow/wallet', () => ({
  ...jest.requireActual('../src/flow/wallet'),
  signInWithWallet: jest.fn(() => new Promise(() => {})),
}));
const walletSignIn = signInWithWallet as jest.Mock;

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
      // On a device the avatar hole has opened the preview by now: the state is not `SignedIn`.
      act(() =>
        core().showAvatarPreview({ x: 0, y: 0, width: 10, height: 10 }),
      );
      expect(core().state().tag).toBe('AvatarPreview');
      await act(async () =>
        tree.root.findByProps({ accessibilityLabel: 'Back' }).props.onPress(),
      );
      expect(core().state().tag).toBe('SignedOut');
      expect(JSON.stringify(tree.toJSON())).toContain('PLAY AS GUEST');
      await playAsGuest();
      expect(JSON.stringify(tree.toJSON())).toContain('Choose your look');
    } finally {
      jest.useRealTimers();
    }
  });

  describe('from Sign In', () => {
    const os = Platform.OS;
    let tree: ReactTestRenderer;
    const press = (label: string) =>
      act(async () =>
        tree.root.findByProps({ accessibilityLabel: label }).props.onPress(),
      );

    beforeEach(async () => {
      if (core().state().tag !== 'SignedOut') {
        act(() => core().signOut());
      }
      jest.useFakeTimers();
      tree = mount(<EntryFlow onDone={() => {}} />);
      await act(async () => {
        jest.advanceTimersByTime(1500);
      });
      await act(async () => {});
      await press('SIGN IN');
    });

    afterEach(() => {
      jest.useRealTimers();
      jest.restoreAllMocks();
      Platform.OS = os;
      act(() => core().cancelSignIn());
    });

    const page = () => JSON.stringify(tree.toJSON());

    it('signs in from the wallet list on Wallet Connect, without the auth site', async () => {
      const signInBrowser = jest.spyOn(core(), 'signInBrowser');
      await press('Wallet Connect');
      expect(walletSignIn).toHaveBeenLastCalledWith(
        'wallet_connect',
        expect.any(Function),
      );
      expect(signInBrowser).not.toHaveBeenCalled();
      expect(page()).toContain('your Wallet...');
      await act(async () => walletSignIn.mock.lastCall[1]('signing'));
      expect(page()).toContain('Confirm in');
    });

    it('opens MetaMask straight from its own button on Android', async () => {
      await press('Back');
      Platform.OS = 'android';
      await press('SIGN IN');
      expect(page()).not.toContain('Wallet Connect');
      await press('MetaMask');
      expect(walletSignIn).toHaveBeenLastCalledWith(
        'metamask',
        expect.any(Function),
      );
      expect(page()).toContain('MetaMask...');
    });

    it('welcomes a returning account back once the wallet has signed', async () => {
      walletSignIn.mockImplementationOnce(async () => {
        core().signInWalletBegin();
        core().signInWalletComplete('0xwallet', '0xsignature');
      });
      await press('Wallet Connect');
      await act(async () => {
        jest.advanceTimersByTime(1000);
      });
      expect(core().state().tag).toBe('SignedIn');
      expect(page()).toContain('Welcome back');
    });

    it('shows the refusal on Sign In when the wallet rejects', async () => {
      walletSignIn.mockImplementationOnce(() =>
        Promise.reject(new WalletError('rejected')),
      );
      await press('Wallet Connect');
      expect(page()).toContain('Sign in to Decentraland');
      expect(page()).toContain('The request was rejected in your Wallet.');
    });

    it('tells the player MetaMask is missing', async () => {
      walletSignIn.mockImplementationOnce(() =>
        Promise.reject(new WalletError('not_installed')),
      );
      await press('Back');
      Platform.OS = 'android';
      await press('SIGN IN');
      await press('MetaMask');
      expect(page()).toContain('MetaMask is not installed.');
    });

    it('comes back to Sign In without an error when the wallet list is closed', async () => {
      walletSignIn.mockImplementationOnce(() =>
        Promise.reject(new WalletError('cancelled')),
      );
      await press('Wallet Connect');
      expect(page()).toContain('Sign in to Decentraland');
      expect(page()).not.toContain('Try again');
    });

    it('closes the keyboard before Back leaves a screen', async () => {
      const dismiss = jest.spyOn(Keyboard, 'dismiss');
      await press('Continue with email');
      await press('Back');
      expect(dismiss).toHaveBeenCalled();
    });

    it('releases the email field before the code dialog opens on Android', async () => {
      Platform.OS = 'android';
      const dismiss = jest.spyOn(Keyboard, 'dismiss');
      await press('Continue with email');
      const field = tree.root.findByProps({ placeholder: 'name@email.com' });
      act(() => field.props.onChangeText('qa@example.com'));
      await press('NEXT');
      await act(async () => {
        jest.advanceTimersByTime(1000);
      });
      expect(dismiss).toHaveBeenCalledTimes(1);
      expect(JSON.stringify(tree.toJSON())).toContain(
        'Enter verification code',
      );
    });
  });
});
