import { ReactNode } from 'react';
import { StyleProp, Text, TextStyle } from 'react-native';
import { colors, fonts } from './tokens';
import { u } from './scale';

/** Figma's Auto line height for Inter (ascender + descender). The Figma styles below carry it: without
 * one Android adds the font's padding to every line. The legacy variants keep the padding, because the
 * screens built on them were tuned with it. */
const AUTO_LINE_HEIGHT = 1.21;

/** The Figma "Design System Mobile" text styles by name (Fonts page, 138:1290), in Text/primary.
 * An Auto line height becomes `fontSize × AUTO_LINE_HEIGHT`. */
const figmaStyles = {
  H1: { fontFamily: fonts.bold, fontSize: u(24) },
  H2: { fontFamily: fonts.semibold, fontSize: u(21) },
  H3: { fontFamily: fonts.bold, fontSize: u(16) },
  H4: { fontFamily: fonts.semibold, fontSize: u(14) },
  H5: { fontFamily: fonts.medium, fontSize: u(15) },
  H6: { fontFamily: fonts.bold, fontSize: u(12) },
  'Btn/Large': {
    fontFamily: fonts.bold,
    fontSize: u(15),
    textTransform: 'uppercase',
  },
  'Btn/Medium': {
    fontFamily: fonts.bold,
    fontSize: u(12),
    textTransform: 'uppercase',
  },
  'Btn/Small': {
    fontFamily: fonts.bold,
    fontSize: u(11),
    textTransform: 'uppercase',
  },
  B1: { fontFamily: fonts.regular, fontSize: u(13), lineHeight: u(17) },
  B2: { fontFamily: fonts.semibold, fontSize: u(13), lineHeight: u(17) },
  B3: { fontFamily: fonts.regular, fontSize: u(10) },
  B4: { fontFamily: fonts.semibold, fontSize: u(12), lineHeight: u(15) },
  B5: { fontFamily: fonts.semibold, fontSize: u(10) },
  Caption1: { fontFamily: fonts.regular, fontSize: u(12) },
  Caption2: { fontFamily: fonts.regular, fontSize: u(11) },
  Caption3: { fontFamily: fonts.regular, fontSize: u(9) },
  Tags: {
    fontFamily: fonts.semibold,
    fontSize: u(9),
    textTransform: 'uppercase',
  },
  Input: { fontFamily: fonts.medium, fontSize: u(15) },
  'Input/Suffix': { fontFamily: fonts.light, fontSize: u(12) },
  'Input/Value': { fontFamily: fonts.regular, fontSize: u(15) },
  'Input/Code': {
    fontFamily: fonts.semibold,
    fontSize: u(24),
    lineHeight: u(24),
  },
  'Pill/Large': { fontFamily: fonts.semibold, fontSize: u(13) },
  'Pill/Medium': { fontFamily: fonts.semibold, fontSize: u(12) },
  'Modal/Title': { fontFamily: fonts.bold, fontSize: u(19) },
  'Modal/Body': {
    fontFamily: fonts.regular,
    fontSize: u(13),
    lineHeight: u(18),
  },
  'Modal/Special Title': {
    fontFamily: fonts.black,
    fontSize: u(24),
    lineHeight: u(29),
  },
  'Modal/Special Body': {
    fontFamily: fonts.regular,
    fontSize: u(16),
    lineHeight: u(20),
  },
  'Nametag/Name': { fontFamily: fonts.medium, fontSize: u(10) },
  'Tag/Count': { fontFamily: fonts.medium, fontSize: u(10) },
  'Tab/Count': { fontFamily: fonts.regular, fontSize: u(14) },
  'Tab/Label': {
    fontFamily: fonts.semibold,
    fontSize: u(14),
    textTransform: 'uppercase',
  },
  'Tag/Label': {
    fontFamily: fonts.semibold,
    fontSize: u(10),
    textTransform: 'uppercase',
  },
  'Header/Title Landscape': { fontFamily: fonts.bold, fontSize: u(22) },
  'Header/Credits': { fontFamily: fonts.medium, fontSize: u(16) },
  Display: { fontFamily: fonts.bold, fontSize: u(36) },
  'Title/Screen': { fontFamily: fonts.bold, fontSize: u(18) },
  'Title/Section': { fontFamily: fonts.bold, fontSize: u(15) },
  'Title/Card': { fontFamily: fonts.bold, fontSize: u(13) },
  'Account/Name': { fontFamily: fonts.semibold, fontSize: u(18) },
  'Body/Large': { fontFamily: fonts.regular, fontSize: u(16) },
  'Body/XLarge': { fontFamily: fonts.regular, fontSize: u(20) },
  Link: { fontFamily: fonts.medium, fontSize: u(12) },
  'Link Caps Small': {
    fontFamily: fonts.semibold,
    fontSize: u(9),
    textTransform: 'uppercase',
  },
  'Menu/Item': { fontFamily: fonts.medium, fontSize: u(14) },
  'Menu/NavItem': { fontFamily: fonts.medium, fontSize: u(15) },
  'Menu/NavItem Selected': { fontFamily: fonts.bold, fontSize: u(15) },
  'Title/Section Caps': {
    fontFamily: fonts.bold,
    fontSize: u(13),
    textTransform: 'uppercase',
  },
  'Title/Section Caps Small': {
    fontFamily: fonts.bold,
    fontSize: u(11),
    textTransform: 'uppercase',
  },
  'Account/Suffix': { fontFamily: fonts.regular, fontSize: u(14) },
  'Account/Wallet': { fontFamily: fonts.semibold, fontSize: u(12) },
  'Card/Body': {
    fontFamily: fonts.medium,
    fontSize: u(11),
    lineHeight: u(13.5),
  },
  'Badge/Title': { fontFamily: fonts.semibold, fontSize: u(16) },
  'Badge/Date': { fontFamily: fonts.medium, fontSize: u(9) },
  'Badge/Progress': { fontFamily: fonts.bold, fontSize: u(8) },
  'Badge/Progress Total': { fontFamily: fonts.regular, fontSize: u(8) },
  'Tooltip/Short': { fontFamily: fonts.medium, fontSize: u(13) },
  'Tooltip/Long': {
    fontFamily: fonts.semibold,
    fontSize: u(9),
    lineHeight: u(12),
  },
  'Alert/Body': { fontFamily: fonts.regular, fontSize: u(13) },
} satisfies Record<string, TextStyle>;

export type FigmaTextStyle = keyof typeof figmaStyles;

/** The original variant names, kept as aliases; prefer the Figma style names. */
export type LegacyTextVariant =
  | 'title'
  | 'heading'
  | 'tab'
  | 'body'
  | 'label'
  | 'caption'
  | 'button'
  | 'menuItem';

const legacy: Record<LegacyTextVariant, TextStyle> = {
  /** `H2` in Text/secondary. */
  title: { ...figmaStyles.H2, color: colors.pearl },
  heading: { fontFamily: fonts.bold, fontSize: u(28), color: colors.snow },
  /** `Tab/Label` in Gray. */
  tab: { ...figmaStyles['Tab/Label'], color: colors.gray },
  body: { fontFamily: fonts.regular, fontSize: u(15), color: colors.pearl },
  label: {
    fontFamily: fonts.medium,
    fontSize: u(12),
    color: colors.snow,
    textTransform: 'uppercase',
  },
  caption: { fontFamily: fonts.medium, fontSize: u(12), color: colors.gray },
  button: { fontFamily: fonts.semibold, fontSize: u(15), color: colors.snow },
  /** `Menu/Item`. */
  menuItem: { ...figmaStyles['Menu/Item'], color: colors.snow },
};

export type TextVariant = LegacyTextVariant | FigmaTextStyle;

const variants: Record<TextVariant, TextStyle> = {
  ...legacy,
  ...(Object.fromEntries(
    Object.entries(figmaStyles).map(([name, style]) => [
      name,
      {
        color: colors.snow,
        lineHeight: style.fontSize * AUTO_LINE_HEIGHT,
        ...style,
      },
    ]),
  ) as Record<FigmaTextStyle, TextStyle>),
};

export function AppText({
  variant = 'body',
  style,
  children,
  numberOfLines,
}: {
  variant?: TextVariant;
  style?: StyleProp<TextStyle>;
  children: ReactNode;
  numberOfLines?: number;
}) {
  return (
    <Text
      allowFontScaling={false}
      numberOfLines={numberOfLines}
      style={[variants[variant], style]}
    >
      {children}
    </Text>
  );
}
