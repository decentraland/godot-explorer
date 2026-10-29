import { view } from './storybook.requires';

/** The story alone, full-bleed, with no Storybook chrome: what the visual snapshots capture. */
export const storyRoot = (storyId: string) =>
  view.getStorybookUI({
    initialSelection: storyId as `${string}--${string}`,
    onDeviceUI: false,
    shouldPersistSelection: false,
  });
