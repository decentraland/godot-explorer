import { NativeModules, Platform } from 'react-native';
import { core } from '../core';
import { Rail } from '../core/native';
import { modalApi } from '../modals/bridge';

export type ReviewTrigger =
  | 'place_favorited'
  | 'wearable_claimed'
  | 'place_upvoted';

/** Beat between the action and the card, from the core's policy. */
export const PROMPT_DELAY_MS = 1200;

const native = () =>
  Platform.OS === 'android'
    ? (NativeModules.DclPush as
        | { requestReview?: () => Promise<boolean> }
        | undefined)
    : undefined;

/** What is up right now that would make a rating card land badly. */
export function activeRails(): Rail[] {
  const rails: Rail[] = [];
  if (core().state().tag === 'Loading') {
    rails.push(Rail.Loading);
  }
  if (modalApi()?.isOpen()) {
    rails.push(Rail.Modal);
  }
  return rails;
}

/** Called right after something good happened. The core decides whether this is a shot; the card is
 * the store's own, with no UI of ours in front of it. */
export async function maybeAskForReview(
  trigger: ReviewTrigger,
  wait: (ms: number) => Promise<void> = ms =>
    new Promise(resolve => setTimeout(resolve, ms)),
): Promise<void> {
  const review = native()?.requestReview;
  if (!review || !core().reviewCheck(activeRails())) {
    return;
  }
  await wait(PROMPT_DELAY_MS);
  // The app can change during the beat, so the core is asked again and counts the shot only now.
  if (core().reviewFire(trigger, activeRails()) !== undefined) {
    await native()?.requestReview?.();
  }
}
