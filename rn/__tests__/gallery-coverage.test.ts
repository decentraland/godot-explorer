/// <reference types="node" />
import { writeFileSync } from 'fs';
import { join } from 'path';
import manifest from '../../docs/rn-ui-spec/manifest.json';
import { coverage, ManifestEntry } from '../src/gallery/coverage';
import {
  previewGroup,
  previewIds,
  getPreview,
  resetRegistryForTests,
} from '../src/gallery/registry';
import '../src/gallery/screens';

const entries = manifest.screens as ManifestEntry[];

describe('coverage', () => {
  const sample: ManifestEntry[] = [
    { id: 'a', source: 's', kind: 'screen', seq: 1, name: 'A' },
    { id: 'b', source: 's', kind: 'screen', seq: 2, name: 'B' },
    { id: 'c', source: 's', kind: 'superseded', seq: 3, name: 'C' },
    { id: 'd', source: 's', kind: 'strip', seq: 4, name: 'D' },
    { id: 'e', source: 's', kind: 'screen', seq: 5, name: 'E' },
  ];

  it('lists unregistered ids, ignoring non-screen kinds and duplicate seqs', () => {
    const r = coverage(sample, ['a'], [5]);
    expect(r.missing.map(e => e.id)).toEqual(['b']);
    expect(r.required).toBe(2);
    expect(r.covered).toBe(1);
  });
});

describe('seeded gallery', () => {
  it('only registers known manifest ids or extra ids', () => {
    const known = new Set(entries.map(e => e.id));
    const unknown = previewIds().filter(
      id => !known.has(id) && !id.startsWith('extra/'),
    );
    expect(unknown).toEqual([]);
  });

  it('orientation matches the manifest for manifest ids', () => {
    const byId = new Map(entries.map(e => [e.id, e]));
    previewIds().forEach(id => {
      const e = byId.get(id);
      if (e) {
        expect([id, getPreview(id)?.orientation]).toEqual([id, e.orient]);
      }
    });
  });

  it('reports coverage; GALLERY_WRITE_IDS=1 regenerates ids.json', () => {
    const r = coverage(entries, previewIds());
    console.log(
      `gallery coverage: ${r.covered}/${r.required} manifest ids registered\n` +
        r.missing.map(e => `  missing ${e.id}`).join('\n'),
    );
    if (process.env.GALLERY_WRITE_IDS) {
      const rows = previewIds().map(id => ({
        id,
        source: previewGroup(id),
        orientation: getPreview(id)?.orientation,
      }));
      writeFileSync(
        join(__dirname, '../src/gallery/ids.json'),
        JSON.stringify(rows, null, 2) + '\n',
      );
    }
    expect(r.covered).toBeLessThanOrEqual(r.required);
  });
});

afterAll(resetRegistryForTests);
