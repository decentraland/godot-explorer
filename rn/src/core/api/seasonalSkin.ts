import type * as Native from '../native';

/** The seasonal Discover reskin: which season is active and how many times its assets may be fetched. */
export type SeasonalSkinApi = Pick<
  typeof Native,
  'seasonalSkin' | 'seasonalSkinTakeFetchAttempt'
>;
