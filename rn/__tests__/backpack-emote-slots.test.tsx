import { ScrollView } from 'react-native';
import { act, create } from 'react-test-renderer';
import { EmoteSlots } from '../src/backpack/EmoteSlots';
import { BASE_EMOTE_PREFIX, DEFAULT_EMOTES } from '../src/backpack/emotes';

const render = (slots: string[], vertical: boolean) => {
  let tree!: ReturnType<typeof create>;
  act(() => {
    tree = create(
      <EmoteSlots
        slots={slots}
        emotes={DEFAULT_EMOTES}
        selected={null}
        vertical={vertical}
        onSelect={() => {}}
      />,
    );
  });
  return tree;
};
const empty = () => Array.from({ length: 10 }, () => '');
const plusCount = (tree: ReturnType<typeof create>) =>
  tree.root.findAll(n => n.props.name === 'plus').length;

describe('EmoteSlots', () => {
  it('fills slots from base urns and bare ids', () => {
    const slots = empty();
    slots[7] = BASE_EMOTE_PREFIX + 'wave';
    slots[8] = 'clap';
    expect(plusCount(render(slots, true))).toBe(8);
  });

  it('shows an equipped emote missing from the catalog as filled', () => {
    const slots = empty();
    slots[3] = 'urn:decentraland:matic:collections-v2:0xabc:0';
    expect(plusCount(render(slots, true))).toBe(9);
  });

  it('lays the landscape slots out without scrolling', () => {
    const tree = render(empty(), true);
    expect(tree.root.findAllByType(ScrollView)).toHaveLength(0);
    expect(
      tree.root.findAll(
        n =>
          /^Emote slot/.test(n.props.accessibilityLabel ?? '') &&
          typeof n.type === 'string',
      ),
    ).toHaveLength(10);
  });
});
