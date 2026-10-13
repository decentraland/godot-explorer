import { request } from '../bridge/request';
import type { Friend as BridgeFriend } from '../bridge/methods';
import { fetchDiscover, MAIN_REALM } from '../discover/api';
import { nameColor } from '../hud/chatModel';
import { parseNotifications } from './bridgeNotifications';
import { linkQueryFor } from './api';
import type {
  DiscoverFeed,
  Friend,
  FriendRequest,
  MenuApi,
  NotificationsResult,
  Person,
  SceneInfo,
  SocialData,
} from './types';

export function toPerson(raw: BridgeFriend): Person {
  const tag = raw.has_claimed_name ? '' : `#${raw.address.slice(-4)}`;
  return {
    address: raw.address,
    name: raw.name || raw.address.slice(0, 6),
    claimed: raw.has_claimed_name,
    avatar: raw.profile_picture_url,
    color: nameColor(`${raw.name}${tag}`),
  };
}

/** Thrown by `social()` when the engine has no social service (guest accounts). */
export class GuestError extends Error {}

const isGuestError = (error: unknown): boolean =>
  error instanceof GuestError ||
  (error instanceof Error &&
    /not authenticated|not initialized/i.test(error.message));

const FALLBACK_SCENE: SceneInfo = {
  title: 'Decentraland',
  creator: '',
  linkQuery: 'position=0,0',
};

/**
 * Scene, friends and notifications go through the Godot bridge and the discover feed comes from the
 * public places/events APIs; screenshots and bug reports stay with `base` until those are wired.
 */
export function createBridgeMenuApi(
  base: MenuApi,
  fetchImpl: typeof fetch = fetch,
): MenuApi {
  const addressAction =
    (
      method:
        | 'friends.accept'
        | 'friends.reject'
        | 'friends.cancel'
        | 'friends.send'
        | 'friends.unblock',
    ) =>
    async (address: string): Promise<void> => {
      await request(method, { address });
    };

  return {
    ...base,
    scene: async (): Promise<SceneInfo> => {
      try {
        const { title, creator, parcel, realm, genesis } = await request(
          'scene.current',
        );
        return {
          title: title || FALLBACK_SCENE.title,
          creator,
          linkQuery: linkQueryFor(
            genesis ? MAIN_REALM : realm,
            parcel.x,
            parcel.y,
          ),
        };
      } catch {
        return FALLBACK_SCENE;
      }
    },
    discover: async (): Promise<DiscoverFeed> => {
      const { featured, events } = await fetchDiscover(fetchImpl);
      return {
        featured,
        events,
        featuredEvents: events.filter(e => e.featured).map(e => e.id),
      };
    },
    social: async (): Promise<SocialData> => {
      const [list, requests, nearby] = await Promise.all([
        request('friends.list'),
        request('friends.requests'),
        request('friends.nearby').catch(() => ({ users: [] })),
      ]).catch(error => {
        throw isGuestError(error) ? new GuestError() : error;
      });
      const friends: Friend[] = list.friends.map(f => ({
        ...toPerson(f),
        status: 'offline',
      }));
      const asRequests = (
        items: BridgeFriend[],
        direction: FriendRequest['direction'],
      ): FriendRequest[] => items.map(f => ({ ...toPerson(f), direction }));
      return {
        friends,
        requests: [
          ...asRequests(requests.received, 'received'),
          ...asRequests(requests.sent, 'sent'),
        ],
        nearby: nearby.users.map(toPerson),
      };
    },
    notifications: async (): Promise<NotificationsResult> => {
      try {
        const { notifications } = await request('notifications.list', {
          limit: 50,
        });
        return { status: 'ok', items: parseNotifications(notifications) };
      } catch (error) {
        return { status: isGuestError(error) ? 'guest' : 'error' };
      }
    },
    markRead: async id => {
      await request('notifications.read', { ids: [id] });
    },
    markAllRead: async () => {
      const { notifications } = await request('notifications.list', {
        only_unread: true,
        limit: 50,
      });
      const ids = parseNotifications(notifications).map(n => n.id);
      if (ids.length > 0) {
        await request('notifications.read', { ids });
      }
    },
    acceptRequest: addressAction('friends.accept'),
    rejectRequest: addressAction('friends.reject'),
    cancelRequest: addressAction('friends.cancel'),
    sendRequest: addressAction('friends.send'),
    unblock: addressAction('friends.unblock'),
  };
}
