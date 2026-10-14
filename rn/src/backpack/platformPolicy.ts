import { Platform } from 'react-native';
import { core } from '../core';

/** What the empty-state hint offers: a Marketplace link, a plain "Shop" mention, or nothing. */
export type EmptyCopy = 'marketplace_link' | 'shop' | 'none';

/** Store rules decide it (legacy `no_items_message.gd`): iOS shows no hint, Android names the Shop. */
export function defaultEmptyCopy(os: string = Platform.OS): EmptyCopy {
  if (os === 'ios') {
    return 'none';
  }
  return os === 'android' ? 'shop' : 'marketplace_link';
}

let current: EmptyCopy | null = null;

/** Injects the policy (tests, previews, a store-policy check); `null` goes back to the platform default. */
export function setEmptyCopy(policy: EmptyCopy | null): void {
  current = policy;
}

export const getEmptyCopy = (): EmptyCopy => current ?? defaultEmptyCopy();

/** The storefront's wearables listing, in the environment the core runs in. */
export const marketplaceUrl = (): string =>
  core().marketplaceBrowseUrl('wearables');
