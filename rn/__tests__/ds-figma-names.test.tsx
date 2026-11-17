import { act, create } from 'react-test-renderer';
import { StyleSheet, Text } from 'react-native';
import {
  AppText,
  color,
  colors,
  control,
  controlHeight,
  icon,
  iconSize,
  space,
  u,
} from '@dcl/ui-native';

function textStyle(element: React.ReactElement) {
  let tree!: ReturnType<typeof create>;
  act(() => {
    tree = create(element);
  });
  return StyleSheet.flatten(tree.root.findByType(Text).props.style);
}

describe('Figma names', () => {
  it('renders the Figma text styles by name', () => {
    expect(textStyle(<AppText variant="H4">x</AppText>)).toMatchObject({
      fontFamily: 'Inter-SemiBold',
      fontSize: u(14),
      color: colors.snow,
    });
    expect(textStyle(<AppText variant="Tags">x</AppText>)).toMatchObject({
      fontSize: u(9),
      textTransform: 'uppercase',
    });
    expect(textStyle(<AppText variant="B1">x</AppText>).lineHeight).toBe(u(17));
  });

  it('keeps the old variant names as aliases', () => {
    expect(textStyle(<AppText variant="title">x</AppText>)).toMatchObject({
      fontSize: u(21),
      color: colors.pearl,
    });
  });

  it("gives Figma's Auto line height unless the style sets one", () => {
    expect(textStyle(<AppText variant="H1">x</AppText>).lineHeight).toBeCloseTo(
      u(24) * 1.21,
    );
    expect(
      textStyle(
        <AppText variant="H1" style={{ fontSize: 10 }}>
          x
        </AppText>,
      ).lineHeight,
    ).toBeCloseTo(12.1);
    expect(
      textStyle(
        <AppText variant="H1" style={{ lineHeight: 30 }}>
          x
        </AppText>,
      ).lineHeight,
    ).toBe(30);
  });

  it('points the Figma tokens at the current values', () => {
    expect(color.text.link).toBe(colors.link);
    expect(color.surface.overlay).toBe(colors.black40);
    expect(space['20']).toBe(u(10));
    expect(control['pill-lg']).toBe(controlHeight.pillLg);
    expect(icon['button-lg']).toBe(iconSize.buttonLg);
  });
});
