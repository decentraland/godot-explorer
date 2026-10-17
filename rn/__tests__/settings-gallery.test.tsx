/// <reference types="node" />
import fs from 'fs';
import path from 'path';
import { Dimensions } from 'react-native';
import { act, create } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { NavProvider, useNav, Nav } from '../src/app/NavProvider';
import { getScreen } from '../src/app/registry';
import { getPreview, previewIds } from '../src/gallery/registry';
import { LinkRow, OutlinedButton } from '../src/ds';
import '../src/settings';
import '../src/settings/gallery';

const MANIFEST = path.join(
  __dirname,
  '..',
  '..',
  'docs',
  'rn-ui-spec',
  'manifest.json',
);
const metrics = {
  frame: { x: 0, y: 0, width: 360, height: 640 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

const ids = previewIds().filter(id => id.startsWith('settings-'));

describe('settings previews', () => {
  it('registers only ids that exist in the manifest, covering revamp 327-342 minus the duplicate', () => {
    const manifest = JSON.parse(fs.readFileSync(MANIFEST, 'utf8')).screens as {
      id: string;
      seq: number;
    }[];
    const known = new Set(manifest.map(e => e.id));
    ids.forEach(id => expect(known.has(id)).toBe(true));
    const revamp = manifest.filter(
      e => e.seq >= 327 && e.seq <= 342 && e.seq !== 331,
    );
    revamp.forEach(e => expect(ids).toContain(e.id));
  });

  it('forces landscape for the three landscape frames', () => {
    const land = ids.filter(id => getPreview(id)?.orientation === 'landscape');
    expect(land).toHaveLength(5);
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
    expect(JSON.stringify(tree.toJSON()).length).toBeGreaterThan(500);
    act(() => tree.unmount());
  });
});

describe('settings tab registration', () => {
  it('plugs the settings screen into the nav registry and closes to Explore', async () => {
    const Tab = getScreen('settings')!;
    expect(Tab).toBeDefined();
    let nav!: Nav;
    const Probe = () => {
      nav = useNav();
      return null;
    };
    let tree!: ReturnType<typeof create>;
    await act(async () => {
      tree = create(
        <SafeAreaProvider initialMetrics={metrics}>
          <NavProvider>
            <Probe />
            <Tab params={undefined as never} footer={null} />
          </NavProvider>
        </SafeAreaProvider>,
      );
    });
    act(() => nav.setTab('settings'));
    await act(async () =>
      Dimensions.set({
        window: { width: 900, height: 400, scale: 1, fontScale: 1 },
      }),
    );
    const back = tree.root.findAll(
      n => n.props.accessibilityLabel === 'Back' && !!n.props.onPress,
    )[0];
    await act(async () => back.props.onPress());
    expect(nav.state.tab).toBe('explore');
    act(() => tree.unmount());
    Dimensions.set({
      window: { width: 360, height: 640, scale: 1, fontScale: 1 },
    });
  });
});

describe('ds additive props', () => {
  it('LinkRow takes a height and an inline icon', () => {
    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(
        <>
          <LinkRow label="A" height={30} onPress={() => {}} />
          <LinkRow label="B" inline onPress={() => {}} />
          <LinkRow label="C" inline external onPress={() => {}} />
        </>,
      );
    });
    const rows = tree.root.findAllByType(LinkRow);
    const svgs = rows.map(
      r => r.findAll(n => (n.type as unknown) === 'RNSVGSvgView').length,
    );
    expect(svgs).toEqual([1, 0, 1]);
    act(() => tree.unmount());
  });

  it('OutlinedButton takes an icon and a box shape', () => {
    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(
        <OutlinedButton
          box
          icon="logout"
          label="SIGN OUT"
          onPress={() => {}}
        />,
      );
    });
    expect(
      tree.root.findAll(n => (n.type as unknown) === 'RNSVGSvgView').length,
    ).toBeGreaterThan(0);
    act(() => tree.unmount());
  });
});
