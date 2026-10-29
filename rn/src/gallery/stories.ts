import { toId } from 'storybook/internal/csf';
import type { Orientation } from '../platform/orientation';
import { getPreview, previewGroup, previewIds } from './registry';

export type StoryPreview = {
  id: string;
  orientation: Orientation;
  settleMs?: number;
};

/** `<manifest id>` -> the Storybook story id, `Screens/<group>` being the title of its file. */
export const storyIdOf = (id: string): string =>
  toId(`Screens/${previewGroup(id)}`, id);

/** One story per registered screen of a group, named by its manifest id (the export key). */
export function screenStories(group: string): Record<string, unknown> {
  const stories: Record<string, unknown> = {
    default: { title: `Screens/${group}` },
  };
  previewIds()
    .filter(id => previewGroup(id) === group)
    .forEach(id => {
      const entry = getPreview(id)!;
      const preview: StoryPreview = {
        id,
        orientation: entry.orientation,
        settleMs: entry.settleMs,
      };
      stories[id] = {
        name: entry.title,
        render: () => entry.render(),
        parameters: { preview },
      };
    });
  return stories;
}
