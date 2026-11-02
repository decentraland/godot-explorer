import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { setCoreForTests } from '../src/core';
import type { CoreApi } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { ApiProvider } from '../src/discover/ApiContext';
import { ftuePlaces } from '../src/discover/ftue';
import { FtueScreen } from '../src/discover/FtueScreen';
import { createMockApi } from '../src/discover/mockApi';
import { PLACES } from '../src/discover/mockData';
import type { Place } from '../src/discover/api';
import type { DiscoverApi } from '../src/discover/types';
import { FtueStep } from '../src/flow/FtueStep';

const metrics = {
  frame: { x: 0, y: 0, width: 400, height: 900 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

const apiWith = (load: () => Promise<Place[]>): DiscoverApi => ({
  ...createMockApi(),
  ftuePlaces: load,
});

function mount(
  api: DiscoverApi,
  onJumpIn: (place: Place) => Promise<boolean>,
): ReactTestRenderer {
  let tree!: ReactTestRenderer;
  act(() => {
    tree = create(
      <SafeAreaProvider initialMetrics={metrics}>
        <ApiProvider api={api}>
          <FtueStep username="Ana" onJumpIn={onJumpIn} onFinish={jest.fn()} />
        </ApiProvider>
      </SafeAreaProvider>,
    );
  });
  return tree;
}

describe('the FTUE gate', () => {
  let trackFtueBypass: jest.Mock;
  beforeEach(() => {
    trackFtueBypass = jest.fn().mockResolvedValue(undefined);
    setCoreForTests({ ...createMockCore(), trackFtueBypass } as CoreApi);
  });
  afterEach(() => setCoreForTests(undefined));

  it('jumps straight into the only place and reports whether it went', async () => {
    const api = apiWith(async () => [PLACES[0]]);
    await ftuePlaces(api);
    const onJumpIn = jest.fn().mockResolvedValue(true);
    const tree = mount(api, onJumpIn);
    await act(async () => {});
    expect(onJumpIn).toHaveBeenCalledWith(PLACES[0]);
    expect(tree.root.findAllByType(FtueScreen)).toHaveLength(0);
    expect(trackFtueBypass).toHaveBeenCalledWith(PLACES[0].id, true);
  });

  it('shows the carousel for two or more places', async () => {
    const api = apiWith(async () => PLACES.slice(0, 2));
    await ftuePlaces(api);
    const onJumpIn = jest.fn();
    const tree = mount(api, onJumpIn);
    await act(async () => {});
    expect(tree.root.findAllByType(FtueScreen)).toHaveLength(1);
    expect(onJumpIn).not.toHaveBeenCalled();
    expect(trackFtueBypass).not.toHaveBeenCalled();
  });

  it('never waits for a list still in flight: the carousel fills when it lands', async () => {
    let land: (places: Place[]) => void = () => {};
    const api = apiWith(() => new Promise(resolve => (land = resolve)));
    ftuePlaces(api);
    const onJumpIn = jest.fn();
    const tree = mount(api, onJumpIn);
    expect(tree.root.findAllByType(FtueScreen)).toHaveLength(1);
    await act(async () => land([PLACES[0]]));
    expect(JSON.stringify(tree.toJSON())).toContain(PLACES[0].title);
    expect(onJumpIn).not.toHaveBeenCalled();
  });

  it('treats a failed fetch as an empty list', async () => {
    const api = apiWith(() => Promise.reject(new Error('offline')));
    await expect(ftuePlaces(api)).resolves.toEqual([]);
  });
});
