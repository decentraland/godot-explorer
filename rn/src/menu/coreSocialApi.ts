import { core } from '../core';
import {
  CoreError,
  NotificationKind_Tags,
  type Friend as CoreFriend,
  type NearbyUser,
  type Notification,
} from '../core/native';
import { nameColor } from '../hud/chatModel';
import type {
  Friend,
  FriendRequest,
  MenuApi,
  MenuNotification,
  NotificationsResult,
  Person,
  SocialData,
} from './types';

/** Thrown by `social()` when the account has no social service (guests). */
export class GuestError extends Error {}

const LIST_LIMIT = 0;
const NOTIFICATION_LIMIT = 50;

export function isGuestError(error: unknown): boolean {
  if (error instanceof GuestError) {
    return true;
  }
  return (
    error instanceof CoreError.InvalidCall &&
    /not authenticated|not signed in/i.test(error.inner.detail)
  );
}

export function toPerson(raw: CoreFriend | NearbyUser): Person {
  const tag = raw.hasClaimedName ? '' : `#${raw.address.slice(-4)}`;
  return {
    address: raw.address,
    name: raw.name || raw.address.slice(0, 6),
    claimed: raw.hasClaimedName,
    avatar: raw.profilePictureUrl,
    color: nameColor(`${raw.name}${tag}`),
  };
}

export function toMenuNotification(raw: Notification): MenuNotification {
  const base = { id: raw.id, read: raw.read, timestamp: Number(raw.timestamp) };
  const { kind } = raw;
  switch (kind.tag) {
    case NotificationKind_Tags.FriendshipRequest:
    case NotificationKind_Tags.FriendshipAccepted: {
      const { name, profileImageUrl } = kind.inner.sender;
      return {
        ...base,
        type:
          kind.tag === NotificationKind_Tags.FriendshipRequest
            ? 'friendship_request'
            : 'friendship_accepted',
        subject: name,
        image: profileImageUrl,
        avatarColor: nameColor(name),
      };
    }
    case NotificationKind_Tags.EventStarted:
      return {
        ...base,
        type: 'event',
        subject: kind.inner.title,
        image: kind.inner.image,
        ...(kind.inner.eventId ? { eventId: kind.inner.eventId } : {}),
      };
    case NotificationKind_Tags.Reward:
      return {
        ...base,
        type: 'reward',
        subject: kind.inner.tokenName || kind.inner.title,
        image: kind.inner.tokenImage,
      };
  }
}

const toMenuNotifications = (items: Notification[]): MenuNotification[] =>
  items.map(toMenuNotification).sort((a, b) => b.timestamp - a.timestamp);

type SocialMenuApi = Pick<
  MenuApi,
  | 'social'
  | 'notifications'
  | 'markRead'
  | 'markAllRead'
  | 'acceptRequest'
  | 'rejectRequest'
  | 'cancelRequest'
  | 'sendRequest'
  | 'unblock'
>;

/** Friends and notifications of the menu, served by the core. */
export function createCoreSocialApi(): SocialMenuApi {
  return {
    social: async (): Promise<SocialData> => {
      const [list, requests, nearby] = await Promise.all([
        core().friends(LIST_LIMIT, 0),
        core().friendRequests(),
        core()
          .nearby()
          .catch((): NearbyUser[] => []),
      ]).catch(error => {
        throw isGuestError(error) ? new GuestError() : error;
      });
      const asRequests = (
        items: CoreFriend[],
        direction: FriendRequest['direction'],
      ): FriendRequest[] => items.map(f => ({ ...toPerson(f), direction }));
      return {
        friends: list.map(
          (f): Friend => ({ ...toPerson(f), status: 'offline' }),
        ),
        requests: [
          ...asRequests(requests.received, 'received'),
          ...asRequests(requests.sent, 'sent'),
        ],
        nearby: nearby.map(toPerson),
      };
    },
    notifications: async (): Promise<NotificationsResult> => {
      try {
        const items = await core().notifications(
          undefined,
          NOTIFICATION_LIMIT,
          false,
        );
        return { status: 'ok', items: toMenuNotifications(items) };
      } catch (error) {
        return { status: isGuestError(error) ? 'guest' : 'error' };
      }
    },
    markRead: async id => {
      await core().markNotificationsRead([id]);
    },
    markAllRead: async () => {
      const unread = await core().notifications(
        undefined,
        NOTIFICATION_LIMIT,
        true,
      );
      if (unread.length > 0) {
        await core().markNotificationsRead(unread.map(n => n.id));
      }
    },
    acceptRequest: address => core().acceptFriendRequest(address),
    rejectRequest: address => core().rejectFriendRequest(address),
    cancelRequest: address => core().cancelFriendRequest(address),
    sendRequest: address => core().sendFriendRequest(address, ''),
    unblock: address => core().unblockUser(address),
  };
}
