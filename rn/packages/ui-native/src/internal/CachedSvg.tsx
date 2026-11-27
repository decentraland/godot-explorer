import { ReactNode, useEffect, useReducer, useRef, useState } from 'react';
import { Image, StyleSheet, View } from 'react-native';
import Svg from 'react-native-svg';

type Entry = {
  uri?: string;
  /** No raster (the platform could not draw one): every instance draws the SVG itself. */
  failed?: boolean;
  drawing: boolean;
  waiting: Set<() => void>;
};

const cache = new Map<string, Entry>();
/** The last image drawn for each effect, at any size: shown before an instance knows its own size. */
const latest = new Map<string, string>();

function entryFor(key: string): Entry {
  let entry = cache.get(key);
  if (!entry) {
    entry = { drawing: false, waiting: new Set() };
    cache.set(key, entry);
  }
  return entry;
}

function settle(entry: Entry) {
  entry.drawing = false;
  entry.waiting.forEach(wake => wake());
}

/**
 * Absolute-fill SVG effect drawn once per `cacheKey` and size: the first instance draws it and keeps
 * the PNG, and every instance shows that image. An effect of hundreds of SVG nodes then costs one
 * image view per instance, and it still comes from code. Until an image of the effect exists the
 * instance shows `placeholder`.
 */
export function CachedSvg({
  cacheKey,
  viewBox,
  placeholder,
  children,
}: {
  /** Same key, same drawing: include everything the children depend on. */
  cacheKey: string;
  viewBox: string;
  /** Background colour under the effect, so the box is never empty while the image loads. */
  placeholder?: string;
  children: ReactNode;
}) {
  const [size, setSize] = useState<{ width: number; height: number } | null>(
    null,
  );
  const [tick, wake] = useReducer((n: number) => n + 1, 0);
  const [drawing, setDrawing] = useState(false);
  // The instance that drew keeps its SVG up until the image has loaded, so the swap does not flash.
  const [handoff, setHandoff] = useState(false);
  const svg = useRef<Svg>(null);
  const key = size
    ? `${cacheKey}@${Math.round(size.width)}x${Math.round(size.height)}`
    : null;
  const entry = key ? entryFor(key) : null;

  // Become the one instance that draws, or wait for it (and take over if the drawing one goes away).
  useEffect(() => {
    if (!entry || entry.uri || entry.failed) {
      return undefined;
    }
    entry.waiting.add(wake);
    if (!entry.drawing) {
      entry.drawing = true;
      setDrawing(true);
    }
    return () => {
      entry.waiting.delete(wake);
    };
  }, [entry, tick]);

  // The drawing instance: rasterize once mounted.
  useEffect(() => {
    if (!drawing || !entry) {
      return undefined;
    }
    let done = false;
    const root = svg.current as
      | (Svg & { toDataURL?: (cb: (base64?: string) => void) => void })
      | null;
    if (typeof root?.toDataURL !== 'function') {
      entry.failed = true;
      settle(entry);
      return undefined;
    }
    root.toDataURL(base64 => {
      done = true;
      if (base64) {
        entry.uri = `data:image/png;base64,${base64.replace(/\s/g, '')}`;
        latest.set(cacheKey, entry.uri);
      } else {
        entry.failed = true;
      }
      settle(entry);
      setHandoff(true);
      setDrawing(false);
    });
    return () => {
      if (!done) {
        settle(entry);
      }
    };
  }, [drawing, entry, cacheKey]);

  const uri = entry?.uri ?? latest.get(cacheKey);
  const svgUp = !!size && (drawing || handoff || !!entry?.failed);

  return (
    <View
      style={[StyleSheet.absoluteFill, { backgroundColor: placeholder }]}
      pointerEvents="none"
      onLayout={event => {
        const { width, height } = event.nativeEvent.layout;
        if (width > 0 && height > 0) {
          setSize({ width, height });
        }
      }}
    >
      {svgUp && size ? (
        <Svg
          ref={svg}
          width={size.width}
          height={size.height}
          viewBox={viewBox}
          preserveAspectRatio="none"
        >
          {children}
        </Svg>
      ) : null}
      {uri ? (
        <Image
          source={{ uri }}
          style={StyleSheet.absoluteFill}
          resizeMode="stretch"
          fadeDuration={0}
          onLoad={() => setHandoff(false)}
        />
      ) : null}
    </View>
  );
}
