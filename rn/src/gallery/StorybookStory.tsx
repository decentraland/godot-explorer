import { useMemo } from 'react';
import { storyRoot } from '../../.rnstorybook';
import { storyIdOf } from './stories';

/** The registered screen `id`, rendered by Storybook (see `.rnstorybook`). */
export function StorybookStory({ id }: { id: string }) {
  const Root = useMemo(() => storyRoot(storyIdOf(id)), [id]);
  return <Root />;
}
