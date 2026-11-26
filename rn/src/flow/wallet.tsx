import { useSyncExternalStore } from 'react';
import { Linking } from 'react-native';
import { core } from '../core';

/** The project of 1.x's native WalletConnect (dashboard.reown.com). */
const PROJECT_ID = '71f57f4190df9de6326bd07de6c40dcc';
/** The link the deep-link router swallows: a wallet returning to the app opens it. */
const RETURN_LINK = 'decentraland://walletconnect';
const METAMASK = { name: 'MetaMask', href: 'metamask://' }; // i18n-ignore
/** The key the WalletConnect client reads to reopen the wallet for a request. */
const DEEPLINK_CHOICE = 'WALLETCONNECT_DEEPLINK_CHOICE';

export type WalletKind = 'metamask' | 'wallet_connect';
export type WalletPhase = 'connecting' | 'signing';

/** Why a wallet sign-in stopped; `cancelled` is the player backing out. */
export type WalletErrorCode =
  | 'cancelled'
  | 'not_installed'
  | 'rejected'
  | 'failed';

export class WalletError extends Error {
  readonly code: WalletErrorCode;
  constructor(code: WalletErrorCode, detail?: unknown) {
    super(detail instanceof Error ? detail.message : code);
    this.code = code;
  }
}

type Sdk = typeof import('@reown/appkit-react-native');
type Kit = {
  instance: ReturnType<Sdk['createAppKit']>;
  storage: import('@reown/appkit-react-native').Storage;
  core: typeof import('@reown/appkit-core-react-native');
};

let kit: Kit | undefined;
const listeners = new Set<() => void>();

/** Loaded on the first wallet sign-in, so WalletConnect's polyfills and relay socket stay out of
 * every other session. */
function load(): Kit {
  if (kit) {
    return kit;
  }
  const AsyncStorage = (
    require('@react-native-async-storage/async-storage') as typeof import('@react-native-async-storage/async-storage')
  ).default;
  const { createAppKit } = require('@reown/appkit-react-native') as Sdk;
  const { EthersAdapter } =
    require('@reown/appkit-ethers-react-native') as typeof import('@reown/appkit-ethers-react-native');
  const storage: Kit['storage'] = {
    getKeys: async () => [...(await AsyncStorage.getAllKeys())],
    getEntries: async () =>
      (await AsyncStorage.multiGet(await AsyncStorage.getAllKeys())).map(
        ([key, value]) => [key, parse(value)],
      ),
    getItem: async key => parse(await AsyncStorage.getItem(key)),
    setItem: (key, value) => AsyncStorage.setItem(key, JSON.stringify(value)),
    removeItem: key => AsyncStorage.removeItem(key),
  };
  const { host } = core().urls();
  const instance = createAppKit({
    projectId: PROJECT_ID,
    metadata: {
      name: 'Decentraland', // i18n-ignore
      description: 'Decentraland Explorer', // i18n-ignore
      url: host,
      icons: [`${host}/favicon.ico`],
      redirect: { native: RETURN_LINK },
    },
    adapters: [new EthersAdapter()],
    networks: [
      {
        id: 1,
        name: 'Ethereum', // i18n-ignore
        nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 }, // i18n-ignore
        rpcUrls: {
          default: {
            http: [
              `https://rpc.walletconnect.org/v1/?chainId=eip155:1&projectId=${PROJECT_ID}`,
            ],
          },
        },
        chainNamespace: 'eip155',
        caipNetworkId: 'eip155:1',
      },
    ],
    storage,
    enableAnalytics: false,
    features: { socials: false, swaps: false, onramp: false },
    universalProviderConfigOverride: { methods: { eip155: ['personal_sign'] } },
  });
  kit = {
    instance,
    storage,
    core: require('@reown/appkit-core-react-native'),
  };
  listeners.forEach(listener => listener());
  return kit;
}

function parse(value: string | null) {
  if (value === null) {
    return undefined;
  }
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

function subscribe<T extends object, K extends keyof T>(
  state: T,
  key: K,
  callback: (value: T[K]) => void,
): () => void {
  const { subscribeKey } =
    require('valtio/utils') as typeof import('valtio/utils');
  return subscribeKey(state, key, callback);
}

/** AppKit's wallet list, mounted at the root once a wallet sign-in has loaded it. */
export function WalletModal() {
  const loaded = useSyncExternalStore(
    listener => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => kit,
  );
  if (!loaded) {
    return null;
  }
  const { AppKit, AppKitProvider } =
    require('@reown/appkit-react-native') as Sdk;
  return (
    <AppKitProvider instance={loaded.instance}>
      <AppKit />
    </AppKitProvider>
  );
}

let attempt = 0;

/**
 * Signs the core's sign-in message with a wallet: MetaMask opens straight away, `wallet_connect`
 * shows the wallet list. Resolves once the core is signed in; rejects with a `WalletError`.
 */
export async function signInWithWallet(
  kind: WalletKind,
  onPhase: (phase: WalletPhase) => void,
): Promise<void> {
  const message = core().signInWalletBegin();
  const id = ++attempt;
  const current = () => {
    if (id !== attempt) {
      throw new WalletError('cancelled');
    }
  };
  let instance: Kit['instance'] | undefined;
  try {
    instance = load().instance;
    await ready();
    current();
    await instance.disconnect();
    onPhase('connecting');
    const address =
      kind === 'metamask' ? await connectMetaMask() : await connectFromList();
    current();
    onPhase('signing');
    const signature = await sign(message, address);
    current();
    core().signInWalletComplete(address, signature);
  } catch (error) {
    if (id === attempt) {
      core().cancelSignIn();
    }
    throw error instanceof WalletError
      ? error
      : new WalletError(reason(error), error);
  } finally {
    // The session carried one signature: the ephemeral key signs from here on.
    if (id === attempt) {
      instance?.disconnect();
    }
  }
}

/** The player backed out: whatever the wallet answers later is dropped. */
export function cancelWalletSignIn() {
  attempt++;
  kit?.instance.close();
  kit?.instance.disconnect();
}

/** AppKit restores saved sessions at creation; connecting before that ends races its provider. */
function ready(): Promise<void> {
  const { ModalController } = load().core;
  if (!ModalController.state.loading) {
    return Promise.resolve();
  }
  return new Promise(resolve => {
    const off = subscribe(ModalController.state, 'loading', loading => {
      if (!loading) {
        off();
        resolve();
      }
    });
  });
}

/** What AppKit's list does when MetaMask is tapped, without the list. */
async function connectMetaMask(): Promise<string> {
  const { instance, storage, core: sdk } = load();
  const { WcController, CoreHelperUtil, ConnectionsController } = sdk;
  WcController.clearUri();
  let off = () => {};
  const opened = new Promise<void>((resolve, reject) => {
    off = subscribe(WcController.state, 'wcUri', uri => {
      if (uri) {
        off();
        const { redirect } = CoreHelperUtil.formatNativeUrl(METAMASK.href, uri);
        Linking.openURL(redirect).then(resolve, error =>
          reject(new WalletError('not_installed', error)),
        );
      }
    });
  });
  const connected = instance.connect();
  connected.catch(() => {});
  try {
    await Promise.race([opened, connected]);
    await connected;
  } finally {
    off();
  }
  await storage.setItem(DEEPLINK_CHOICE, METAMASK);
  return plainAddress(ConnectionsController.state.activeAddress);
}

function connectFromList(): Promise<string> {
  const { instance, storage, core: sdk } = load();
  const { ModalController, ConnectionsController, WcController } = sdk;
  return new Promise((resolve, reject) => {
    const settle = async () => {
      offAddress();
      offOpen();
      const address = ConnectionsController.state.activeAddress;
      if (!address) {
        reject(new WalletError('cancelled'));
        return;
      }
      // The list saves the wallet to reopen only after its own connect resolves, which can be
      // after the signature request is sent.
      const linking = WcController.state.wcLinking;
      if (linking) {
        await storage.setItem(DEEPLINK_CHOICE, linking);
      }
      try {
        resolve(plainAddress(address));
      } catch (error) {
        reject(error);
      }
    };
    const offAddress = subscribe(
      ConnectionsController.state,
      'activeAddress',
      address => {
        if (address) {
          settle();
        }
      },
    );
    const offOpen = subscribe(ModalController.state, 'open', open => {
      if (!open) {
        settle();
      }
    });
    instance.open({ view: 'Connect' });
  });
}

async function sign(message: string, address: string): Promise<string> {
  const provider = load().instance.getProvider('eip155');
  if (!provider) {
    throw new WalletError('failed');
  }
  const signature = await provider.request(
    { method: 'personal_sign', params: [toHex(message), address] },
    'eip155:1',
  );
  if (typeof signature !== 'string') {
    throw new WalletError('failed');
  }
  return signature;
}

/** `eip155:1:0xabc…` → `0xabc…`. */
export function plainAddress(caip: string | undefined): string {
  const address = caip?.split(':').pop() ?? '';
  if (!/^0x[0-9a-fA-F]{40}$/.test(address)) {
    throw new WalletError('failed');
  }
  return address;
}

/** `personal_sign` takes the message as hex-encoded UTF-8. */
export function toHex(text: string): string {
  const bytes = Array.from(new TextEncoder().encode(text), byte =>
    byte.toString(16).padStart(2, '0'),
  );
  return `0x${bytes.join('')}`;
}

/** The codes WalletConnect and EIP-1193 use when the player refuses in the wallet. */
export function reason(error: unknown): WalletErrorCode {
  const { code, message } = (error ?? {}) as {
    code?: unknown;
    message?: unknown;
  };
  if (
    code === 4001 ||
    code === 5000 ||
    code === 5002 ||
    /reject|denied|disapproved/i.test(String(message ?? ''))
  ) {
    return 'rejected';
  }
  return 'failed';
}
