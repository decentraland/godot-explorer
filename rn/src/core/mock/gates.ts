import type { GatesApi } from '../api/gates';
import {
  AccessVerdict,
  CampaignFallback,
  DestinationKind,
  Environment,
  ResolveFailure,
  SupportStatus,
  ReportVerdict,
  UpdateDecision,
  type Destination,
  type Parcel,
  type ResolvedDestination,
  type Urls,
} from '../native';

const HOST = 'https://decentraland.org';
const MARKETPLACE = `${HOST}/shop`;
const PEER = 'https://peer.decentraland.org';
const MOBILE_OPEN = 'https://mobile.dclexplorer.com/open';
const WORLDS = 'https://worlds-content-server.decentraland.org/world/';
const MAIN_REALM = 'https://realm-provider-ea.decentraland.org/main';
const GENESIS_BOUND = 150;
const TERMS_VERSION = 1;

/** The production-like host set of the mock core; the one place TypeScript spells hosts, besides tests. */
const URLS: Urls = Object.freeze({
  host: HOST,
  help: `${HOST}/help`,
  privacyPolicy: `${HOST}/privacy`,
  termsOfService: `${HOST}/terms`,
  contentPolicy: `${HOST}/content`,
  placesApi: 'https://places.decentraland.org/api',
  places: 'https://places.decentraland.org/api/places',
  eventsApi: 'https://events.decentraland.org/api/events',
  jumpEvents: `${HOST}/jump/events`,
  mainRealm: MAIN_REALM,
  worldsContentServer: WORLDS,
  peerBase: PEER,
  peerContent: `${PEER}/content/`,
  peerLambdas: `${PEER}/lambdas/`,
  mobileBff: 'https://mobile-bff.decentraland.org',
  destinationsApi: 'https://mobile-bff.decentraland.org/destinations',
  mobileEventsApi: 'https://mobile-bff.decentraland.org/events',
  accountDeletion: 'https://mobile-bff.decentraland.org/deletion',
  notificationsApi: 'https://notifications.decentraland.org',
  creditsServer: 'https://credits.decentraland.org',
  socialService: 'wss://rpc-social-service-ea.decentraland.org',
  marketplace: MARKETPLACE,
  marketplaceApi: 'https://marketplace-api.decentraland.org',
  marketplaceCatalogApi:
    'https://marketplace-api.decentraland.org/v3/catalog/unified',
  marketplaceClaimName: `${MARKETPLACE}/items?category=names`,
  authFrontend: `${HOST}/auth/requests`,
  authMobileFrontend: `${HOST}/auth/mobile`,
  authApiBase: 'https://auth-api.decentraland.org',
  authApiRequests: 'https://auth-api.decentraland.org/requests',
  mobileOpen: MOBILE_OPEN,
  devRealms: [
    MAIN_REALM,
    'https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest',
    `${WORLDS}olavra.dcl.eth`,
  ],
});

export const mockUrls = (): Urls => URLS;

/** Reserved characters escaped like the core does: only unreserved ones stay. */
const encode = (text: string): string =>
  Array.from(new TextEncoder().encode(text), byte =>
    /[A-Za-z0-9\-_.~]/.test(String.fromCharCode(byte))
      ? String.fromCharCode(byte)
      : `%${byte.toString(16).toUpperCase().padStart(2, '0')}`,
  ).join('');

export const mockWorldRealm = (name: string): string =>
  `${WORLDS}${encode(name.toLowerCase())}`;

const browsePath = (section: string): string =>
  section === 'wearables'
    ? '/items?category=wearable'
    : section === 'emotes'
    ? '/items?category=emote'
    : '/items';

const trimSlash = (url: string): string => url.replace(/\/+$/, '');

const sceneLink = (realm: string, x: number, y: number): string => {
  const genesis = trimSlash(realm) === trimSlash(MAIN_REALM);
  const name = trimSlash(realm).split('/').pop() ?? '';
  return `${MOBILE_OPEN}?${genesis ? `position=${x},${y}` : `realm=${name}`}`;
};

type Intent = { realm: string; parcel?: Parcel };

const LINK_HOSTS = [
  'decentraland.org',
  'decentraland.zone',
  'mobile.dclexplorer.com',
];

const coordinates = (text: string): Parcel | undefined => {
  const match = /^\(?\s*(-?\d+)\s*,\s*(-?\d+)\s*\)?$/.exec(text.trim());
  return match ? { x: Number(match[1]), y: Number(match[2]) } : undefined;
};

const looksLikeRealm = (text: string): boolean =>
  !/\s/.test(text) && !text.includes(',') && /\.|:\/\//.test(text);

function linkIntent(input: string): Intent | undefined {
  const match = /^([a-z][a-z0-9+.-]*):\/\/([^/?#]*)[^?#]*\??([^#]*)/i.exec(
    input,
  );
  if (!match) {
    return undefined;
  }
  const [, scheme, host, query] = match;
  const isLink =
    scheme === 'decentraland' ||
    ((scheme === 'http' || scheme === 'https') && LINK_HOSTS.includes(host));
  if (!isLink) {
    return undefined;
  }
  const params = new Map<string, string>(
    query
      .split('&')
      .map((pair): [string, string] => {
        const [key, ...rest] = pair.split('=');
        return [key, decodeURIComponent(rest.join('='))];
      })
      .filter(([key]) => key !== ''),
  );
  const realm = params.get('realm') || params.get('preview') || '';
  const parcel = coordinates(
    params.get('position') ?? params.get('location') ?? '',
  );
  return realm || parcel ? { realm, parcel } : { realm: '' };
}

/** The accepted shapes of the core's destination grammar; `undefined` when the input names nothing. */
function parseInput(raw: string): Intent | undefined {
  const input = raw.trim();
  if (!input) {
    return undefined;
  }
  const link = linkIntent(input);
  if (link) {
    return link.realm || link.parcel ? link : undefined;
  }
  const alone = coordinates(input);
  if (alone) {
    return { realm: '', parcel: alone };
  }
  for (const at of Array.from(input.matchAll(/\s/g), m => m.index ?? 0)) {
    const parcel = coordinates(input.slice(at));
    if (parcel) {
      const realm = input.slice(0, at).trim();
      return looksLikeRealm(realm) ? { realm, parcel } : undefined;
    }
  }
  return looksLikeRealm(input) ? { realm: input } : undefined;
}

const PREVIEW_HOST = /^https?:\/\/(localhost|\d{1,3}(\.\d{1,3}){3})(:|\/|$)/;

const refused = (failure: ResolveFailure): ResolvedDestination => ({
  kind: DestinationKind.Genesis,
  access: AccessVerdict.Allowed,
  failure,
  ready: false,
  worldName: '',
  sceneId: '',
  title: '',
  creator: '',
  imageUrl: '',
  assetCount: 0,
  destination: {},
});

function resolve(input: string): ResolvedDestination {
  const intent = parseInput(input);
  if (!intent) {
    return refused(ResolveFailure.InvalidInput);
  }
  const { realm, parcel } = intent;
  const isWorldName = realm !== '' && !realm.includes('://');
  const kind =
    realm === ''
      ? DestinationKind.Genesis
      : isWorldName
      ? DestinationKind.World
      : PREVIEW_HOST.test(realm)
      ? DestinationKind.Preview
      : DestinationKind.CustomRealm;
  if (
    kind === DestinationKind.Genesis &&
    parcel &&
    (Math.abs(parcel.x) > GENESIS_BOUND || Math.abs(parcel.y) > GENESIS_BOUND)
  ) {
    return refused(ResolveFailure.ParcelOutOfBounds);
  }
  const destination: Destination = {
    realm:
      realm === '' ? undefined : isWorldName ? mockWorldRealm(realm) : realm,
    parcel,
  };
  return {
    kind,
    access: AccessVerdict.Allowed,
    failure: undefined,
    ready: true,
    worldName: isWorldName ? realm.toLowerCase() : '',
    parcel,
    sceneId: '',
    title: '',
    creator: '',
    imageUrl: '',
    assetCount: 0,
    destination,
  };
}

/** Production-like answers for builds without the engine library; the terms acceptance lives in memory per instance. */
export function createMockGates(): GatesApi {
  let acceptedTerms = 0;

  return {
    urls: mockUrls,
    environment: () => Environment.Org,
    marketplaceBrowseUrl: section => `${MARKETPLACE}${browsePath(section)}`,
    marketplaceItemUrl: (contract, item) =>
      `${MARKETPLACE}/item/${contract}/${item}`,
    jumpRealmUrl: realm => `${HOST}/jump/?realm=${encode(realm)}`,
    jumpPositionUrl: position => `${HOST}/jump/?position=${encode(position)}`,
    eventUrl: eventId => `${HOST}/events/event/?id=${encode(eventId)}`,
    sceneLinkUrl: sceneLink,

    versionGate: async () => ({
      decision: UpdateDecision.Proceed,
      current: 0n,
      minimalRequired: 0n,
      recommended: 0n,
    }),
    snoozeVersionGate: () => {},

    deviceSupport: async () => ({
      status: SupportStatus.Supported,
      verdict: ReportVerdict.NotChecked,
      modal: undefined,
      soc: '',
    }),
    deviceSupportModalShown: () => {},

    pendingCampaign: async () => undefined,
    campaignResolution: async () => ({
      token: '',
      campaign: undefined,
      fallback: CampaignFallback.NoToken,
    }),
    captureCampaignToken: () => false,
    consumeCampaign: () => {},

    resolveDestination: async input => resolve(input),
    resolveDestinationWithSecret: async input => resolve(input),

    termsVersion: () => TERMS_VERSION,
    termsAcceptedVersion: () => acceptedTerms,
    acceptTerms: version => {
      acceptedTerms = version;
    },
  };
}
