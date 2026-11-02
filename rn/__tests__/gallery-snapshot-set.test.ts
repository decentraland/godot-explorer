import snapshotSet from '../src/gallery/snapshot-set.json';
import { previewIds } from '../src/gallery/registry';
import '../src/gallery/screens';

describe('snapshot set', () => {
  it('lists registered screens once each', () => {
    const registered = new Set(previewIds());
    expect(snapshotSet.filter(id => !registered.has(id))).toEqual([]);
    expect(new Set(snapshotSet).size).toBe(snapshotSet.length);
  });
});
