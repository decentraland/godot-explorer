import { StyleSheet } from 'react-native';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { colors, PillButton } from '@dcl/ui-native';

function render(element: React.ReactElement) {
  let tree!: ReactTestRenderer;
  act(() => {
    tree = create(element);
  });
  return tree;
}

function background(tree: ReactTestRenderer) {
  const pill = tree.root.findAll(
    n =>
      typeof n.type === 'string' &&
      n.props.accessibilityState?.selected !== undefined,
  )[0];
  return StyleSheet.flatten(pill.props.style).backgroundColor;
}

describe('PillButton', () => {
  it('renders its label and icon', () => {
    const tree = render(
      <PillButton
        label="Wearables"
        icon="wearables"
        selected
        onPress={() => {}}
      />,
    );
    expect(JSON.stringify(tree.toJSON())).toContain('Wearables');
    act(() => tree.unmount());
  });

  it.each([
    ['Lavender', false, colors.white20],
    ['Lavender', true, colors.lavender],
    ['Ruby', false, colors.black40],
    ['Ruby', true, colors.ruby],
  ] as const)(
    'Color=%s, selected=%s is filled as in Figma',
    (color, selected, fill) => {
      const tree = render(
        <PillButton
          big
          color={color}
          label="Pill"
          selected={selected}
          onPress={() => {}}
        />,
      );
      expect(background(tree)).toBe(fill);
      act(() => tree.unmount());
    },
  );
});
