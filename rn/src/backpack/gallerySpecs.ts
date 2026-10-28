import type { SubRow } from './BackpackChrome';
import { BackpackState } from './backpackState';
import { DEFAULT_EMOTES } from './emotes';
import { MockApiOptions } from './mockApi';
import { MOCK_CATALOG, MOCK_SKINS, MOCK_WEARABLES } from './mockCatalog';
import { EmptyCopy } from './platformPolicy';

export type Spec = {
  api?: MockApiOptions;
  initial?: Partial<BackpackState>;
  copy?: EmptyCopy;
  subRow?: SubRow;
};

const idOf = (name: string) =>
  MOCK_WEARABLES.find(item => item.name === name)?.id ?? '';
const emoteUrn = (index: number) => DEFAULT_EMOTES[index].urn;
const slotsWith = (overrides: Record<number, string>) =>
  DEFAULT_EMOTES.slice(0, 10).map((emote, i) => overrides[i] ?? emote.urn);
const without = (...categories: string[]) => ({
  ...MOCK_CATALOG,
  wearables: MOCK_WEARABLES.filter(item => !categories.includes(item.category)),
});

const body = (slot: string): Partial<BackpackState> => ({
  tab: 'body',
  slotByTab: { body: slot },
});
const face = (slot: string): Partial<BackpackState> => ({
  tab: 'face',
  slotByTab: { face: slot },
});
const clothing = (slot = 'upper_body'): Partial<BackpackState> => ({
  tab: 'clothing',
  slotByTab: { clothing: slot },
});
const extras = (slot: string): Partial<BackpackState> => ({
  tab: 'extras',
  slotByTab: { extras: slot },
});
const emotes = (rest: Partial<BackpackState> = {}): Partial<BackpackState> => ({
  mode: 'emotes',
  ...rest,
});
const editor = (
  initial: Partial<BackpackState>,
  colorTarget: BackpackState['colorTarget'],
): Partial<BackpackState> => ({ ...initial, colorTarget });

const SIX_WORN = [
  'hoodie',
  'puffer_jacket',
  'skeleton_torso',
  'yellow_tee',
  'xmas_sweater',
  'brown_hoodie',
].map(idOf);
const PUFFER = {
  wearables: ['puffer_jacket', 'pants', 'sneakers', 'short_hair', 'beard']
    .concat(['eyes_00', 'brows_00', 'mouth_00'])
    .map(idOf),
};

const PILL_IDS = [
  'body-body-shape',
  'body-skin',
  'face-hairstyle',
  'face-facialhair',
  'face-eye',
  'face-eyebrows',
  'face-mouth',
  'clothing-chest',
  'clothing-legs',
  'clothing-feet',
  'clothing-hand',
  'accessories-',
  'backpack__menu',
  'equip01',
  'equp02',
  'skeleton__',
  'android',
  'ios__',
];
const withPills = (rows: [string, string, Spec][]) =>
  rows.map(([id, title, spec]): [string, string, Spec] =>
    PILL_IDS.some(key => id.includes(key))
      ? [id, title, { ...spec, subRow: 'pills' }]
      : [id, title, spec],
  );

const PORTRAIT_ROWS: [string, string, Spec][] = [
  [
    'backpack__body-body-shape__43-918',
    'Body: shape',
    { initial: body('shape') },
  ],
  [
    'backpack__body-body-shape__126-2652',
    'Skin colour editor',
    { initial: editor(body('skin'), 'skin') },
  ],
  [
    'backpack__body-body-shape__271-8440',
    'Skin: custom colour',
    { initial: body('skin'), api: { avatar: { skin: '#b5835a' } } },
  ],
  ['backpack__emotes__83-2357', 'Emotes: initial', { initial: emotes() }],
  [
    'backpack__body-skin-empty__126-2391',
    'Skin: empty',
    { initial: body('skin') },
  ],
  [
    'backpack__body-skin__156-4350',
    'Skin: loading',
    { initial: body('skin'), api: { catalog: 'loading' } },
  ],
  [
    'backpack__body-skin__169-4714',
    'Skin: collectible worn',
    {
      initial: body('skin'),
      api: {
        catalog: {
          ...MOCK_CATALOG,
          wearables: [...MOCK_WEARABLES, ...MOCK_SKINS],
        },
        avatar: { wearables: [...MOCK_SKINS.slice(0, 1).map(s => s.id)] },
      },
    },
  ],
  [
    'backpack__emotes__215-8041',
    'Emotes: slot 1',
    { initial: emotes({ emoteSlot: 0 }) },
  ],
  [
    'backpack__emotes__215-8436',
    'Emotes: slot 2',
    { initial: emotes({ emoteSlot: 1 }) },
  ],
  [
    'backpack__emotes__215-8842',
    'Emotes: pending equip',
    { initial: emotes({ emoteSlot: 1, pendingEmote: DEFAULT_EMOTES[2].id }) },
  ],
  [
    'backpack__emotes__912-19520',
    'Emotes: Equipped toast',
    { initial: emotes({ emoteSlot: 1, equippedToast: true }) },
  ],
  [
    'backpack__emotes__916-10419',
    'Emotes: emote playing',
    {
      initial: emotes({ emoteSlot: 1 }),
      api: {
        avatar: { emotes: slotsWith({ 1: emoteUrn(2), 2: emoteUrn(1) }) },
      },
    },
  ],
  [
    'backpack__emotes__916-10839',
    'Emotes: slot 1 (b)',
    { initial: emotes({ emoteSlot: 0 }) },
  ],
  [
    'backpack__emotes__916-11246',
    'Emotes: empty slot selected',
    {
      initial: emotes({ emoteSlot: 0 }),
      api: { avatar: { emotes: slotsWith({ 0: '' }) } },
    },
  ],
  [
    'backpack__emotes__980-14266',
    'Emotes: last slot',
    { initial: emotes({ emoteSlot: 9 }) },
  ],
  [
    'backpack__emotes__984-15749',
    'Emotes: pending equip (b)',
    { initial: emotes({ emoteSlot: 1, pendingEmote: DEFAULT_EMOTES[2].id }) },
  ],
  [
    'backpack__emotes__984-16132',
    'Emotes: slot 1 (c)',
    { initial: emotes({ emoteSlot: 0 }) },
  ],
  [
    'backpack__face-hairstyle__169-4931',
    'Face: hair',
    { initial: face('hair') },
  ],
  [
    'backpack__face-facialhair__182-2523',
    'Face: facial hair',
    { initial: face('facial_hair') },
  ],
  ['backpack__face-eye__182-2866', 'Face: eye', { initial: face('eyes') }],
  [
    'backpack__face-eyebrows__182-3242',
    'Face: eyebrows',
    { initial: face('eyebrows') },
  ],
  ['backpack__face-mouth__182-3630', 'Face: mouth', { initial: face('mouth') }],
  [
    'backpack__face-hairstyle-color__342-12908',
    'Hair colour editor',
    { initial: editor(face('hair'), 'hair') },
  ],
  [
    'backpack__face-facialhair-color__342-13098',
    'Facial hair colour editor',
    { initial: editor(face('facial_hair'), 'facial_hair') },
  ],
  [
    'backpack__face-eye-color__342-13288',
    'Eye colour editor',
    { initial: editor(face('eyes'), 'eyes') },
  ],
  [
    'backpack__face-eyebrows-color__342-13478',
    'Eyebrows colour editor',
    { initial: editor(face('eyebrows'), 'eyebrows') },
  ],
  ['backpack__emotes__226-9279', 'Emotes: all equipped', { initial: emotes() }],
  [
    'backpack__emotes__226-9697',
    'Filter menu: All',
    { initial: emotes({ filterMenuOpen: true }) },
  ],
  [
    'backpack__emotes__226-10131',
    'Filter menu: Collectibles',
    { initial: emotes({ filterMenuOpen: true, filter: 'collectibles' }) },
  ],
  [
    'backpack__emotes__226-10557',
    'Emotes: collectibles',
    { initial: emotes({ filter: 'collectibles' }) },
  ],
  [
    'backpack__emotes__1378-16445',
    'Emotes: no collectibles',
    {
      initial: emotes({ filter: 'collectibles' }),
      api: { catalog: { ...MOCK_CATALOG, emotes: DEFAULT_EMOTES } },
    },
  ],
  [
    'backpack__clothing-chest__186-4510',
    'Clothing: chest',
    { initial: clothing() },
  ],
  [
    'backpack__clothing-legs__206-5723',
    'Clothing: legs',
    { initial: clothing('lower_body') },
  ],
  [
    'backpack__clothing-feet__206-5978',
    'Clothing: feet',
    { initial: clothing('feet') },
  ],
  [
    'backpack__clothing-hand__271-8932',
    'Clothing: hand',
    { initial: clothing('hands_wear') },
  ],
  [
    'backpack__emotes__880-18121',
    'Emotes: slot 2 (b)',
    { initial: emotes({ emoteSlot: 1 }) },
  ],
  [
    'backpack__emotes__880-18182',
    'Emotes: toast (1)',
    { initial: emotes({ emoteSlot: 1, equippedToast: true }) },
  ],
  [
    'backpack__emotes__880-18244',
    'Emotes: toast (2)',
    { initial: emotes({ emoteSlot: 1, equippedToast: true }) },
  ],
  [
    'backpack__emotes__880-18306',
    'Emotes: steady',
    { initial: emotes({ emoteSlot: 1 }) },
  ],
  [
    'backpack__accessories-hat__239-8072',
    'Extras: hat',
    { initial: extras('hat') },
  ],
  [
    'backpack__accessories-hat__186-3439',
    'Extras: hat, empty',
    { initial: extras('hat'), api: { catalog: without('hat') } },
  ],
  [
    'backpack__accessories-helmet__186-3838',
    'Extras: helmets, empty',
    { initial: extras('helmet') },
  ],
  [
    'backpack__accessories-tiaras__206-6599',
    'Extras: tiaras',
    { initial: extras('tiara') },
  ],
  [
    'backpack__accessories-glasses__215-4551',
    'Extras: glasses',
    { initial: extras('eyewear') },
  ],
  [
    'backpack__accessories-mask__215-4822',
    'Extras: mask, empty',
    { initial: extras('mask') },
  ],
  [
    'backpack__accessories-earrings__215-5262',
    'Extras: earrings',
    { initial: extras('earring') },
  ],
  [
    'backpack__menu__215-6293',
    'Clothing: chest (menu)',
    { initial: clothing() },
  ],
  [
    'backpack__menu-open-all__215-6540',
    'Filter menu: All',
    { initial: { ...clothing(), filterMenuOpen: true } },
  ],
  [
    'backpack__menu-open-collectibles__215-6808',
    'Filter menu: Collectibles',
    {
      initial: { ...clothing(), filterMenuOpen: true, filter: 'collectibles' },
    },
  ],
  [
    'backpack__menu__215-7318',
    'Clothing: collectibles',
    { initial: { ...clothing(), filter: 'collectibles' } },
  ],
  ['backpack__equip01__373-9823', 'Equip: before', { initial: clothing() }],
  [
    'backpack__equp02__373-10073',
    'Equip: after',
    { initial: clothing(), api: { avatar: PUFFER } },
  ],
  [
    'backpack__skeleton__727-13070',
    'Clothing: skeleton',
    { initial: clothing(), api: { catalog: 'loading' } },
  ],
  [
    'backpack__android__855-14530',
    'Skin empty: Android',
    { initial: body('skin'), copy: 'shop' },
  ],
  [
    'backpack__ios__855-14723',
    'Skin empty: iOS',
    { initial: body('skin'), copy: 'none' },
  ],
  ['backpack__all__861-15341', 'All (older order)', {}],
  [
    'backpack__all__1665-13665',
    'All',
    { api: { avatar: { wearables: SIX_WORN } } },
  ],
  ['backpack__body__1663-13206', 'Body', { initial: body('shape') }],
  ['backpack__face__1658-15000', 'Face', { initial: face('hair') }],
  ['backpack__clothing__1462-12305', 'Clothing', { initial: clothing() }],
  ['backpack__extra__1658-15475', 'Extras', { initial: extras('hat') }],
  [
    'backpack__full__1462-13153',
    'Sheet: full',
    { initial: { ...clothing(), sheet: 'full' } },
  ],
  [
    'backpack__full-avatar__1531-19038',
    'Sheet: collapsed',
    { initial: { sheet: 'peek' } },
  ],
  [
    'backpack__body__1665-17382',
    'Body: skin empty',
    { initial: body('skin'), copy: 'shop' },
  ],
  [
    'backpack__full__1665-16724',
    'Sheet: full, skin empty',
    { initial: { ...body('skin'), sheet: 'full' }, copy: 'shop' },
  ],
];

export const PORTRAIT = withPills(PORTRAIT_ROWS);

export const LANDSCAPE: [string, string, Spec][] = [
  [
    'backpack__wearables__226-12623',
    'Emotes (landscape)',
    { initial: emotes() },
  ],
  [
    'backpack__wearables__1261-16342',
    'Connection lost (landscape)',
    { api: { catalog: 'error' } },
  ],
  [
    'backpack__all__1658-14380',
    'All (landscape)',
    { api: { avatar: { wearables: SIX_WORN } } },
  ],
  [
    'backpack__body__1665-14477',
    'Body (landscape)',
    { initial: body('shape') },
  ],
  ['backpack__face__1665-14732', 'Face (landscape)', { initial: face('hair') }],
  [
    'backpack__clothing__1665-14225',
    'Clothing (landscape)',
    { initial: clothing() },
  ],
  [
    'backpack__extra__1665-15169',
    'Extras (landscape)',
    { initial: extras('hat') },
  ],
  [
    'menu__backpack-wearable__533-5322',
    'Body (landscape, menu)',
    { initial: body('shape') },
  ],
  [
    'menu__backpack-emotes__609-3439',
    'Emotes (landscape, menu)',
    { initial: emotes() },
  ],
];
