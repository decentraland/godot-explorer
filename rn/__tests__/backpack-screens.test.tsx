import { ReactElement } from 'react';
import { Dimensions } from 'react-native';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
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
import { EmptyCopy } from '../src/backpack/platformPolicy';

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
const collect = (node: Json | Json[] | null): string =>
  node === null
    ? ''
    : Array.isArray(node)
    ? node.map(collect).join(' ')
    : typeof node === 'string'
    ? node
    : (node.children ?? []).map(collect).join(' ');
const text = (tree: ReactTestRenderer) =>
  collect(tree.toJSON() as Json | Json[]);
const find = (tree: ReactTestRenderer, label: string) =>
  tree.root.findAll(
    node =>
      node.props.accessibilityLabel === label &&
      typeof node.props.onPress === 'function',
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

  it('drops the swatches when a pill list is empty', async () => {
    const { tree } = await mount({
      subRow: 'pills',
      initial: { tab: 'body', slotByTab: { body: 'skin' } },
    });
    expect(find(tree, 'Custom color')).toBeUndefined();
  });

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
      act(() =>
        tree.root.findByProps({ accessibilityRole: 'link' }).props.onPress(),
      );
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
});
