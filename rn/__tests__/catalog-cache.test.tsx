import { act, create } from 'react-test-renderer';
import type { Catalog } from '../src/backpack/api';
import { createMockApi } from '../src/backpack/mockApi';
import { MOCK_CATALOG, MOCK_WEARABLES } from '../src/backpack/mockCatalog';
import {
  CatalogState,
  preloadCatalog,
  useCatalog,
} from '../src/backpack/useCatalog';

/** Mounts `useCatalog` and returns every state it rendered, first render included. */
async function open(api: ReturnType<typeof createMockApi>) {
  const seen: CatalogState[] = [];
  function Probe() {
    seen.push(useCatalog(api).catalog);
    return null;
  }
  let tree!: ReturnType<typeof create>;
  await act(async () => {
    tree = create(<Probe />);
  });
  act(() => tree.unmount());
  return seen;
}

const names = (state: CatalogState) =>
  state.status === 'ready' ? state.wearables.map(w => w.name) : [];

describe('Backpack catalog cache', () => {
  it('opens ready on a preloaded catalog, owned items included', async () => {
    const sweater = MOCK_WEARABLES.find(w => w.name === 'xmas_sweater')!;
    const base: Catalog = {
      ...MOCK_CATALOG,
      wearables: MOCK_WEARABLES.filter(w => w !== sweater),
    };
    const api = createMockApi({
      catalog: base,
      owned: { wearables: [sweater], emotes: [] },
    });
    await act(async () => preloadCatalog(api));
    const seen = await open(api);
    expect(seen[0].status).toBe('ready');
    expect(names(seen[0])).toContain('xmas_sweater');
  });

  it('never shows the base catalog without the owned items', async () => {
    const sweater = MOCK_WEARABLES.find(w => w.name === 'xmas_sweater')!;
    const api = createMockApi({
      catalog: {
        ...MOCK_CATALOG,
        wearables: MOCK_WEARABLES.filter(w => w !== sweater),
      },
    });
    // The owned list lands well after the base one, as on a real account.
    api.loadOwned = () =>
      new Promise(resolve =>
        setTimeout(() => resolve({ wearables: [sweater], emotes: [] }), 20),
      );
    const seen: CatalogState[] = [];
    function Probe() {
      seen.push(useCatalog(api).catalog);
      return null;
    }
    let tree!: ReturnType<typeof create>;
    await act(async () => {
      tree = create(<Probe />);
    });
    await act(async () => {
      await new Promise(resolve => setTimeout(resolve, 50));
    });
    act(() => tree.unmount());
    const ready = seen.filter(state => state.status === 'ready');
    expect(ready.length).toBeGreaterThan(0);
    expect(ready.every(state => names(state).includes('xmas_sweater'))).toBe(
      true,
    );
  });

  it('reopens on the last catalog instead of the skeleton', async () => {
    const api = createMockApi();
    const first = await open(api);
    expect(first[0].status).toBe('loading');
    const again = await open(api);
    expect(again.every(state => state.status === 'ready')).toBe(true);
  });

  it('keeps the shown catalog when a refresh fails', async () => {
    const api = createMockApi();
    await open(api);
    api.loadCatalog = () => Promise.reject(new Error('offline'));
    const again = await open(api);
    expect(again.map(state => state.status)).not.toContain('error');
  });
});
