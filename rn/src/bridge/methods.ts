// Typed method map for the generic req/res channel (see request.ts and rn/README.md).

export type SettingValue = string | number | boolean;

export type Friend = {
  address: string;
  name: string;
  has_claimed_name: boolean;
  profile_picture_url: string;
  friendship_id?: string;
};

export type AddressParams = { address: string };

export type ProfileFields = {
  name?: string;
  description?: string;
  country?: string;
  pronouns?: string;
  gender?: string;
  sexual_orientation?: string;
  relationship_status?: string;
  employment_status?: string;
  profession?: string;
  real_name?: string;
  hobbies?: string;
  links?: Array<{ title: string; url: string }>;
  has_claimed_name?: boolean;
};

export type ProfileData = ProfileFields & {
  address: string;
  version?: number;
  profile_picture_url?: string;
  avatar?: Record<string, unknown>;
};

export type SignedFetchParams = {
  url: string;
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  body?: string;
  headers?: Record<string, string>;
};

export type SignedFetchResult = { status: number; body: unknown };

export type BridgeMethods = {
  'settings.get_all': { params: void; result: Record<string, SettingValue> };
  'settings.set': {
    params: { key: string; value: SettingValue; commit?: boolean };
    result: { key: string; value: SettingValue };
  };
  'settings.cache_size': {
    params: void;
    result: { bytes: number; max_bytes: number };
  };
  'settings.clear_cache': { params: void; result: { bytes: number } };
  'settings.version': { params: void; result: { version: string } };
  'account.sign_out': { params: void; result: Record<string, never> };
  'account.delete': { params: void; result: Record<string, never> };
  'friends.list': {
    params: { limit?: number; offset?: number } | void;
    result: { friends: Friend[] };
  };
  'friends.requests': {
    params: void;
    result: { received: Friend[]; sent: Friend[] };
  };
  'friends.nearby': { params: void; result: { users: Friend[] } };
  'friends.accept': { params: AddressParams; result: Record<string, never> };
  'friends.reject': { params: AddressParams; result: Record<string, never> };
  'friends.cancel': { params: AddressParams; result: Record<string, never> };
  'friends.send': {
    params: AddressParams & { message?: string };
    result: Record<string, never>;
  };
  'friends.unblock': { params: AddressParams; result: Record<string, never> };
  'friends.block': { params: AddressParams; result: Record<string, never> };
  'notifications.list': {
    params: { only_unread?: boolean; limit?: number; from?: number } | void;
    result: { notifications: Array<Record<string, unknown>> };
  };
  'notifications.read': {
    params: { ids: string[] };
    result: { updated: number };
  };
  'profile.get': { params: { address?: string } | void; result: ProfileData };
  'profile.names': { params: void; result: { names: string[] } };
  'profile.deploy': {
    params: { fields: ProfileFields };
    result: Record<string, never>;
  };
  'fetch.signed': { params: SignedFetchParams; result: SignedFetchResult };
  'places.favorite': {
    params: { place_id: string; value: boolean; is_world?: boolean };
    result: Record<string, never>;
  };
  'places.vote': {
    params: {
      place_id: string;
      vote: 'up' | 'down' | 'none';
      is_world?: boolean;
    };
    result: Record<string, never>;
  };
  'scene.current': {
    params: void;
    result: {
      title: string;
      creator: string;
      parcel: { x: number; y: number };
      realm: string;
      genesis: boolean;
    };
  };
  'clipboard.copy': { params: { text: string }; result: Record<string, never> };
  'screenshot.capture': {
    params: void;
    result: { path: string; base64?: string };
  };
};

export type MethodName = keyof BridgeMethods;
export type MethodParams<M extends MethodName> = BridgeMethods[M]['params'];
export type MethodResult<M extends MethodName> = BridgeMethods[M]['result'];
