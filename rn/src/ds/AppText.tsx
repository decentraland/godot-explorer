import { ReactNode } from 'react';
import { StyleProp, Text, TextStyle } from 'react-native';
import { colors, fonts } from './tokens';

export type TextVariant =
  | 'title'
  | 'heading'
  | 'tab'
  | 'body'
  | 'label'
  | 'caption'
  | 'button';

const variants: Record<TextVariant, TextStyle> = {
  title: { fontFamily: fonts.semibold, fontSize: 21, color: colors.pearl },
  heading: { fontFamily: fonts.bold, fontSize: 28, color: colors.snow },
  tab: {
    fontFamily: fonts.semibold,
    fontSize: 14,
    color: colors.gray,
    textTransform: 'uppercase',
  },
  body: { fontFamily: fonts.regular, fontSize: 15, color: colors.pearl },
  label: {
    fontFamily: fonts.medium,
    fontSize: 12,
    color: colors.snow,
    textTransform: 'uppercase',
  },
  caption: { fontFamily: fonts.medium, fontSize: 12, color: colors.gray },
  button: { fontFamily: fonts.semibold, fontSize: 15, color: colors.snow },
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
