import { ReactNode } from 'react';
import { StyleProp, Text, TextStyle } from 'react-native';
import { colors, fonts } from './tokens';
import { u } from './scale';

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
  textBreakStrategy,
}: {
  variant?: TextVariant;
  style?: StyleProp<TextStyle>;
  children: ReactNode;
  numberOfLines?: number;
  /** `simple` breaks greedily, as Figma does; Android's default balances the lines. */
  textBreakStrategy?: 'simple' | 'highQuality' | 'balanced';
}) {
  return (
    <Text
      allowFontScaling={false}
      numberOfLines={numberOfLines}
      textBreakStrategy={textBreakStrategy}
      style={[variants[variant], style]}
    >
      {children}
    </Text>
  );
}
