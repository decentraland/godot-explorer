import { ReactElement, useState } from 'react';
import { Dimensions } from 'react-native';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { CreditsBadge, ItemCard, LinkButton } from '@dcl/ui-native';
import { setCoreForTests } from '../src/core';
import type { CoreApi } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { CameraFocus, ColorTarget, Control } from '../src/core/native';
import { BackpackScreen } from '../src/backpack/BackpackScreen';
import { BackpackState } from '../src/backpack/backpackState';
import { DEFAULT_EMOTES } from '../src/backpack/emotes';
import {
  createMockApi,
  MockApi,
  MockApiOptions,
} from '../src/backpack/mockApi';
import { MOCK_CATALOG, MOCK_WEARABLES } from '../src/backpack/mockCatalog';
import { EmptyCopy, showsCredits } from '../src/backpack/platformPolicy';
import { registerModalApi } from '../src/modals/bridge';

const mockCore = {
  ...createMockCore(),
  showAvatarPreview: jest.fn(),
  hideAvatarPreview: jest.fn(),
  control: jest.fn(),
  avatarState: () => undefined,
} as CoreApi;

beforeEach(() => setCoreForTests(mockCore));

const metrics = {
  frame: { x: 0, y: 0, width: 400, height: 900 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};
const mounted: ReactTestRenderer[] = [];

async function mount(
  options: {
    api?: MockApiOptions;
    initial?: Partial<BackpackState>;
    copy?: EmptyCopy;
    footer?: ReactElement | null;
    subRow?: 'tiles' | 'pills';
    credits?: boolean;
  } = {},
): Promise<{ tree: ReactTestRenderer; api: MockApi }> {
  const api = createMockApi(options.api);
  let tree!: ReactTestRenderer;
  act(() => {
    tree = create(
      <SafeAreaProvider initialMetrics={metrics}>
        <BackpackScreen
          api={api}
          initial={options.initial}
          emptyCopy={options.copy ?? 'marketplace_link'}
          subRow={options.subRow}
          credits={options.credits ?? false}
          footer={options.footer === undefined ? null : options.footer}
        />
      </SafeAreaProvider>,
    );
  });
  mounted.push(tree);
  await act(async () => {});
  return { tree, api };
}

type Json = { children?: (Json | string)[] | null } | string;
type Node = { parent: Node | null; props: Record<string, unknown> };
/** Inside a kept, hidden grid: on screen it is transparent and untouchable. */
const offScreen = (node: Node | null): boolean =>
  node !== null &&
  (node.props.accessibilityElementsHidden === true || offScreen(node.parent));
const collect = (node: Json | Json[] | null): string =>
  node === null
    ? ''
    : Array.isArray(node)
    ? node.map(collect).join(' ')
    : typeof node === 'string'
    ? node
    : (node as { props?: { accessibilityElementsHidden?: boolean } }).props
        ?.accessibilityElementsHidden
    ? ''
    : (node.children ?? []).map(collect).join(' ');
const text = (tree: ReactTestRenderer) =>
  collect(tree.toJSON() as Json | Json[]);
const find = (tree: ReactTestRenderer, label: string) =>
  tree.root.findAll(
    node =>
      node.props.accessibilityLabel === label &&
      typeof node.props.onPress === 'function' &&
      !offScreen(node as unknown as Node),
  )[0];
const press = (tree: ReactTestRenderer, label: string) => {
  const node = find(tree, label);
  if (!node) {
    throw new Error(`no pressable labelled ${label}`);
  }
  act(() => node.props.onPress());
};
const collectChildren = (n: {
  findAll: (
    f: (x: { children: unknown[]; type: unknown }) => boolean,
  ) => { children: unknown[] }[];
}): string =>
  n
    .findAll(x => typeof x.type === 'string')
    .flatMap(x => x.children)
    .filter(c => typeof c === 'string')
    .join(' ');
const pressText = (tree: ReactTestRenderer, label: string) => {
  const node = tree.root.findAll(
    n =>
      typeof n.props.onPress === 'function' &&
      (String(n.props.accessibilityLabel ?? '').includes(label) ||
        collectChildren(n).includes(label)),
  )[0];
  act(() => node.props.onPress());
};

/** Waits for the grids the Backpack mounts hidden after its first frame (at low priority). */
const settleKept = async (tree: ReactTestRenderer) => {
  const kept = () =>
    tree.root.findAll(
      node =>
        typeof node.type === 'string' &&
        node.props.accessibilityElementsHidden === true,
    ).length;
  let last = -1;
  for (let i = 0; i < 40 && (kept() === 0 || kept() !== last); i++) {
    last = kept();
    await act(async () => {
      await new Promise(resolve => setTimeout(resolve, 25));
    });
  }
};

const original = Dimensions.get('window');

afterEach(() => {
  act(() => mounted.splice(0).forEach(tree => tree.unmount()));
  Dimensions.set({ window: original, screen: original });
});

describe('Backpack wearables', () => {
  it('lists the equipped items first with UNEQUIP strips under All', async () => {
    const { tree } = await mount();
    const out = text(tree);
    expect(out).toContain('Wearables');
    expect(out).toContain('Unequip');
    expect(out).toContain('EXTRAS');
  });

  it('keeps every tab grid mounted, so changing tab mounts no new cards', async () => {
    const { tree } = await mount();
    // The other tabs mount hidden after the first frame.
    await settleKept(tree);
    const cards = () => tree.root.findAllByType(ItemCard);
    const before = new Set(cards().map(card => card.instance ?? card));
    const hidden = tree.root.findAll(
      node => typeof node.type === 'string' && node.props.style?.opacity === 0,
    );
    expect(hidden.length).toBeGreaterThanOrEqual(4);
    press(tree, 'CLOTHING');
    await act(async () => {});
    press(tree, 'ALL');
    await act(async () => {});
    expect(cards().length).toBe(before.size);
  });

  it('keeps every slot grid of the open tab mounted, so picking a slot mounts no new cards', async () => {
    const { tree } = await mount({ initial: { tab: 'face' } });
    await settleKept(tree);
    const count = () => tree.root.findAllByType(ItemCard).length;
    const before = count();
    press(tree, 'Eye');
    await act(async () => {});
    press(tree, 'Hair');
    await act(async () => {});
    expect(count()).toBe(before);
  });

  it('draws the grid once the avatar is known, with the worn items first', async () => {
    // Opening from Discover: the catalog is there but no preview has reported the avatar yet.
    const api = createMockApi();
    const real = api.useAvatar;
    let release: (on: boolean) => void = () => {};
    function useHeldAvatar() {
      const avatar = real();
      const [known, setKnown] = useState(false);
      release = setKnown;
      return known ? avatar : null;
    }
    api.useAvatar = useHeldAvatar;
    let tree!: ReactTestRenderer;
    act(() => {
      tree = create(
        <SafeAreaProvider initialMetrics={metrics}>
          <BackpackScreen
            api={api}
            emptyCopy="marketplace_link"
            credits={false}
            footer={null}
          />
        </SafeAreaProvider>,
      );
    });
    mounted.push(tree);
    await act(async () => {});
    const cards = () =>
      tree.root
        .findAllByType(ItemCard)
        .filter(card => !offScreen(card as unknown as Node));
    expect(cards()).toHaveLength(0);
    await act(async () => release(true));
    const first = cards()[0];
    expect(first.props.state).toBe('unequip');
  });

  it('keeps the All order while equipping and unequipping', async () => {
    const { tree, api } = await mount();
    const names = new Set(MOCK_WEARABLES.map(w => w.name));
    const order = () =>
      tree.root
        .findAll(
          n =>
            names.has(n.props.accessibilityLabel) &&
            typeof n.props.onPress === 'function' &&
            !offScreen(n as unknown as Node),
        )
        .map(n => n.props.accessibilityLabel as string)
        .filter((name, i, all) => all.indexOf(name) === i);
    const before = order();
    const item = MOCK_WEARABLES.find(
      w => w.name === before[before.length - 1],
    )!;
    press(tree, item.name);
    await act(async () => {});
    expect(api.calls).toContainEqual(new Control.Equip({ urn: item.id }));
    expect(order()).toEqual(before);
    press(tree, item.name);
    await act(async () => {});
    expect(order()).toEqual(before);
  });

  it('equips on tap and keeps the body shape untouched when it is already worn', async () => {
    const { tree, api } = await mount({ initial: { tab: 'extras' } });
    const hat = MOCK_WEARABLES.find(w => w.category === 'hat')!;
    press(tree, hat.name);
    expect(api.calls).toContainEqual(new Control.Equip({ urn: hat.id }));
    press(tree, hat.name);
    expect(api.calls).toContainEqual(new Control.Unequip({ urn: hat.id }));
    const { tree: body, api: bodyApi } = await mount({
      initial: { tab: 'body' },
    });
    press(body, 'BaseMale');
    expect(
      bodyApi.calls.some(c => c.tag === 'SetBodyShape' || c.tag === 'Unequip'),
    ).toBe(false);
  });

  it('shows the swatch row inline and opens the editor from the wheel', async () => {
    const { tree, api } = await mount({
      initial: { tab: 'face', slotByTab: { face: 'facial_hair' } },
    });
    press(tree, '#5a3410');
    expect(api.calls).toContainEqual(
      new Control.SetColor({ target: ColorTarget.Hair, color: '#5a3410' }),
    );
    press(tree, 'Custom color');
    expect(text(tree)).toContain('Facial hair color');
    expect(text(tree)).toContain('Saturation');
    press(tree, 'Back');
    expect(text(tree)).not.toContain('Facial hair color');
  });

  it('has no swatches under Mouth', async () => {
    const { tree } = await mount({
      initial: { tab: 'face', slotByTab: { face: 'mouth' } },
    });
    expect(find(tree, 'Custom color')).toBeUndefined();
  });

  it('draws the pill row, and colours the body shape, in the pill generation', async () => {
    const { tree } = await mount({
      subRow: 'pills',
      initial: { tab: 'body', slotByTab: { body: 'shape' } },
    });
    expect(find(tree, 'Custom color')).toBeDefined();
    expect(
      tree.root.findAll(n => n.props.label === 'Skin').length,
    ).toBeGreaterThan(0);
  });

  it.each(['pills', 'tiles'] as const)(
    'keeps the skin swatches when no skin item is owned (%s)',
    async subRow => {
      const { tree } = await mount({
        subRow,
        initial: { tab: 'body', slotByTab: { body: 'skin' } },
      });
      expect(text(tree)).toContain('Nothing here yet');
      expect(find(tree, 'Custom color')).toBeDefined();
    },
  );

  it('sends the head zoom when the face tab opens', async () => {
    const { tree, api } = await mount();
    pressText(tree, 'FACE');
    expect(api.calls).toContainEqual(
      new Control.FocusCamera({ focus: CameraFocus.Head }),
    );
  });

  it.each([
    ['marketplace_link', 'Marketplace'],
    ['shop', 'Find new ones in the Shop'],
    ['none', 'Nothing here yet'],
  ] as const)('draws the %s empty copy', async (copy, expected) => {
    const { tree, api } = await mount({
      initial: { tab: 'body', slotByTab: { body: 'skin' } },
      copy,
    });
    const out = text(tree);
    expect(out).toContain('Nothing here yet');
    expect(out).toContain(expected);
    expect(out.includes('Shop')).toBe(copy === 'shop');
    if (copy === 'marketplace_link') {
      act(() => tree.root.findByType(LinkButton).props.onPress());
      expect(api.marketplaceOpens).toBe(1);
    }
  });

  it('shows the loading skeleton and the connection lost state', async () => {
    const loading = await mount({
      api: { catalog: 'loading' },
      initial: { tab: 'clothing' },
    });
    expect(text(loading.tree)).not.toContain('Nothing here yet');
    const lost = await mount({ api: { catalog: 'error' } });
    expect(text(lost.tree)).toContain('Connection lost');
    await act(async () => press(lost.tree, 'Try again'));
  });

  it('applies the Collectibles filter and shows the red dot', async () => {
    const { tree } = await mount({ initial: { tab: 'clothing' } });
    press(tree, 'Filter');
    pressText(tree, 'Collectibles');
    expect(find(tree, 'green_hoodie')).toBeUndefined();
    expect(find(tree, 'xmas_sweater')).toBeDefined();
    expect(text(tree)).not.toContain('Collectibles');
    press(tree, 'Filter');
    expect(text(tree)).toContain('Collectibles');
  });

  it('keeps the last row padded when the grid is partial', async () => {
    const { tree } = await mount({
      initial: { tab: 'extras', slotByTab: { extras: 'tiara' } },
    });
    expect(find(tree, 'tiara_5')).toBeDefined();
  });
});

describe('Backpack emotes', () => {
  it('selects a slot, arms EQUIP on the first tap and equips on the second', async () => {
    const { tree, api } = await mount({ initial: { mode: 'emotes' } });
    press(tree, 'Emote slot 2');
    expect(text(tree)).toContain(MOCK_CATALOG.emotes[1].name);
    const target = DEFAULT_EMOTES[2];
    press(tree, MOCK_CATALOG.emotes[2].name);
    expect(api.calls).toContainEqual(
      new Control.PlayPreviewEmote({ urn: target.urn }),
    );
    expect(text(tree)).toContain('Equip');
    press(tree, MOCK_CATALOG.emotes[2].name);
    expect(api.calls).toContainEqual(
      new Control.SetEmoteSlot({ slot: 1, urn: target.urn }),
    );
    expect(text(tree)).toContain('Equipped');
  });

  it('unequips from an UNEQUIP tile and shows the empty collectibles state', async () => {
    const { tree, api } = await mount({
      initial: { mode: 'emotes', emoteSlot: 0 },
    });
    press(tree, MOCK_CATALOG.emotes[0].name);
    expect(api.calls).toContainEqual(
      new Control.SetEmoteSlot({ slot: 0, urn: '' }),
    );
    const none = await mount({
      initial: { mode: 'emotes', filter: 'collectibles' },
      api: { catalog: { ...MOCK_CATALOG, emotes: DEFAULT_EMOTES } },
    });
    expect(text(none.tree)).toContain('Nothing here yet');
  });
});

describe('Backpack layouts', () => {
  it('renders the landscape split without the footer', async () => {
    Dimensions.set({
      window: { width: 800, height: 360, scale: 1, fontScale: 1 },
      screen: { width: 800, height: 360, scale: 1, fontScale: 1 },
    });
    const { tree } = await mount({ footer: <></> });
    expect(text(tree)).toContain('ALL');
    const emotes = await mount({ initial: { mode: 'emotes' } });
    expect(
      emotes.tree.root.findAll(
        n => n.props.accessibilityLabel === 'Emote slot 10',
      ),
    ).not.toHaveLength(0);
  });

  it('renders the three sheet snaps', async () => {
    for (const sheet of ['peek', 'half', 'full'] as const) {
      const { tree } = await mount({ initial: { sheet } });
      expect(tree.toJSON()).not.toBeNull();
    }
  });

  it('shows the credits balance over the avatar on iOS, wearables only', async () => {
    expect(showsCredits('ios')).toBe(true);
    expect(showsCredits('android')).toBe(false);
    const { tree } = await mount({ api: { credits: 120 }, credits: true });
    expect(tree.root.findByType(CreditsBadge).props.amount).toBe(120);
    const off = await mount({ api: { credits: 120 } });
    expect(off.tree.root.findAllByType(CreditsBadge)).toHaveLength(0);
    const full = await mount({ credits: true, initial: { sheet: 'full' } });
    expect(full.tree.root.findAllByType(CreditsBadge)).toHaveLength(0);
    const emotes = await mount({ credits: true, initial: { mode: 'emotes' } });
    expect(emotes.tree.root.findAllByType(CreditsBadge)).toHaveLength(0);
  });
});

describe('Backpack data', () => {
  it('adds what the account owns after the catalog', async () => {
    const sweater = MOCK_WEARABLES.find(w => w.name === 'xmas_sweater')!;
    const { tree } = await mount({
      initial: { tab: 'clothing', filter: 'collectibles' },
      api: {
        catalog: {
          ...MOCK_CATALOG,
          wearables: MOCK_WEARABLES.filter(w => w !== sweater),
        },
        owned: { wearables: [sweater], emotes: [] },
      },
    });
    expect(find(tree, 'xmas_sweater')).toBeDefined();
  });

  it('saves the avatar once when the Backpack goes away', async () => {
    const { tree, api } = await mount();
    expect(api.saves).toBe(0);
    act(() => tree.unmount());
    expect(api.saves).toBe(1);
  });

  it('asks to try again when the save fails', async () => {
    const show = jest.fn();
    registerModalApi({
      show,
      dismiss: jest.fn(),
      askPassword: jest.fn(),
      isOpen: () => false,
    });
    const { tree, api } = await mount({ api: { saveFails: true } });
    await act(async () => tree.unmount());
    expect(show).toHaveBeenCalledWith(
      expect.objectContaining({ key: 'backpack-save-failed' }),
    );
    await act(async () => show.mock.calls[0][0].buttons[1].onPress());
    expect(api.saves).toBe(2);
    registerModalApi(null);
  });
});
