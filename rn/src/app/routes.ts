import type { Place } from '../discover/api';
import type { NavKey } from '../ds';

/**
 * Route name -> params. Feature packages add their sheets by augmenting this interface
 * (`declare module '../app/routes' { interface RouteParams { ... } }`) and calling `registerScreen`.
 */
export interface RouteParams {
  home: undefined;
  world: { place: Place };
  placeDetail: { placeId: string };
  eventDetail: { eventId: string };
  friendDetail: { address: string };
  profileEdit: undefined;
  /** Tab bodies other than explore/backpack; rendered inside the home shell. */
  settings: undefined;
  profile: undefined;
}

export type RouteName = keyof RouteParams;

export type Route = {
  [K in RouteName]: RouteParams[K] extends undefined
    ? { name: K; params?: undefined }
    : { name: K; params: RouteParams[K] };
}[RouteName];

/** Extra mount points a package can fill without touching the shell. */
export type SlotName = 'worldDrawer';

export type Tab = NavKey;
