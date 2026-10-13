export type DiscoverFlag = 'seasonalBackground' | 'storyViewer';

// Remote-config values land here; both features ship dark.
let flags: Record<DiscoverFlag, boolean> = {
  seasonalBackground: false,
  storyViewer: false,
};

export const getFlag = (name: DiscoverFlag): boolean => flags[name];

export const setFlags = (next: Partial<Record<DiscoverFlag, boolean>>) => {
  flags = { ...flags, ...next };
};
