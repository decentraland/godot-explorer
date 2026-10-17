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

  it('account screen shows the Get started copy', () => {
    const tree = mount(
      <AccountScreen
        onGuest={jest.fn()}
        onSignIn={jest.fn()}
        onBack={jest.fn()}
      />,
    );
    const json = JSON.stringify(tree.toJSON());
    expect(json).toContain('Get started!');
    expect(json).toContain('Discover a virtual social world');
    expect(json).toContain('CREATE ACCOUNT');
    expect(json).toContain('SIGN IN');
  });

  it('create account and sign in both open sign-in; guest link plays as guest', () => {
    const onSignIn = jest.fn();
    const onGuest = jest.fn();
    const tree = mount(
      <AccountScreen
        onGuest={onGuest}
        onSignIn={onSignIn}
        onBack={jest.fn()}
      />,
    );
    press(tree, 'CREATE ACCOUNT');
    press(tree, 'SIGN IN');
    expect(onSignIn).toHaveBeenCalledTimes(2);
    press(tree, 'Play as guest');
    expect(onGuest).toHaveBeenCalledTimes(1);
  });
});
