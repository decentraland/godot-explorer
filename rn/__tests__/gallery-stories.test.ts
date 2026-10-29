import { previewGroup, previewIds } from '../src/gallery/registry';
import { screenStories, storyIdOf } from '../src/gallery/stories';
import '../src/gallery/screens';

describe('gallery stories', () => {
  it('gives every registered screen a story with a unique id', () => {
    const groups = new Set(previewIds().map(previewGroup));
    const exported = [...groups].flatMap(group =>
      Object.keys(screenStories(group)).filter(key => key !== 'default'),
    );
    expect(exported.sort()).toEqual([...previewIds()].sort());
    const storyIds = previewIds().map(storyIdOf);
    expect(new Set(storyIds).size).toBe(storyIds.length);
  });

  it('files each story under its group', () => {
    expect(storyIdOf('discover__home__1-1')).toBe(
      'screens-discover--discover-home-1-1',
    );
    expect(storyIdOf('extra/flow-account')).toBe(
      'screens-extra--extra-flow-account',
    );
    // Letters followed by digits are split by Storybook's export-name rule.
    expect(storyIdOf('backpack__equip01__373-9823')).toBe(
      'screens-backpack--backpack-equip-01-373-9823',
    );
  });
});
