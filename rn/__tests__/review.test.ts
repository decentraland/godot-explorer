import { NativeModules, Platform } from 'react-native';
import { setCoreForTests } from '../src/core';
import type { CoreApi } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { Rail } from '../src/core/native';
import {
  PROMPT_DELAY_MS,
  maybeAskForReview,
} from '../src/notifications/review';

describe('review prompt', () => {
  const original = Platform.OS;
  let requestReview: jest.Mock;
  beforeEach(() => {
    Platform.OS = 'android';
    requestReview = jest.fn().mockResolvedValue(true);
    NativeModules.DclPush = { requestReview };
  });
  afterEach(() => {
    Platform.OS = original;
    delete NativeModules.DclPush;
    setCoreForTests(undefined);
  });

  const withCore = (patch: Partial<CoreApi>) =>
    setCoreForTests({ ...createMockCore(), ...patch } as CoreApi);

  it('waits the beat, asks the core again and launches the card for a counted shot', async () => {
    const waits: number[] = [];
    withCore({ reviewCheck: () => true, reviewFire: () => 1 });
    await maybeAskForReview('place_favorited', async ms => {
      waits.push(ms);
    });
    expect(waits).toEqual([PROMPT_DELAY_MS]);
    expect(requestReview).toHaveBeenCalledTimes(1);
  });

  it('does nothing when the core says it is not the moment', async () => {
    const wait = jest.fn();
    withCore({ reviewCheck: () => false });
    await maybeAskForReview('place_upvoted', wait);
    expect(wait).not.toHaveBeenCalled();
    expect(requestReview).not.toHaveBeenCalled();
  });

  it('does not launch when the rails went up during the beat', async () => {
    const fire = jest.fn().mockReturnValue(undefined);
    withCore({ reviewCheck: () => true, reviewFire: fire });
    await maybeAskForReview('wearable_claimed', async () => {});
    expect(fire).toHaveBeenCalledWith('wearable_claimed', expect.any(Array));
    expect(requestReview).not.toHaveBeenCalled();
  });

  it('names the loading rail while a world loads', async () => {
    const check = jest.fn().mockReturnValue(false);
    const base = createMockCore();
    withCore({
      reviewCheck: check,
      state: () => ({ tag: 'Loading' } as never),
    });
    expect(base).toBeDefined();
    await maybeAskForReview('place_favorited');
    expect(check).toHaveBeenCalledWith([Rail.Loading]);
  });

  it('is a no-op without the Android module', async () => {
    delete NativeModules.DclPush;
    const check = jest.fn();
    withCore({ reviewCheck: check });
    await maybeAskForReview('place_favorited');
    expect(check).not.toHaveBeenCalled();
  });
});
