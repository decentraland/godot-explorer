import { u } from './scale';
/** Design tokens from the "Design System Mobile" Figma file. Sizes are Figma px / 2 (dp). */
export const colors = {
  ruby: '#FF2D55',
  orange: '#FF7439',
  shadow: '#161518',
  pearl: '#ECEBED',
  snow: '#FCFCFC',
  gray: '#CFCDD4',
  silver: '#A09BA8',
  lavender: '#E8B9FF',
  purple350: '#691FA9',
  purple450: '#460F6F',
  purple500: '#350854',
  purple550: '#24033B',
  cardLight: '#B48CD7',
  glass: 'rgba(105,31,169,0.4)',
  scrim: 'rgba(0,0,0,0.4)',
  white70: 'rgba(255,255,255,0.7)',
  white30: 'rgba(252,252,252,0.3)',
  white10: 'rgba(255,255,255,0.1)',
} as const;

export const rarityColors = {
  base: { dark: colors.cardLight, light: colors.snow },
  common: { dark: '#73D3D3', light: '#ACF8F8' },
  uncommon: { dark: '#FF8362', light: '#FFD6B2' },
  rare: { dark: '#34CE76', light: '#73FFAF' },
  epic: { dark: '#438FFF', light: '#81D1FF' },
  legendary: { dark: '#A14BF3', light: '#DF9CFF' },
  exotic: { dark: '#CAFF73', light: '#E4FFB8' },
  mythic: { dark: '#FF4BED', light: '#F6C1FF' },
  unique: { dark: '#FEA217', light: '#FFF280' },
} as const;

export type Rarity = keyof typeof rarityColors;

export const radii = {
  card: u(6),
  pill: 100,
  sheet: u(12),
  button: u(8),
} as const;

export const fonts = {
  regular: 'Inter-Regular',
  medium: 'Inter-Medium',
  semibold: 'Inter-SemiBold',
  bold: 'Inter-Bold',
} as const;

/** Item grids are always three columns, 12dp apart, 24dp from the screen edge. */
export const layout = {
  gutter: u(24),
  gap: u(12),
  columns: 3,
  navHeight: u(77),
} as const;
