import { ReactElement } from 'react';
import { Text } from 'react-native';
import { act, create } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { setOrientation } from '../src/GodotBridge';
import { GalleryHost } from '../src/gallery';
import {
  getPreview,
  previewIds,
  registerAlias,
  registerPreview,
  resetRegistryForTests,
} from '../src/gallery/registry';

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

const entry = (
  title: string,
  orientation: 'portrait' | 'landscape' = 'portrait',
) => ({
  title,
  orientation,
  render: (): ReactElement => <Text>{title}</Text>,
});

describe('registry', () => {
  beforeEach(resetRegistryForTests);

  it('aliases render the target lazily, whatever the registration order', () => {
    registerAlias('a__os__2-2', 'a__x__1-1', 'portrait', 'OS sheet');
    registerPreview('a__x__1-1', entry('A'));
    const alias = getPreview('a__os__2-2');
    expect(alias?.title).toBe('A');
    expect(alias?.notes).toBe('OS sheet');
    expect(alias?.orientation).toBe('portrait');
    expect(alias?.render()).toEqual(getPreview('a__x__1-1')?.render());
  });

  it('registers and looks up entries', () => {
    registerPreview('a__x__1-1', entry('A'));
    expect(getPreview('a__x__1-1')?.title).toBe('A');
    expect(previewIds()).toEqual(['a__x__1-1']);
  });

  it('replaces a preview registered again under the same id', () => {
    registerPreview('a__x__1-1', entry('A'));
    registerPreview('a__x__1-1', entry('B'));
    expect(getPreview('a__x__1-1')?.title).toBe('B');
  });
});

describe('GalleryHost', () => {
  beforeEach(() => {
    resetRegistryForTests();
    jest.useFakeTimers();
    jest.spyOn(console, 'log').mockImplementation(() => {});
    (setOrientation as jest.Mock).mockClear();
  });
  afterEach(() => jest.useRealTimers());

  it('forces the entry orientation and logs the ready marker once laid out', () => {
    registerPreview('a__x__1-1', entry('Screen A', 'landscape'));
    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(
        <SafeAreaProvider initialMetrics={metrics}>
          <GalleryHost id="a__x__1-1" />
        </SafeAreaProvider>,
      );
    });
    expect(setOrientation).toHaveBeenCalledWith('landscape');
    expect(JSON.stringify(tree.toJSON())).toContain('Screen A');

    const root = tree.root.findByProps({ testID: 'gallery-host' });
    act(() => {
      root.props.onLayout({
        nativeEvent: { layout: { width: 900, height: 400 } },
      });
      jest.advanceTimersByTime(5000);
    });
    expect(console.log).toHaveBeenCalledWith('DCL_PREVIEW_READY a__x__1-1');
    act(() => tree.unmount());
  });

  it('shows the list for an unknown id', () => {
    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(
        <SafeAreaProvider initialMetrics={metrics}>
          <GalleryHost id="nope" />
        </SafeAreaProvider>,
      );
    });
    expect(JSON.stringify(tree.toJSON())).toContain('Unknown preview');
    act(() => tree.unmount());
  });
});
