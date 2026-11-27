import { Image } from 'react-native';
import { Rect } from 'react-native-svg';
import { act, create, ReactTestRenderer } from 'react-test-renderer';
import { CachedSvg } from '../packages/ui-native/src/internal/CachedSvg';

const mockDraws: ((base64?: string) => void)[] = [];

// Svg loads its native rasterizer on the first `toDataURL`: the test answers each draw itself.
jest.mock('react-native-svg/lib/commonjs/fabric/NativeSvgViewModule', () => ({
  __esModule: true,
  default: {
    toDataURL: (
      _tag: number,
      _options: unknown,
      callback: (base64?: string) => void,
    ) => mockDraws.push(callback),
  },
}));

function grid(count: number, cacheKey: string) {
  let tree!: ReactTestRenderer;
  act(() => {
    tree = create(
      <>
        {Array.from({ length: count }, (_, i) => (
          <CachedSvg key={i} cacheKey={cacheKey} viewBox="0 0 96 96">
            <Rect width={96} height={96} fill="red" />
          </CachedSvg>
        ))}
      </>,
    );
  });
  const boxes = tree.root.findAll(
    node =>
      typeof node.props?.onLayout === 'function' &&
      (node.type as unknown) === 'View',
  );
  act(() =>
    boxes.forEach(box =>
      box.props.onLayout({
        nativeEvent: { layout: { x: 0, y: 0, width: 120, height: 120 } },
      }),
    ),
  );
  return tree;
}

const drawn = (tree: ReactTestRenderer) =>
  tree.root.findAll(
    node =>
      node.props?.viewBox === '0 0 96 96' && !!node.props.preserveAspectRatio,
  ).length;

beforeEach(() => mockDraws.splice(0));

describe('CachedSvg', () => {
  it('draws an effect once and shows the image on every instance', () => {
    const tree = grid(5, 'once');
    expect(drawn(tree)).toBe(1);
    expect(mockDraws).toHaveLength(1);
    act(() => mockDraws[0]('iVBOR\nw0KGgo='));
    const images = tree.root.findAllByType(Image);
    expect(images).toHaveLength(5);
    expect(images[0].props.source.uri).toBe(
      'data:image/png;base64,iVBORw0KGgo=',
    );
    // The drawing instance keeps its SVG until the image has loaded, so the swap does not flash.
    expect(drawn(tree)).toBe(1);
    act(() => images.forEach(image => image.props.onLoad?.()));
    expect(drawn(tree)).toBe(0);

    // Later instances use the stored image straight away.
    const later = grid(3, 'once');
    expect(drawn(later)).toBe(0);
    expect(mockDraws).toHaveLength(1);
    expect(later.root.findAllByType(Image)).toHaveLength(3);
  });

  it('shows a drawn effect before the instance has measured itself', () => {
    grid(1, 'early');
    act(() => mockDraws[0]('iVBORw0KGgo='));
    let tree!: ReactTestRenderer;
    act(() => {
      tree = create(
        <CachedSvg cacheKey="early" viewBox="0 0 96 96" placeholder="#123456">
          <Rect width={96} height={96} fill="red" />
        </CachedSvg>,
      );
    });
    expect(tree.root.findAllByType(Image)).toHaveLength(1);
    expect(drawn(tree)).toBe(0);
  });

  it('hands the draw to a waiting instance when the drawing one goes away', () => {
    let tree!: ReactTestRenderer;
    const render = (keys: string[]) => (
      <>
        {keys.map(key => (
          <CachedSvg key={key} cacheKey="handover" viewBox="0 0 96 96">
            <Rect width={96} height={96} fill="red" />
          </CachedSvg>
        ))}
      </>
    );
    act(() => {
      tree = create(render(['first', 'second']));
    });
    tree.root
      .findAll(
        node =>
          typeof node.props?.onLayout === 'function' &&
          (node.type as unknown) === 'View',
      )
      .forEach(box =>
        act(() =>
          box.props.onLayout({
            nativeEvent: { layout: { x: 0, y: 0, width: 120, height: 120 } },
          }),
        ),
      );
    expect(mockDraws).toHaveLength(1);
    act(() => tree.update(render(['second'])));
    expect(mockDraws).toHaveLength(2);
    expect(drawn(tree)).toBe(1);
  });

  it('draws the SVG on every instance when the platform cannot rasterize', () => {
    const tree = grid(3, 'failed');
    act(() => mockDraws[0](undefined));
    expect(drawn(tree)).toBe(3);
  });
});
