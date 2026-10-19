import { core } from '../core';
import {
  CoreError,
  NotificationKind_Tags,
  type Notification,
} from '../core/native';
import { nameColor } from '../hud/chatModel';
import type { MenuApi, MenuNotification, NotificationsResult } from './types';

/** Thrown by `social()` when the account has no social service (guests). */
export class GuestError extends Error {}

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

type NotificationsMenuApi = Pick<
  MenuApi,
  'notifications' | 'markRead' | 'markAllRead'
>;

/** The notifications of the menu, served by the core. */
export function createCoreNotificationsApi(): NotificationsMenuApi {
  return {
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
  };
}
