import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { ApiProvider } from '../src/discover/ApiContext';
import { ftuePlaces } from '../src/discover/ftue';
import { FtueScreen } from '../src/discover/FtueScreen';
import { createMockApi } from '../src/discover/mockApi';
import { PLACES } from '../src/discover/mockData';
import type { Place } from '../src/discover/api';
import type { DiscoverApi } from '../src/discover/types';

const metrics = {
  frame: { x: 0, y: 0, width: 400, height: 900 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

const apiWith = (load: () => Promise<Place[]>): DiscoverApi => ({
  ...createMockApi(),
  ftuePlaces: load,
});

function mount(api: DiscoverApi, onJump: (place: Place) => void) {
  let tree!: ReactTestRenderer;
  act(() => {
    tree = create(
      <SafeAreaProvider initialMetrics={metrics}>
        <ApiProvider api={api}>
          <FtueScreen username="Ana" onJump={onJump} onSkip={jest.fn()} />
        </ApiProvider>
      </SafeAreaProvider>,
    );
  });
  return tree;
}

describe('the FTUE screen', () => {
  it('never waits for a list still in flight: the carousel fills when it lands', async () => {
    let land: (places: Place[]) => void = () => {};
    const api = apiWith(() => new Promise(resolve => (land = resolve)));
    ftuePlaces(api);
    const tree = mount(api, jest.fn());
    expect(JSON.stringify(tree.toJSON())).not.toContain('JUMP IN');
    await act(async () => land(PLACES.slice(0, 2)));
    const json = JSON.stringify(tree.toJSON());
    expect(json).toContain(PLACES[0].title);
    expect(json).toContain('JUMP IN');
  });

  it('jumps into the settled place', async () => {
    const api = apiWith(async () => PLACES.slice(0, 2));
    await ftuePlaces(api);
    const onJump = jest.fn();
    const tree = mount(api, onJump);
    await act(async () => {});
    act(() =>
      tree.root.findByProps({ accessibilityLabel: 'JUMP IN' }).props.onPress(),
    );
    expect(onJump).toHaveBeenCalledWith(PLACES[0]);
  });

  it('treats a failed fetch as an empty list', async () => {
    const api = apiWith(() => Promise.reject(new Error('offline')));
    await expect(ftuePlaces(api)).resolves.toEqual([]);
  });
});
