import { act, create } from 'react-test-renderer';
import { BottomSheet, ItemCard } from '../src/ds';

const text = (node: unknown): string =>
  JSON.stringify(node, (key, value) => (key === 'props' ? undefined : value));

describe('ItemCard strip and outline', () => {
  it('draws a strip without being equipped, with a label or an icon', () => {
    let ruby!: ReturnType<typeof create>;
    let check!: ReturnType<typeof create>;
    act(() => {
      ruby = create(<ItemCard strip={{ label: 'Equip', tone: 'ruby' }} />);
      check = create(
        <ItemCard
          strip={{ icon: 'check', tone: 'muted' }}
          outline="lavender"
        />,
      );
    });
    expect(text(ruby.toJSON())).toContain('Equip');
    expect(check.toJSON()).not.toBeNull();
    act(() => {
      ruby.unmount();
      check.unmount();
    });
  });

  it('keeps the old caption behaviour when no strip is given', () => {
    let plain!: ReturnType<typeof create>;
    act(() => {
      plain = create(<ItemCard equipped caption="Unequip" outline="orange" />);
    });
    expect(text(plain.toJSON())).toContain('Unequip');
    act(() => plain.unmount());
  });
});

describe('BottomSheet background and handle width', () => {
  it('renders the background behind the content', () => {
    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(
        <BottomSheet
          snap="half"
          onSnapChange={() => {}}
          containerHeight={600}
          handleWidth={100}
          background={<></>}
        />,
      );
    });
    expect(tree.toJSON()).not.toBeNull();
    act(() => tree.unmount());
  });
});
