import { ReactElement } from 'react';
import { ActivityIndicator } from 'react-native';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { AccountScreen, SplashScreen } from '../src/flow/screens';

const metrics = {
  frame: { x: 0, y: 0, width: 400, height: 900 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

function mount(node: ReactElement): ReactTestRenderer {
  let renderer!: ReactTestRenderer;
  act(() => {
    renderer = create(
      <SafeAreaProvider initialMetrics={metrics}>{node}</SafeAreaProvider>,
    );
  });
  return renderer;
}

const press = (tree: ReactTestRenderer, label: string) =>
  act(() =>
    tree.root.findByProps({ accessibilityLabel: label }).props.onPress(),
  );

describe('entry flow Figma screens', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => jest.useRealTimers());

  it('splash shows no spinner and still finishes after the delay', () => {
    const onDone = jest.fn();
    const tree = mount(<SplashScreen onDone={onDone} />);
    expect(tree.root.findAllByType(ActivityIndicator)).toHaveLength(0);
    act(() => jest.advanceTimersByTime(1500));
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it('account screen shows the welcome copy', () => {
    const tree = mount(<AccountScreen onGuest={jest.fn()} onSignIn={jest.fn()} />);
    const json = JSON.stringify(tree.toJSON());
    expect(json).toContain('Welcome to Decentraland');
    expect(json).toContain('Discover a world shaped');
    expect(json).toContain('PLAY AS GUEST');
    expect(json).toContain('SIGN IN');
  });

  it('play as guest and sign in trigger their own handlers', () => {
    const onSignIn = jest.fn();
    const onGuest = jest.fn();
    const tree = mount(<AccountScreen onGuest={onGuest} onSignIn={onSignIn} />);
    press(tree, 'SIGN IN');
    expect(onSignIn).toHaveBeenCalledTimes(1);
    press(tree, 'PLAY AS GUEST');
    expect(onGuest).toHaveBeenCalledTimes(1);
  });
});

describe('email sign-in screens', () => {
  it('validates the address and the code before enabling the button', () => {
    const { isValidCode, isValidEmail } = require('../src/flow/screens');
    expect(isValidEmail('ana@example.com')).toBe(true);
    expect(isValidEmail('ana@example')).toBe(false);
    expect(isValidEmail('not an email')).toBe(false);
    expect(isValidCode('123456')).toBe(true);
    expect(isValidCode('12')).toBe(false);
    expect(isValidCode('12a456')).toBe(false);
  });
});
