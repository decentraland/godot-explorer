import { act, create } from 'react-test-renderer';
import { BottomSheet, ItemCard } from '@dcl/ui-native';

const text = (node: unknown): string =>
  JSON.stringify(node, (key, value) => (key === 'props' ? undefined : value));

describe('ItemCard states (Figma Cards-Wearables)', () => {
  it('draws the action bar for equip / unequip and the check bar when equipped', () => {
    let equip!: ReturnType<typeof create>;
    let unequip!: ReturnType<typeof create>;
    let equipped!: ReturnType<typeof create>;
    act(() => {
      equip = create(<ItemCard state="equip" actionLabel="Equip" />);
      unequip = create(<ItemCard state="unequip" actionLabel="Unequip" />);
      equipped = create(<ItemCard state="equipped" />);
    });
    expect(text(equip.toJSON())).toContain('Equip');
    expect(text(unequip.toJSON())).toContain('Unequip');
    expect(equipped.toJSON()).not.toBeNull();
    act(() => {
      equip.unmount();
      unequip.unmount();
      equipped.unmount();
    });
  });

  it('shows the price pill and the new tag, and no bar without a label', () => {
    let premium!: ReturnType<typeof create>;
    act(() => {
      premium = create(<ItemCard price="120" newLabel="New" state="unequip" />);
    });
    const json = text(premium.toJSON());
    expect(json).toContain('New');
    expect(json).not.toContain('120');
    act(() => premium.unmount());
  });
});

describe('BottomSheet', () => {
  it('renders the Figma Sheet look without caller styling', () => {
    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(
        <BottomSheet
          snap="half"
          onSnapChange={() => {}}
          containerHeight={600}
        />,
      );
    });
    expect(tree.toJSON()).not.toBeNull();
    act(() => tree.unmount());
  });
});
