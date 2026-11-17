import { ReactNode } from 'react';
import { StyleProp, StyleSheet, Text, TextStyle } from 'react-native';
import { colors, fonts } from './tokens';
import { u } from './scale';

/** Figma's Auto line height for Inter (ascender + descender), applied when a style sets none: Android
 * otherwise adds the font's padding to every line. */
const AUTO_LINE_HEIGHT = 1.21;

export type TextVariant =
  | 'title'
  | 'heading'
  | 'tab'
  | 'body'
  | 'label'
  | 'caption'
  | 'button'
  | 'menuItem';

const variants: Record<TextVariant, TextStyle> = {
  title: { fontFamily: fonts.semibold, fontSize: u(21), color: colors.pearl },
  heading: { fontFamily: fonts.bold, fontSize: u(28), color: colors.snow },
  tab: {
    fontFamily: fonts.semibold,
    fontSize: u(14),
    color: colors.gray,
    textTransform: 'uppercase',
  },
  body: { fontFamily: fonts.regular, fontSize: u(15), color: colors.pearl },
  label: {
    fontFamily: fonts.medium,
    fontSize: u(12),
    color: colors.snow,
    textTransform: 'uppercase',
  },
  caption: { fontFamily: fonts.medium, fontSize: u(12), color: colors.gray },
  button: { fontFamily: fonts.semibold, fontSize: u(15), color: colors.snow },
  /** Figma `Menu/Item`: Medium 28. */
  menuItem: { fontFamily: fonts.medium, fontSize: u(14), color: colors.snow },
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
  const { fontSize, lineHeight } = StyleSheet.flatten([
    variants[variant],
    style,
  ]);
  return (
    <Text
      allowFontScaling={false}
      numberOfLines={numberOfLines}
      style={[
        variants[variant],
        style,
        lineHeight === undefined &&
          fontSize !== undefined && {
            lineHeight: fontSize * AUTO_LINE_HEIGHT,
          },
      ]}
    >
      {children}
    </Text>
  );
}
