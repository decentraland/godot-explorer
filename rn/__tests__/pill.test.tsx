import { act, create } from 'react-test-renderer';
import { PillButton } from '@dcl/ui-native';

describe('PillButton', () => {
  it('renders its label and icon', () => {
    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(
        <PillButton
          label="Wearables"
          icon="wearables"
          selected
          onPress={() => {}}
        />,
      );
    });
    const json = JSON.stringify(tree.toJSON());
    expect(json).toContain('Wearables');
    act(() => tree.unmount());
  });
});
