import { ReactElement } from 'react';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { HomeTabs } from '../src/app/HomeTabs';
import { NavProvider } from '../src/app/NavProvider';
import { EntryFlow } from '../src/flow/EntryFlow';

jest.mock('../src/GodotBridge', () => ({
  sendCommand: jest.fn(),
  startEngine: jest.fn(),
  setOrientation: jest.fn(),
  subscribe: jest.fn(() => () => {}),
}));

const metrics = {
  frame: { x: 0, y: 0, width: 400, height: 900 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

const mounted: ReactTestRenderer[] = [];

function mount(node: ReactElement): ReactTestRenderer {
  let renderer!: ReactTestRenderer;
  act(() => {
    renderer = create(
      <SafeAreaProvider initialMetrics={metrics}>{node}</SafeAreaProvider>,
    );
  });
  mounted.push(renderer);
  return renderer;
}

describe('screens render', () => {
  afterEach(() => {
    act(() => mounted.splice(0).forEach(tree => tree.unmount()));
  });

  beforeEach(() => {
    jest.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('offline'));
  });

  it('shows the connection lost state when the catalog cannot load', async () => {
    const tree = mount(
      <NavProvider>
        <HomeTabs onJumpIn={() => {}} />
      </NavProvider>,
    );
    await act(async () => {});
    expect(JSON.stringify(tree.toJSON())).toContain('Connection lost');
  });

  it('starts the entry flow on the splash screen', () => {
    const tree = mount(<EntryFlow onDone={() => {}} />);
    expect(tree.toJSON()).not.toBeNull();
  });
});
