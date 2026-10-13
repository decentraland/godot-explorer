import type { ReactElement } from 'react';
import type { Orientation } from '../GodotBridge';

export type PreviewEntry = {
  title: string;
  orientation: Orientation;
  /** `state` is the optional `?state=` of the deep link, for screens with sub-states. */
  render: (state?: string) => ReactElement;
  /** Time to wait after layout before the capture marker (async content). */
  settleMs?: number;
  notes?: string;
};

const entries = new Map<string, PreviewEntry>();

/** Re-registering an id replaces it, so a reload that re-evaluates a package does not crash.
 * Feature packages call this from their own dir (`<pkg>/gallery.tsx`) with a manifest id. */
export function registerPreview(id: string, entry: PreviewEntry): void {
  entries.set(id, entry);
}

/** A manifest frame that is the same screen as `of` (or an OS surface drawn over it). Resolved on
 * use, so `of` may be registered later; `orientation` is the frame's own. */
export function registerAlias(
  id: string,
  of: string,
  orientation: Orientation,
  notes: string,
): void {
  const target = () => {
    const entry = entries.get(of);
    if (!entry) {
      throw new Error(`Preview alias ${id}: no preview ${of}`);
    }
    return entry;
  };
  registerPreview(id, {
    get title() {
      return target().title;
    },
    orientation,
    notes,
    render: state => target().render(state),
    get settleMs() {
      return target().settleMs;
    },
  });
}

export function getPreview(id: string): PreviewEntry | undefined {
  return entries.get(id);
}

export function previewIds(): string[] {
  return [...entries.keys()];
}

/** Ids are `<manifest source>__...`; hand-made previews use `extra/...`. */
export function previewGroup(id: string): string {
  return id.includes('__') ? id.split('__')[0] : 'extra';
}

export function resetRegistryForTests(): void {
  entries.clear();
}
