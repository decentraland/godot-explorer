/// <reference types="node" />
import fs from 'fs';
import path from 'path';
import { act, create } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import '../src/hud/gallery';
import { getPreview, previewIds } from '../src/gallery/registry';

jest.mock('../src/GodotBridge', () => ({
  sendCommand: jest.fn(),
  startEngine: jest.fn(),
  setOrientation: jest.fn(),
  subscribe: jest.fn(() => () => {}),
  godotAvailable: false,
}));

const manifest = JSON.parse(
  fs.readFileSync(
    path.join(__dirname, '..', '..', 'docs', 'rn-ui-spec', 'manifest.json'),
    'utf8',
  ),
) as { screens: { id: string; seq: number }[] };

const metrics = {
  frame: { x: 0, y: 0, width: 800, height: 360 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

describe('HUD and loading previews', () => {
  const ids = previewIds();

  it('covers manifest seq 0-12 and 273-275 with real manifest ids', () => {
    const wanted = manifest.screens
      .filter(s => s.seq <= 12 || (s.seq >= 273 && s.seq <= 275))
      .map(s => s.id);
    expect(wanted).toHaveLength(16);
    expect(wanted.filter(id => !ids.includes(id))).toEqual([]);
    expect(
      ids.filter(id => !id.startsWith('extra/') && !wanted.includes(id)),
    ).toEqual([]);
  });

  it.each(ids)('renders %s', async id => {
    let tree!: ReturnType<typeof create>;
    await act(async () => {
      tree = create(
        <SafeAreaProvider initialMetrics={metrics}>
          {getPreview(id)!.render()}
        </SafeAreaProvider>,
      );
    });
    expect(tree.toJSON()).not.toBeNull();
    act(() => tree.unmount());
  });
});
