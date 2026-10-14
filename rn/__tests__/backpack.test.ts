import type { AvatarState } from '../src/core/native';
import {
  initialBackpack,
  reduceBackpack,
  activeSlot,
} from '../src/backpack/backpackState';
import {
  parseWearables,
  MALE,
  FEMALE,
  BASE_PREFIX,
} from '../src/backpack/catalog';
import {
  DEFAULT_EMOTES,
  emoteIdOf,
  withThumbnails,
} from '../src/backpack/emotes';
import {
  equippedItem,
  equippedThumbnail,
  gridItems,
  isEquipped,
  targetEmoteSlot,
  visibleWearables,
} from '../src/backpack/selection';
import { SLOTS, isRemovable } from '../src/backpack/taxonomy';

const raw = (
  id: string,
  category: string,
  shapes: string[] = [MALE, FEMALE],
) => ({
  id: BASE_PREFIX + id,
  name: id,
  thumbnail: `https://t/${id}`,
  i18n: [{ code: 'en', text: id.toUpperCase() }],
  data: {
    category,
    representations: shapes.map(bodyShapes => ({ bodyShapes: [bodyShapes] })),
  },
});

const catalog = parseWearables({
  wearables: [
    raw('BaseMale', 'body_shape', [MALE]),
    raw('BaseFemale', 'body_shape', [FEMALE]),
    raw('hoodie', 'upper_body', [MALE]),
    raw('dress', 'upper_body', [FEMALE]),
    raw('cap', 'hat'),
    {
      id: 'urn:decentraland:matic:collections-v2:0x1:0',
      thumbnail: 't',
      rarity: 'epic',
      data: { category: 'hat' },
    },
    { id: 'broken' },
  ],
});

const avatar = (patch: Partial<AvatarState> = {}): AvatarState => ({
  bodyShape: MALE,
  wearables: [BASE_PREFIX + 'hoodie'],
  emotes: ['wave', '', '', '', '', '', '', '', '', ''],
  skin: '#ffdcb6',
  hair: '#1c1c1c',
  eyes: '#2b2b2b',
  ...patch,
});

describe('parseWearables', () => {
  it('drops malformed entries and maps names, rarity and collectible flag', () => {
    expect(catalog).toHaveLength(6);
    expect(catalog.find(item => item.id.endsWith('hoodie'))?.name).toBe(
      'HOODIE',
    );
    const epic = catalog.find(item => item.rarity === 'epic');
    expect(epic?.collectible).toBe(true);
    expect(catalog.find(item => item.id.endsWith('cap'))?.collectible).toBe(
      false,
    );
  });

  it('tolerates garbage payloads', () => {
    expect(parseWearables(null)).toEqual([]);
    expect(parseWearables({ wearables: 'nope' })).toEqual([]);
  });
});

describe('selection', () => {
  it('hides wearables the current body shape cannot wear', () => {
    const ids = visibleWearables(catalog, avatar(), 'all').map(item =>
      item.id.replace(BASE_PREFIX, ''),
    );
    expect(ids).toContain('hoodie');
    expect(ids).not.toContain('dress');
    expect(ids).toContain('BaseFemale');
  });

  it('filters collectibles', () => {
    const items = visibleWearables(catalog, avatar(), 'collectibles');
    expect(items).toHaveLength(1);
    expect(items[0].rarity).toBe('epic');
  });

  it('lists equipped first under All and by category under a slot', () => {
    const all = gridItems('all', null, catalog, avatar(), 'all');
    expect(all.slice(0, 2).map(item => isEquipped(avatar(), item.id))).toEqual([
      true,
      true,
    ]);
    const chest = gridItems(
      'clothing',
      SLOTS.clothing[0],
      catalog,
      avatar(),
      'all',
    );
    expect(chest.map(item => item.category)).toEqual(['upper_body']);
    expect(gridItems('body', SLOTS.body[1], catalog, avatar(), 'all')).toEqual(
      [],
    );
  });

  it('finds the thumbnail worn in a slot', () => {
    expect(equippedThumbnail(SLOTS.clothing[0], catalog, avatar())).toBe(
      'https://t/hoodie',
    );
    expect(
      equippedThumbnail(SLOTS.extras[0], catalog, avatar()),
    ).toBeUndefined();
  });

  it('finds the item worn in a slot', () => {
    expect(equippedItem(SLOTS.clothing[0], catalog, avatar())?.name).toBe(
      catalog.find(item => item.category === 'upper_body')?.name,
    );
    expect(equippedItem(SLOTS.extras[0], catalog, avatar())).toBeUndefined();
  });

  it('picks the emote slot to fill', () => {
    const emotes = ['wave', 'clap', '', 'x'];
    expect(targetEmoteSlot(emotes, 2)).toBe(2);
    expect(targetEmoteSlot(emotes, 0)).toBe(2);
    expect(targetEmoteSlot(['a', 'b'], 1)).toBe(1);
  });

  it('knows which categories can be unequipped', () => {
    expect(isRemovable('hat')).toBe(true);
    expect(isRemovable('eyes')).toBe(false);
  });
});

describe('reduceBackpack', () => {
  it('remembers the slot per tab and drops the color editor on navigation', () => {
    let state = reduceBackpack(initialBackpack, { type: 'tab', tab: 'body' });
    expect(activeSlot(state)?.key).toBe('shape');
    state = reduceBackpack(state, { type: 'slot', slot: SLOTS.body[1] });
    expect(state.colorTarget).toBeNull();
    state = reduceBackpack(state, { type: 'openColor', target: 'skin' });
    expect(state.colorTarget).toBe('skin');
    state = reduceBackpack(state, { type: 'tab', tab: 'face' });
    expect(state.colorTarget).toBeNull();
    state = reduceBackpack(state, { type: 'tab', tab: 'body' });
    expect(activeSlot(state)?.key).toBe('skin');
  });

  it('closes menus on mode change and does not mutate', () => {
    const open = reduceBackpack(initialBackpack, { type: 'toggleFilterMenu' });
    expect(open.filterMenuOpen).toBe(true);
    expect(initialBackpack.filterMenuOpen).toBe(false);
    expect(
      reduceBackpack(open, { type: 'mode', mode: 'emotes' }).filterMenuOpen,
    ).toBe(false);
  });
});

describe('emotes', () => {
  it('normalises ids and attaches thumbnails by pointer', () => {
    expect(emoteIdOf('urn:decentraland:off-chain:base-emotes:wave')).toBe(
      'wave',
    );
    expect(emoteIdOf('wave')).toBe('wave');
    const out = withThumbnails(DEFAULT_EMOTES, [
      {
        pointers: ['urn:decentraland:off-chain:base-emotes:wave'],
        metadata: { thumbnail: 'thumbnail.png' },
        content: [{ file: 'thumbnail.png', hash: 'abc' }],
      },
    ]);
    expect(out.find(emote => emote.id === 'wave')?.thumbnail).toMatch(
      /contents\/abc$/,
    );
    expect(out.find(emote => emote.id === 'dance')?.thumbnail).toBeUndefined();
  });
});
