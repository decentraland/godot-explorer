import { ReactElement } from 'react';
import { act, create, ReactTestInstance } from 'react-test-renderer';
import { Pressable } from 'react-native';
import {
  BottomSheet,
  ClaimDialog,
  ClaimState,
  Dialog,
  OtpDialog,
  OtpState,
  ToastProvider,
  nearestSnap,
  snapHeights,
  toastReducer,
  useToast,
} from '@dcl/ui-native';

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

describe('ClaimDialog', () => {
  const claim = (state: ClaimState, onClaim = jest.fn(), onClose = jest.fn()) =>
    render(
      <ClaimDialog
        visible
        state={state}
        title="You’re all set"
        body="Enjoy the free wearable!"
        reward={null}
        buttonLabel="Claim reward"
        continueLabel="Tap to continue"
        onClaim={onClaim}
        onRequestClose={onClose}
      />,
    );

  it('claims from the default state', () => {
    const onClaim = jest.fn();
    const tree = claim('default', onClaim);
    expect(text(tree)).not.toContain('Tap to continue');
    act(() => pressable(tree, 'Claim reward').props.onPress());
    expect(onClaim).toHaveBeenCalled();
  });

  it('locks the button while loading and shows the spinner', () => {
    const tree = claim('loading');
    expect(text(tree)).toContain('"disabled":true');
    expect(text(tree)).toContain('progressbar');
  });

  it('closes on a tap once claimed', () => {
    const onClose = jest.fn();
    const tree = claim('claimed', jest.fn(), onClose);
    expect(text(tree)).toContain('Tap to continue');
    act(() => pressable(tree, 'Tap to continue').props.onPress());
    expect(onClose).toHaveBeenCalled();
  });
});

describe('OtpDialog', () => {
  const otp = (state: OtpState, onResend = jest.fn(), onClose = jest.fn()) =>
    render(
      <OtpDialog
        visible
        state={state}
        title="Enter verification code"
        body="One time password sent to you."
        code="12"
        onChangeCode={jest.fn()}
        codeLabel="Verification code"
        errorMessage="The code is invalid"
        verifyingLabel="Verifying..."
        resendPrompt="Didn’t get an email?"
        resendLabel="Resend Code"
        onResend={onResend}
        onRequestClose={onClose}
      />,
    );

  it('shows the error message only in the error state', () => {
    expect(text(otp('default'))).not.toContain('The code is invalid');
    expect(text(otp('error'))).toContain('The code is invalid');
  });

  it('shows the spinner row while verifying', () => {
    expect(text(otp('default'))).not.toContain('Verifying...');
    const tree = otp('verifying');
    expect(text(tree)).toContain('Verifying...');
    expect(text(tree)).toContain('progressbar');
  });

  it('resends and closes', () => {
    const onResend = jest.fn();
    const onClose = jest.fn();
    const tree = otp('default', onResend, onClose);
    act(() =>
      tree.root
        .findAll(
          n => n.props.accessibilityRole === 'link' && n.props.onPress,
        )[0]
        .props.onPress(),
    );
    expect(onResend).toHaveBeenCalled();
    act(() => pressable(tree, 'Close').props.onPress());
    expect(onClose).toHaveBeenCalled();
  });
});

describe('Dialog', () => {
  it('renders title, body and fires button callbacks', () => {
    const onLeave = jest.fn();
    const onStay = jest.fn();
    const tree = render(
      <Dialog
        visible
        icon="alert"
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
