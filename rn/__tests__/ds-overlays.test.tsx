import { ReactElement } from 'react';
import { act, create, ReactTestInstance } from 'react-test-renderer';
import { Pressable } from 'react-native';
import {
  BottomSheet,
  Dialog,
  ToastProvider,
  nearestSnap,
  snapHeights,
  toastReducer,
  useToast,
} from '../src/ds';

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

describe('Dialog', () => {
  it('renders title, body and fires button callbacks', () => {
    const onLeave = jest.fn();
    const onStay = jest.fn();
    const tree = render(
      <Dialog
        visible
        icon="warning"
        title="Leave without saving?"
        body="You will lose changes."
        buttons={[
          { label: 'LEAVE', tone: 'secondary', onPress: onLeave },
          { label: 'STAY', onPress: onStay },
        ]}
        onRequestClose={() => {}}
      />,
    );
    const out = text(tree);
    expect(out).toContain('Leave without saving?');
    expect(out).toContain('You will lose changes.');
    act(() => pressable(tree, 'STAY').props.onPress());
    act(() => pressable(tree, 'LEAVE').props.onPress());
    expect(onStay).toHaveBeenCalled();
    expect(onLeave).toHaveBeenCalled();
  });

  it('renders nothing visible when hidden', () => {
    const tree = render(
      <Dialog
        visible={false}
        title="Hidden"
        buttons={[]}
        onRequestClose={() => {}}
      />,
    );
    expect(text(tree) ?? '').not.toContain('Hidden');
  });

  it('renders custom children', () => {
    const tree = render(
      <Dialog visible title="Add Link" buttons={[]} onRequestClose={() => {}}>
        <Pressable accessibilityLabel="inner" />
      </Dialog>,
    );
    expect(
      tree.root.findAllByProps({ accessibilityLabel: 'inner' }).length,
    ).toBeGreaterThan(0);
  });
});

describe('toastReducer', () => {
  it('adds and removes toasts', () => {
    const a = toastReducer([], {
      type: 'show',
      toast: { id: 1, message: 'hi' },
    });
    expect(a).toHaveLength(1);
    expect(toastReducer(a, { type: 'hide', id: 1 })).toEqual([]);
    expect(toastReducer(a, { type: 'hide', id: 9 })).toBe(a);
  });

  it('keeps only the latest toast', () => {
    const a = toastReducer([], {
      type: 'show',
      toast: { id: 1, message: 'a' },
    });
    const b = toastReducer(a, { type: 'show', toast: { id: 2, message: 'b' } });
    expect(b.map(t => t.id)).toEqual([2]);
  });
});

describe('Toast host', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => jest.useRealTimers());

  function Trigger() {
    const show = useToast();
    return (
      <Pressable
        accessibilityLabel="go"
        onPress={() => show('Copied', { duration: 1000 })}
      />
    );
  }

  it('shows then dismisses a toast', () => {
    const tree = render(
      <ToastProvider>
        <Trigger />
      </ToastProvider>,
    );
    expect(text(tree)).not.toContain('Copied');
    act(() => pressable(tree, 'go').props.onPress());
    expect(text(tree)).toContain('Copied');
    act(() => {
      jest.advanceTimersByTime(1500);
    });
    expect(text(tree)).not.toContain('Copied');
    act(() => tree.unmount());
  });
});

describe('snap math', () => {
  const heights = snapHeights(1000, { peek: 100, half: 0.5, full: 1 });

  it('computes visible heights per snap', () => {
    expect(heights).toEqual({ peek: 100, half: 500, full: 1000 });
  });

  it('picks the nearest snap for a slow release', () => {
    expect(nearestSnap(120, 0, heights)).toBe('peek');
    expect(nearestSnap(320, 0, heights)).toBe('half');
    expect(nearestSnap(800, 0, heights)).toBe('full');
  });

  it('lets a fast fling carry to the next snap', () => {
    expect(nearestSnap(500, -2, heights)).toBe('full');
    expect(nearestSnap(500, 2, heights)).toBe('peek');
  });
});

describe('BottomSheet', () => {
  it('renders children and the handle toggles the snap', () => {
    const onSnapChange = jest.fn();
    const tree = render(
      <BottomSheet
        snap="half"
        containerHeight={800}
        onSnapChange={onSnapChange}
      >
        <Pressable accessibilityLabel="content" />
      </BottomSheet>,
    );
    expect(
      tree.root.findAllByProps({ accessibilityLabel: 'content' }).length,
    ).toBeGreaterThan(0);
    act(() => pressable(tree, 'Expand sheet').props.onPress());
    expect(onSnapChange).toHaveBeenCalledWith('full');
  });

  it('collapses from full to half', () => {
    const onSnapChange = jest.fn();
    const tree = render(
      <BottomSheet
        snap="full"
        containerHeight={800}
        onSnapChange={onSnapChange}
      >
        <Pressable />
      </BottomSheet>,
    );
    act(() => pressable(tree, 'Collapse sheet').props.onPress());
    expect(onSnapChange).toHaveBeenCalledWith('half');
  });
});
