export type Snap = 'peek' | 'half' | 'full';
export type SnapHeights = Record<Snap, number>;

/** `peek` is dp; `half` and `full` are fractions of the container height. */
export type SnapConfig = { peek: number; half: number; full: number };

export const DEFAULT_SNAP_CONFIG: SnapConfig = { peek: 72, half: 0.6, full: 1 };

const ORDER: Snap[] = ['peek', 'half', 'full'];

const FLING_PROJECTION_MS = 200;

export function snapHeights(
  containerHeight: number,
  config: SnapConfig = DEFAULT_SNAP_CONFIG,
): SnapHeights {
  return {
    peek: config.peek,
    half: Math.round(containerHeight * config.half),
    full: Math.round(containerHeight * config.full),
  };
}

/**
 * Picks the snap closest to where the sheet would land. `velocityY` is dp/ms
 * with the pointer convention: negative = moving up (growing the sheet).
 */
export function nearestSnap(
  visibleHeight: number,
  velocityY: number,
  heights: SnapHeights,
): Snap {
  const projected = visibleHeight - velocityY * FLING_PROJECTION_MS;
  return ORDER.reduce((best, snap) =>
    Math.abs(heights[snap] - projected) < Math.abs(heights[best] - projected)
      ? snap
      : best,
  );
}
