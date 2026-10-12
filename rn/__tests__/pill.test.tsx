import { act, create } from 'react-test-renderer';
import { Pill } from '../src/ds';

describe('Pill', () => {
  it('renders its label and icon', () => {
    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(
        <Pill label="Wearables" icon="wearables" selected onPress={() => {}} />,
      );
    });
    const json = JSON.stringify(tree.toJSON());
    expect(json).toContain('Wearables');
    act(() => tree.unmount());
  });
});
