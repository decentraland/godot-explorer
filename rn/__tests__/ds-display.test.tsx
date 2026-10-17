import { ReactElement } from 'react';
import { act, create, ReactTestInstance } from 'react-test-renderer';
import { Text } from 'react-native';
import {
  ActionSquareButton,
  Badge,
  Chip,
  CollapsibleSection,
  EmptyState,
  HudCircleButton,
  Icon,
  IconButton,
  LinkRow,
  PageDots,
  ProgressBar,
  Skeleton,
  SnapCarousel,
  StatusPill,
  Tabs,
  WarningBanner,
  colors,
  SCALE,
  indexFromOffset,
} from '../src/ds';
import { iconNames } from '../src/ds/Icon';
import { demos } from '../src/ds/demo';

function render(element: ReactElement) {
  let tree!: ReturnType<typeof create>;
  act(() => {
    tree = create(element);
  });
  return tree;
}

const text = (tree: ReturnType<typeof create>) => JSON.stringify(tree.toJSON());

function pressable(tree: ReturnType<typeof create>, label: string) {
  const node = tree.root
    .findAllByProps({ accessibilityLabel: label })
    .find((n: ReactTestInstance) => typeof n.props.onPress === 'function');
  if (!node) {
    throw new Error(`no pressable "${label}"`);
  }
  return node;
}

describe('tokens', () => {
  it('has the new Figma colours', () => {
    expect(colors.mythic).toBe('#FF4BED');
    expect(colors.iconHud).toBe('#DFD0FF');
    expect(colors.teal).toBe('#2FCF9A');
    expect(colors.periwinkle).toBe('#9D8CFF');
    expect(colors.fieldDark).toBe('#2B0847');
  });
});

describe('Icon', () => {
  it('renders every icon name', () => {
    expect(iconNames.length).toBeGreaterThan(60);
    for (const name of iconNames) {
      const tree = render(<Icon name={name} />);
      expect(tree.toJSON()).not.toBeNull();
      act(() => tree.unmount());
    }
  });
});

describe('Badge, IconButton, Tabs, HudCircleButton', () => {
  it('Badge shows a count, caps it, or is a dot', () => {
    expect(text(render(<Badge count={3} />))).toContain('3');
    expect(text(render(<Badge count={120} />))).toContain('99+');
    expect(text(render(<Badge />))).not.toContain('99');
  });

  it('IconButton renders a badge and extends the label', () => {
    const tree = render(
      <IconButton
        icon="bell"
        label="Notifications"
        badge={2}
        onPress={() => {}}
      />,
    );
    expect(text(tree)).toContain('2');
  });

  it('Tabs show a badge on the tab', () => {
    const tree = render(
      <Tabs
        items={[
          { key: 'a', label: 'Friends', badge: 4 },
          { key: 'b', label: 'Other' },
        ]}
        active="a"
        onChange={() => {}}
      />,
    );
    expect(text(tree)).toContain('4');
  });

  it('HudCircleButton presses and shows its badge', () => {
    const onPress = jest.fn();
    const tree = render(
      <HudCircleButton icon="chat" label="Chat" badge={1} onPress={onPress} />,
    );
    act(() => pressable(tree, 'Chat').props.onPress());
    expect(onPress).toHaveBeenCalled();
    expect(text(tree)).toContain('1');
  });
});

describe('ActionSquareButton', () => {
  it('presses with its label', () => {
    const onPress = jest.fn();
    const tree = render(
      <ActionSquareButton icon="share" label="Share" onPress={onPress} />,
    );
    act(() => pressable(tree, 'Share').props.onPress());
    expect(onPress).toHaveBeenCalled();
  });
});

describe('StatusPill and Chip', () => {
  it('renders each kind', () => {
    expect(text(render(<StatusPill kind="live" label="LIVE" />))).toContain(
      'LIVE',
    );
    expect(
      text(render(<StatusPill kind="trending" label="Trending" />)),
    ).toContain('Trending');
    expect(text(render(<StatusPill kind="count" label="12" />))).toContain(
      '12',
    );
  });

  it('Chip presses and removes', () => {
    const onPress = jest.fn();
    const onRemove = jest.fn();
    const tree = render(
      <Chip
        label="Instagram"
        icon="link"
        onPress={onPress}
        onRemove={onRemove}
      />,
    );
    act(() => pressable(tree, 'Instagram').props.onPress());
    act(() => pressable(tree, 'Remove Instagram').props.onPress());
    expect(onPress).toHaveBeenCalled();
    expect(onRemove).toHaveBeenCalled();
  });
});

describe('ProgressBar', () => {
  it('clamps the fill and exposes progress', () => {
    const tree = render(<ProgressBar value={1.5} label="Cache" />);
    const node = tree.root.findAllByProps({
      accessibilityRole: 'progressbar',
    })[0];
    expect(node.props.accessibilityValue).toMatchObject({
      min: 0,
      max: 100,
      now: 100,
    });
  });
});

describe('Skeleton and EmptyState', () => {
  it('Skeleton renders and unmounts cleanly', () => {
    const tree = render(<Skeleton width={100} height={20} />);
    expect(tree.toJSON()).not.toBeNull();
    act(() => tree.unmount());
  });

  it('EmptyState shows copy and an action', () => {
    const onPress = jest.fn();
    const tree = render(
      <EmptyState
        icon="wifiOff"
        title="Connection lost"
        body="Try again"
        actionLabel="RETRY"
        onAction={onPress}
      />,
    );
    expect(text(tree)).toContain('Connection lost');
    act(() => pressable(tree, 'RETRY').props.onPress());
    expect(onPress).toHaveBeenCalled();
  });
});

describe('CollapsibleSection', () => {
  it('hides children until open and toggles', () => {
    const onToggle = jest.fn();
    const closed = render(
      <CollapsibleSection title="Links" open={false} onToggle={onToggle}>
        <Text>inner</Text>
      </CollapsibleSection>,
    );
    expect(text(closed)).not.toContain('inner');
    act(() => pressable(closed, 'Links').props.onPress());
    expect(onToggle).toHaveBeenCalled();
    const open = render(
      <CollapsibleSection title="Links" open onToggle={onToggle}>
        <Text>inner</Text>
      </CollapsibleSection>,
    );
    expect(text(open)).toContain('inner');
  });
});

describe('LinkRow and WarningBanner', () => {
  it('LinkRow presses and marks selection', () => {
    const onPress = jest.fn();
    const tree = render(
      <LinkRow label="Privacy Policy" external selected onPress={onPress} />,
    );
    const node = pressable(tree, 'Privacy Policy');
    expect(node.props.accessibilityRole).toBe('link');
    expect(node.props.accessibilityState).toMatchObject({ selected: true });
    act(() => node.props.onPress());
    expect(onPress).toHaveBeenCalled();
  });

  it('WarningBanner shows its message', () => {
    expect(
      text(render(<WarningBanner message="Device no longer supported" />)),
    ).toContain('Device no longer supported');
  });
});

describe('PageDots and SnapCarousel', () => {
  it('PageDots marks the active dot', () => {
    const tree = render(<PageDots count={3} index={1} />);
    const dots = tree.root.findAll(
      n =>
        typeof n.type === 'string' &&
        n.props.accessibilityState?.selected === true,
    );
    expect(dots).toHaveLength(1);
  });

  it('PageDots round keeps the active dot the same size', () => {
    const tree = render(<PageDots round count={3} index={0} />);
    const active = tree.root.findAll(
      n =>
        typeof n.type === 'string' &&
        n.props.accessibilityState?.selected === true,
    )[0];
    const [base, override] = active.props.style;
    expect(override.width).toBeUndefined();
    expect(override.backgroundColor).not.toBe(base.backgroundColor);
  });

  it('indexFromOffset rounds to the nearest item and clamps', () => {
    expect(indexFromOffset(0, 100, 10, 3)).toBe(0);
    expect(indexFromOffset(115, 100, 10, 3)).toBe(1);
    expect(indexFromOffset(900, 100, 10, 3)).toBe(2);
    expect(indexFromOffset(-30, 100, 10, 3)).toBe(0);
  });

  it('SnapCarousel renders items and reports index changes', () => {
    const onIndexChange = jest.fn();
    const tree = render(
      <SnapCarousel
        data={['a', 'b', 'c']}
        itemWidth={100}
        gap={10}
        keyOf={x => x}
        renderItem={x => <Text>{`item-${x}`}</Text>}
        onIndexChange={onIndexChange}
      />,
    );
    expect(text(tree)).toContain('item-c');
    const scroller = tree.root.findAll(
      n => typeof n.props.snapToInterval === 'number',
    )[0];
    act(() =>
      scroller.props.onMomentumScrollEnd({
        nativeEvent: { contentOffset: { x: 110 * SCALE } },
      }),
    );
    expect(onIndexChange).toHaveBeenCalledWith(1);
  });
});

describe('demo props', () => {
  it('defines one entry per new component', () => {
    for (const key of [
      'Switch',
      'Slider',
      'Dropdown',
      'TextField',
      'SearchField',
      'Dialog',
      'Toast',
      'BottomSheet',
      'Skeleton',
      'EmptyState',
      'StatusPill',
      'Chip',
      'ProgressBar',
      'OutlinedButton',
      'CollapsibleSection',
      'LinkRow',
      'WarningBanner',
      'PageDots',
      'SnapCarousel',
      'HudCircleButton',
      'ReadOnlyField',
      'ActionSquareButton',
    ]) {
      expect(demos).toHaveProperty(key);
    }
  });
});
