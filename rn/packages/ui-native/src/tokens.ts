import { u } from './scale';
/** Design tokens from the "Design System Mobile" Figma file. Sizes are Figma px / 2 (dp). */
export const colors = {
  ruby: '#FF2D55',
  rubyPressed: '#D80029',
  rubyDisabled: 'rgba(255,45,85,0.5)',
  /** Figma `ruby[200]` — `Input/Counter` at its limit. */
  ruby200: '#FF5F77',
  orange: '#FF7439',
  shadow: '#161518',
  pearl: '#ECEBED',
  snow: '#FCFCFC',
  gray: '#CFCDD4',
  silver: '#A09BA8',
  textDisabled: 'rgba(160,155,168,0.5)',
  lavender: '#E8B9FF',
  magenta: '#C640CD',
  purple100: '#DCCBEC',
  /** `Purple/200`: the claimed (disabled) Claim button. */
  purple200: '#AE8AD2',
  purple350: '#691FA9',
  purple450: '#460F6F',
  purple500: '#350854',
  purple550: '#24033B',
  cardLight: '#B48CD7',
  glass: 'rgba(105,31,169,0.4)',
  glassPressed: 'rgba(105,31,169,0.6)',
  /** `Button/Discover` Disabled. */
  glass20: 'rgba(105,31,169,0.2)',
  scrim: 'rgba(0,0,0,0.4)',
  white70: 'rgba(255,255,255,0.7)',
  white50: 'rgba(252,252,252,0.5)',
  white30: 'rgba(252,252,252,0.3)',
  white20: 'rgba(255,255,255,0.2)',
  white10: 'rgba(255,255,255,0.1)',
  mythic: '#FF4BED',
  iconHud: '#DFD0FF',
  teal: '#22C1C1',
  periwinkle: '#A0ABFF',
  /** Figma `text/link` (`Button/Link`), the Periwinkle swatch. */
  link: '#A0ABFF',
  fieldDark: '#2B0847',
  panelDark: '#1F0033',
  rowSelected: '#7B3AA0',
  violetSquare: '#45146E',
  black10: 'rgba(0,0,0,0.1)',
  black20: 'rgba(0,0,0,0.2)',
  black30: 'rgba(0,0,0,0.3)',
  black40: 'rgba(0,0,0,0.4)',
  black60: 'rgba(0,0,0,0.6)',
  black70: 'rgba(0,0,0,0.7)',
  white: '#FFFFFF',
  /** Figma `color/black`. */
  black: '#000000',
  white40: 'rgba(255,255,255,0.4)',
  snow10: 'rgba(252,252,252,0.1)',
  snow70: 'rgba(252,252,252,0.7)',
  lavender70: 'rgba(232,185,255,0.7)',
  paleBlack: '#43404A',
  danger: '#FB3B3B',
  green: '#30CD00',
  modal: '#4C147C',
  menu: '#1B002D',
  /** Figma `background/brand` (the `Background` component). */
  brand: '#470071',
  /** `Card-Profile` Type=Name tile: base fill and its violet / peach glows. */
  nameCard: '#604CB0',
  nameCardViolet: '#BC93FF',
  nameCardPeach: '#F4B57B',
  navbar: 'rgba(26,9,37,0.9)',
  black25: 'rgba(0,0,0,0.25)',
  /** `Cards-Wearables` Equipped bar: Shadow at 80%. */
  shadow80: 'rgba(22,21,24,0.8)',
  black4: 'rgba(0,0,0,0.04)',
  black8: 'rgba(0,0,0,0.08)',
  black50: 'rgba(0,0,0,0.5)',
  white4: 'rgba(255,255,255,0.04)',
  white8: 'rgba(255,255,255,0.08)',
  /** Figma `alpha/deep-purple-*`: Purple 500 `#350854` at 40% / 50% (Discover card without image). */
  deepPurple40: 'rgba(53,8,84,0.4)',
  deepPurple50: 'rgba(53,8,84,0.5)',
  /** `RarityLabel` Exotic background base (drawn at 20%). */
  exoticTint: '#CAFF73',
  /** Figma `alpha/lavender-50`: Lavender Light at 50%. */
  lavender50: 'rgba(232,185,255,0.5)',
} as const;

/** The Figma `alpha/*` primitives by name: `alpha/white-30` is `alpha['white-30']`. */
export const alpha = {
  'black-4': 'rgba(0,0,0,0.04)',
  'black-8': 'rgba(0,0,0,0.08)',
  'black-10': 'rgba(0,0,0,0.1)',
  'black-20': 'rgba(0,0,0,0.2)',
  'black-30': 'rgba(0,0,0,0.3)',
  'black-40': 'rgba(0,0,0,0.4)',
  'black-50': 'rgba(0,0,0,0.5)',
  'black-60': 'rgba(0,0,0,0.6)',
  'black-70': 'rgba(0,0,0,0.7)',
  'white-4': 'rgba(255,255,255,0.04)',
  'white-8': 'rgba(255,255,255,0.08)',
  'white-10': 'rgba(255,255,255,0.1)',
  'white-20': 'rgba(255,255,255,0.2)',
  'white-30': 'rgba(255,255,255,0.3)',
  'white-40': 'rgba(255,255,255,0.4)',
  'white-70': 'rgba(255,255,255,0.7)',
  'snow-10': 'rgba(252,252,252,0.1)',
  'snow-30': 'rgba(252,252,252,0.3)',
  'snow-50': 'rgba(252,252,252,0.5)',
  'snow-70': 'rgba(252,252,252,0.7)',
  'silver-50': 'rgba(160,155,168,0.5)',
  'ruby-50': 'rgba(255,45,85,0.5)',
  'gray-30': 'rgba(207,205,212,0.3)',
  'purple-20': 'rgba(105,31,169,0.2)',
  'purple-40': 'rgba(105,31,169,0.4)',
  'purple-60': 'rgba(105,31,169,0.6)',
  'purple-70': 'rgba(105,31,169,0.7)',
  'deep-purple-40': 'rgba(53,8,84,0.4)',
  'deep-purple-50': 'rgba(53,8,84,0.5)',
  'lavender-30': 'rgba(232,185,255,0.3)',
  'lavender-50': 'rgba(232,185,255,0.5)',
  'lavender-70': 'rgba(232,185,255,0.7)',
  'violet-70': 'rgba(161,75,243,0.7)',
  'shadow-70': 'rgba(22,21,24,0.7)',
  'shadow-90': 'rgba(22,21,24,0.9)',
  'midnight-90': 'rgba(26,9,37,0.9)',
} as const;

/** The Figma "Color" collection (Design System Mobile, Colors page 1:154) by its own names: `text/link`
 * is `color.text.link`. Each points at the palette above. */
export const color = {
  background: {
    canvas: colors.shadow,
    elevated: colors.paleBlack,
    brand: colors.brand,
  },
  surface: {
    overlay: colors.black40,
    'overlay-pressed': colors.black60,
    'overlay-disabled': colors.black30,
    subtle: colors.white10,
    'subtle-strong': colors.white20,
    inverse: colors.snow,
    'inverse-pressed': colors.snow70,
    'inverse-disabled': colors.snow10,
    modal: colors.modal,
    'modal-glow': colors.mythic,
    'modal-special': '#160338',
    'modal-special-glow': '#AE59FF',
    scrim: colors.scrim,
    nametag: alpha['shadow-70'],
    navbar: colors.navbar,
    menu: colors.menu,
    selected: alpha['lavender-30'],
  },
  text: {
    primary: colors.snow,
    secondary: colors.pearl,
    placeholder: colors.white50,
    disabled: colors.textDisabled,
    inverse: colors.shadow,
    accent: colors.ruby,
    username: '#34FEE6',
    link: colors.link,
  },
  border: {
    default: colors.snow,
    disabled: colors.textDisabled,
    accent: colors.ruby,
    subtle: colors.white30,
    menu: colors.lavender70,
  },
  action: {
    primary: colors.ruby,
    'primary-pressed': colors.rubyPressed,
    'primary-disabled': colors.rubyDisabled,
    selected: colors.lavender,
  },
  status: {
    success: '#34CE77',
    online: colors.green,
    danger: colors.danger,
    live: colors.ruby,
    notification: colors.ruby,
  },
  icon: {
    default: colors.snow,
    secondary: colors.pearl,
    disabled: colors.textDisabled,
    danger: colors.danger,
  },
} as const;

/** Figma `Gradients/Light Rainbow` (Featured tag, selected avatar card); CSS 147.6° = `LinearFill` 32. */
export const lightRainbow = [
  { offset: 0, color: '#FFBC5B' },
  { offset: 0.505, color: colors.ruby },
  { offset: 1, color: colors.magenta },
];

/** `Button/Primary` `Type=Claim` image fill, read along its middle row: magenta into Ruby, which it
 * reaches at 76% of the width (`LinearFill` 90). */
export const claimGradient = [
  { offset: 0, color: '#C93EC5' },
  { offset: 0.76, color: colors.ruby },
  { offset: 1, color: colors.ruby },
];

/** Figma `Elevation/Down|Up`: one shadow, 0/±4 offset, blur 4, Shadow at 50% — values ÷ 2 for
 * dp. `boxShadow` draws the same on both platforms (Android `elevation` ignores the colour). */
const shadow = (offsetY: number, blurRadius = u(2), opacity = '80') => ({
  boxShadow: [
    { offsetX: 0, offsetY, blurRadius, color: `${colors.shadow}${opacity}` },
  ],
});
export const elevation = {
  down: shadow(u(2)),
  up: shadow(-u(2)),
  /** `Elevation/Menu` (the floating `Menu`): 0/8, blur 12, Shadow at 40%. */
  menu: shadow(u(4), u(6), '66'),
};

/** Notification card title colours by category. */
export const notificationTitle = {
  social: '#A0ABFF',
  reward: '#73FFAF',
  event: '#DF9CFF',
} as const;

export const rarityColors = {
  base: { dark: colors.cardLight, light: colors.snow },
  common: { dark: '#73D3D3', light: '#ACF8F8' },
  uncommon: { dark: '#FF8362', light: '#FFE0B2' },
  rare: { dark: '#34CE76', light: '#73FFAF' },
  epic: { dark: '#438FFF', light: '#81E1FF' },
  legendary: { dark: '#A14BF3', light: '#E8B9FF' },
  exotic: { dark: '#9CD71E', light: '#DBF0AD' },
  mythic: { dark: '#FF4BED', light: '#F6C1FF' },
  unique: { dark: '#FEA217', light: '#FFF280' },
} as const;

export type Rarity = keyof typeof rarityColors;

/** Discover `Categories Carousel` tiles (Discover file 2325:46883, 2107:25684): fill, the blurred
 * multiply shadow under the art (colour at opacity) and the inner glow. */
export const categoryTile = {
  game: {
    fill: rarityColors.epic.dark,
    shadow: rarityColors.epic.dark,
    shadowOpacity: 0.8,
    glow: 'rgba(161,191,255,0.9)',
  },
  social: {
    fill: rarityColors.legendary.dark,
    shadow: colors.purple350,
    shadowOpacity: 0.7,
    glow: 'rgba(232,185,255,0.9)',
  },
  music: {
    fill: colors.magenta,
    shadow: '#D400DF',
    shadowOpacity: 0.8,
    glow: 'rgba(246,193,255,0.8)',
  },
  art: {
    fill: '#FFC95B',
    shadow: colors.orange,
    shadowOpacity: 0.6,
    glow: 'rgba(255,242,128,0.8)',
  },
  seasonal: {
    fill: colors.purple350,
    shadow: colors.purple350,
    shadowOpacity: 0.6,
    glow: 'rgba(223,156,255,0.6)',
  },
} as const;

export type CategoryTileKind = keyof typeof categoryTile;

export const radii = {
  card: u(6),
  pill: 100,
  sheet: u(12),
  button: u(8),
} as const;

/** The Figma "Spacing & Sizes" page (values ÷ 2): the page asks for these names, never the raw
 * number. `radii` above is the older alias set (card = lg, button = xl, sheet = 2xl). */
export const space = {
  none: 0,
  '3xs': u(2),
  '2xs': u(3),
  xs: u(4),
  sm: u(6),
  md: u(8),
  lg: u(12),
  xl: u(16),
  '2xl': u(24),
  '10': u(5),
  '14': u(7),
  '18': u(9),
  '20': u(10),
  '26': u(13),
  '30': u(15),
  '40': u(20),
  '44': u(22),
  '50': u(25),
  '54': u(27),
  '56': u(28),
  '60': u(30),
  '64': u(32),
} as const;
export const radius = {
  xs: u(2),
  sm: u(4),
  md: u(5),
  lg: u(6),
  xl: u(8),
  '2xl': u(12),
  pill: 999,
} as const;
/** @deprecated Use `control` (Figma `control/*`). */
export const controlHeight = {
  lg: u(48),
  md: u(37),
  sm: u(27),
  pillLg: u(30),
  pillMd: u(28),
  tag: u(21),
  badge: u(16),
} as const;
/** @deprecated Use `icon` (Figma `icon/*` sizes). */
export const iconSize = {
  buttonLg: 16,
  buttonMd: 14,
  buttonSm: 12,
  iconButton: 20,
  pill: 16,
  tag: 13,
} as const;
/** The Figma "Size" collection: `control/pill-lg` is `control['pill-lg']`. */
export const control = {
  lg: u(48),
  md: u(37),
  sm: u(27),
  'pill-lg': u(30),
  'pill-md': u(28),
  tag: u(21),
  badge: u(16),
} as const;
export const icon = {
  'button-lg': 16,
  'button-md': 14,
  'button-sm': 12,
  'icon-button': 20,
  pill: 16,
  tag: 13,
} as const;
/** `border/thin` is 2 Figma px = 1dp (button and input outlines); `minTouchTarget` 88 px = 44dp,
 * extend smaller controls with `hitSlop`. */
export const border = { thin: 1 } as const;
export const minTouchTarget = 44;

/** Figma Motion page: durations in ms and the three cubic-bezier easings. */
export const motion = {
  duration: { instant: 100, fast: 150, base: 200, slow: 300 },
  easing: {
    standard: [0.2, 0, 0, 1],
    enter: [0, 0, 0, 1],
    exit: [0.3, 0, 1, 1],
  },
} as const;

export const fonts = {
  light: 'Inter-Light',
  regular: 'Inter-Regular',
  medium: 'Inter-Medium',
  semibold: 'Inter-SemiBold',
  bold: 'Inter-Bold',
  black: 'Inter-Black',
} as const;

/** Item grids are always three columns, 12dp apart, 24dp from the screen edge. */
export const layout = {
  gutter: u(24),
  gap: u(12),
  columns: 3,
  navHeight: u(77),
} as const;
