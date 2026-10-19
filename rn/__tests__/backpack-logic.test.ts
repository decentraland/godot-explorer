import { DEFAULT_EMOTES } from '../src/backpack/emotes';
import { chipName, emoteStrip, tapEmote } from '../src/backpack/emoteLogic';
import { initialBackpack, reduceBackpack } from '../src/backpack/backpackState';
import { BASE_PREFIX, FEMALE, MALE, Wearable } from '../src/backpack/catalog';
import { createMockApi } from '../src/backpack/mockApi';
import { createMockAvatar } from '../src/core/mock/avatar';
import { CameraFocus, ColorTarget, Control } from '../src/core/native';
import {
  MOCK_AVATAR,
  MOCK_CATALOG,
  MOCK_WEARABLES,
} from '../src/backpack/mockCatalog';
import {
  defaultEmptyCopy,
  getEmptyCopy,
  setEmptyCopy,
} from '../src/backpack/platformPolicy';
import {
  isCustomColor,
  showsUnequipStrip,
  visibleEmotes,
  wearableCommand,
} from '../src/backpack/selection';
import {
  colorKey,
  coreColorTarget,
  cameraFocusFor,
  COLOR_TITLES,
  PALETTES,
  SLOTS,
} from '../src/backpack/taxonomy';
import { t } from '../src/i18n';

const [wave, , fist] = DEFAULT_EMOTES;
const slots = ['x:wave', '', '', '', '', '', '', '', '', ''].map(urn =>
  urn ? DEFAULT_EMOTES[1].urn : '',
);

describe('emote strips and taps', () => {
  const worn = DEFAULT_EMOTES.slice(0, 3).map(e => e.urn);

  it('shows UNEQUIP for any worn emote while no slot is selected', () => {
    expect(emoteStrip(wave.id, worn, null, null)).toBe('unequip');
    expect(emoteStrip('nope', worn, null, null)).toBeNull();
  });

  it('separates the selected slot, other slots and the pending candidate', () => {
    expect(emoteStrip(wave.id, worn, 0, null)).toBe('unequip');
    expect(emoteStrip(DEFAULT_EMOTES[1].id, worn, 0, null)).toBe('check');
    expect(
      emoteStrip(DEFAULT_EMOTES[1].id, worn, 0, DEFAULT_EMOTES[1].id),
    ).toBe('equip');
  });

  it('first tap previews and arms, second tap equips into the selected slot', () => {
    const first = tapEmote(fist, slots, 1, null);
    expect(first.commands).toEqual([
      new Control.PlayPreviewEmote({ urn: fist.urn }),
    ]);
    expect(first.pending).toBe(fist.id);
    const second = tapEmote(fist, slots, 1, fist.id);
    expect(second.commands).toEqual([
      new Control.SetEmoteSlot({ slot: 1, urn: fist.urn }),
    ]);
    expect(second.equippedSlot).toBe(1);
    expect(second.pending).toBeNull();
  });

  it('moves an emote worn elsewhere and picks the first empty slot without a selection', () => {
    const moved = tapEmote(DEFAULT_EMOTES[1], slots, 3, DEFAULT_EMOTES[1].id);
    expect(moved.commands).toEqual([
      new Control.SetEmoteSlot({ slot: 0, urn: '' }),
      new Control.SetEmoteSlot({ slot: 3, urn: DEFAULT_EMOTES[1].urn }),
    ]);
    const auto = tapEmote(fist, slots, null, fist.id);
    expect(auto.equippedSlot).toBe(1);
  });

  it('empties the slot of an UNEQUIP tile', () => {
    const tap = tapEmote(DEFAULT_EMOTES[1], slots, 0, null);
    expect(tap.commands).toEqual([
      new Control.SetEmoteSlot({ slot: 0, urn: '' }),
    ]);
    expect(tap.pending).toBeNull();
  });

  it('names the chip after the previewed emote, else the selected slot', () => {
    expect(chipName(DEFAULT_EMOTES, slots, 0, null)).toBe(
      DEFAULT_EMOTES[1].name,
    );
    expect(chipName(DEFAULT_EMOTES, slots, 2, null)).toBeNull();
    expect(chipName(DEFAULT_EMOTES, slots, 2, fist.id)).toBe(fist.name);
    expect(chipName(DEFAULT_EMOTES, slots, null, null)).toBeNull();
  });
});

describe('wearable taps', () => {
  const item = (
    category: string,
    id = `${BASE_PREFIX}${category}`,
  ): Wearable => ({
    id,
    name: category,
    category,
    thumbnail: '',
    rarity: 'base',
    bodyShapes: [],
    collectible: false,
  });

  it('equips, unequips and swaps the body shape', () => {
    expect(wearableCommand(item('hat'), false)).toEqual(
      new Control.Equip({ urn: item('hat').id }),
    );
    expect(wearableCommand(item('hat'), true)).toEqual(
      new Control.Unequip({ urn: item('hat').id }),
    );
    expect(wearableCommand(item('body_shape'), false)?.tag).toBe(
      'SetBodyShape',
    );
  });

  it('does nothing for the equipped body shape and the required parts', () => {
    expect(wearableCommand(item('body_shape'), true)).toBeNull();
    expect(wearableCommand(item('eyes'), true)).toBeNull();
  });

  it('draws the UNEQUIP strip on the body shape but not on eyes', () => {
    expect(showsUnequipStrip('body_shape')).toBe(true);
    expect(showsUnequipStrip('eyes')).toBe(false);
    expect(showsUnequipStrip('hat')).toBe(true);
  });

  it('keeps only collectible emotes under the filter', () => {
    expect(visibleEmotes(MOCK_CATALOG.emotes, 'all')).toHaveLength(
      MOCK_CATALOG.emotes.length,
    );
    expect(
      visibleEmotes(MOCK_CATALOG.emotes, 'collectibles').every(
        e => e.collectible,
      ),
    ).toBe(true);
    expect(visibleEmotes(DEFAULT_EMOTES, 'collectibles')).toEqual([]);
  });
});

describe('colour taxonomy', () => {
  it('maps the hair-like targets onto the single hair colour', () => {
    expect(colorKey('hair')).toBe('hair');
    expect(colorKey('facial_hair')).toBe('hair');
    expect(colorKey('eyebrows')).toBe('hair');
    expect(colorKey('skin')).toBe('skin');
    expect(colorKey('eyes')).toBe('eyes');
    expect(coreColorTarget('eyebrows')).toBe(ColorTarget.Hair);
  });

  it('titles every target', () => {
    expect(t(COLOR_TITLES.facial_hair)).toBe('Facial hair color');
    expect(t(COLOR_TITLES.skin)).toBe('Skin color');
  });

  it('tells a preset from a custom colour, ignoring case', () => {
    expect(isCustomColor(PALETTES.skin, '#FFE4C6')).toBe(false);
    expect(isCustomColor(PALETTES.skin, '#123456')).toBe(true);
  });

  it('zooms to the head on the face tab only', () => {
    expect(cameraFocusFor('face')).toBe(CameraFocus.Head);
    expect(cameraFocusFor('clothing')).toBe(CameraFocus.Overall);
  });

  it('lists the five face slots and four clothing slots', () => {
    expect(SLOTS.face.map(s => s.key)).toEqual([
      'hair',
      'facial_hair',
      'eyes',
      'eyebrows',
      'mouth',
    ]);
    expect(SLOTS.clothing).toHaveLength(4);
    expect(SLOTS.face.every(s => (s.key === 'mouth') === !s.color)).toBe(true);
  });
});

describe('backpack reducer', () => {
  it('tracks the emote selection, the armed candidate and the toast', () => {
    let state = reduceBackpack(initialBackpack, {
      type: 'mode',
      mode: 'emotes',
    });
    expect(state.emoteSlot).toBeNull();
    state = reduceBackpack(state, { type: 'emoteSlot', slot: 2 });
    state = reduceBackpack(state, { type: 'emotePending', id: 'wave' });
    expect(state.pendingEmote).toBe('wave');
    state = reduceBackpack(state, { type: 'emoteEquipped', slot: 2 });
    expect(state).toMatchObject({ pendingEmote: null, equippedToast: true });
    expect(reduceBackpack(state, { type: 'toastDone' }).equippedToast).toBe(
      false,
    );
  });

  it('starts at the default sheet and moves between the three snaps', () => {
    expect(initialBackpack.sheet).toBe('half');
    expect(
      reduceBackpack(initialBackpack, { type: 'sheet', sheet: 'full' }).sheet,
    ).toBe('full');
  });

  it('closes the filter menu and the colour editor on tab changes', () => {
    let state = reduceBackpack(initialBackpack, {
      type: 'openColor',
      target: 'hair',
    });
    state = reduceBackpack(state, { type: 'toggleFilterMenu' });
    state = reduceBackpack(state, { type: 'tab', tab: 'face' });
    expect(state).toMatchObject({ colorTarget: null, filterMenuOpen: false });
    expect(
      reduceBackpack(state, { type: 'closeFilterMenu' }).filterMenuOpen,
    ).toBe(false);
  });
});

describe('platform policy', () => {
  afterEach(() => setEmptyCopy(null));

  it('maps the OS to the empty-state copy and accepts an injected policy', () => {
    expect(defaultEmptyCopy('ios')).toBe('none');
    expect(defaultEmptyCopy('android')).toBe('shop');
    expect(defaultEmptyCopy('web')).toBe('marketplace_link');
    setEmptyCopy('shop');
    expect(getEmptyCopy()).toBe('shop');
  });
});

describe('mock avatar', () => {
  const hat = MOCK_WEARABLES.find(w => w.category === 'hat')!;
  const cap = MOCK_WEARABLES.find(
    w => w.category === 'hat' && w.id !== hat.id,
  )!;
  const fresh = () =>
    createMockAvatar({
      initial: MOCK_AVATAR,
      wearables: MOCK_WEARABLES,
    });
  const wornAfter = (...commands: Control[]) => {
    const avatar = fresh();
    commands.forEach(avatar.applyAvatarControl);
    return avatar.avatarState()!;
  };

  it('swaps same-category wearables and unequips', () => {
    const swapped = wornAfter(
      new Control.Equip({ urn: hat.id }),
      new Control.Equip({ urn: cap.id }),
    );
    expect(swapped.wearables).toContain(cap.id);
    expect(swapped.wearables).not.toContain(hat.id);
    const off = wornAfter(
      new Control.Equip({ urn: cap.id }),
      new Control.Unequip({ urn: cap.id }),
    );
    expect(off.wearables).not.toContain(cap.id);
  });

  it('applies body shape, colours and emote slots, and ignores the rest', () => {
    expect(wornAfter(new Control.SetBodyShape({ urn: FEMALE })).bodyShape).toBe(
      FEMALE,
    );
    expect(MOCK_AVATAR.bodyShape).toBe(MALE);
    expect(
      wornAfter(
        new Control.SetColor({ target: ColorTarget.Hair, color: '#fff' }),
      ).hair,
    ).toBe('#fff');
    expect(
      wornAfter(new Control.SetEmoteSlot({ slot: 0, urn: '' })).emotes[0],
    ).toBe('');
    expect(
      wornAfter(new Control.FocusCamera({ focus: CameraFocus.Head })),
    ).toBe(MOCK_AVATAR);
  });

  it('randomizes into a valid avatar and tells its listeners', () => {
    const avatar = fresh();
    const seen = jest.fn();
    avatar.onChange(seen);
    avatar.applyAvatarControl(new Control.RandomizeAvatar());
    const next = avatar.avatarState()!;
    expect([MALE, FEMALE]).toContain(next.bodyShape);
    expect(next.wearables.length).toBeGreaterThan(0);
    expect(seen).toHaveBeenCalledWith(next);
  });

  it('keeps the avatar object when nothing changes', () => {
    const avatar = fresh();
    const seen = jest.fn();
    avatar.onChange(seen);
    avatar.applyAvatarControl(new Control.SetName({ name: 'Ana' }));
    expect(seen).not.toHaveBeenCalled();
  });
});

describe('mock api', () => {
  it('resolves, never resolves or rejects the catalog and records calls', async () => {
    const api = createMockApi();
    await expect(api.loadCatalog()).resolves.toBe(MOCK_CATALOG);
    await expect(
      createMockApi({ catalog: 'error' }).loadCatalog(),
    ).rejects.toThrow();
    const pending = jest.fn();
    createMockApi({ catalog: 'loading' }).loadCatalog().then(pending);
    await Promise.resolve();
    expect(pending).not.toHaveBeenCalled();
    const focus = new Control.FocusCamera({ focus: CameraFocus.Head });
    api.send(focus);
    api.openMarketplace();
    expect(api.calls).toEqual([focus]);
    expect(api.marketplaceOpens).toBe(1);
  });
});
