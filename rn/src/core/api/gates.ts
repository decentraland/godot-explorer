import type * as Native from '../native';

/** The gates slice of the generated bindings. */
export type GatesApi = Pick<
  typeof Native,
  | 'urls'
  | 'environment'
  | 'marketplaceBrowseUrl'
  | 'marketplaceItemUrl'
  | 'jumpRealmUrl'
  | 'jumpPositionUrl'
  | 'eventUrl'
  | 'sceneLinkUrl'
  | 'versionGate'
  | 'snoozeVersionGate'
  | 'deviceSupport'
  | 'deviceSupportModalShown'
  | 'pendingCampaign'
  | 'campaignResolution'
  | 'captureCampaignToken'
  | 'consumeCampaign'
  | 'resolveDestination'
  | 'resolveDestinationWithSecret'
  | 'termsVersion'
  | 'termsAcceptedVersion'
  | 'acceptTerms'
>;
