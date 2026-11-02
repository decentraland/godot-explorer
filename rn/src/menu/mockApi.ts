import type { MenuApi, NotificationsResult, SocialData } from './types';
import {
  mockEvents,
  mockFeatured,
  mockNotifications,
  mockScene,
  mockSocial,
} from './mockData';

export type MockOptions = {
  guest?: boolean;
  /** Make the named loaders reject. */
  fail?: ('social' | 'notifications' | 'discover')[];
  social?: SocialData;
  notifications?: NotificationsResult;
};

/** Deterministic in-memory implementation; every call resolves on the next tick. */
export function createMockApi(options: MockOptions = {}): MenuApi {
  let social = options.social ?? mockSocial;
  let notes = mockNotifications;
  const fails = (name: 'social' | 'notifications' | 'discover') =>
    options.fail?.includes(name) ?? false;
  const later = <T>(fn: () => T): Promise<T> =>
    new Promise((resolve, reject) =>
      setTimeout(() => {
        try {
          resolve(fn());
        } catch (e) {
          reject(e);
        }
      }, 0),
    );
  const dropRequest = (address: string) => {
    social = {
      ...social,
      requests: social.requests.filter(r => r.address !== address),
    };
  };

  return {
    isGuest: () => options.guest ?? false,
    scene: () => later(() => mockScene),
    discover: () =>
      later(() => {
        if (fails('discover')) {
          throw new Error('discover failed');
        }
        return {
          featured: mockFeatured,
          events: mockEvents,
          featuredEvents: ['event-2'],
        };
      }),
    social: () =>
      later(() => {
        if (fails('social')) {
          throw new Error('social failed');
        }
        return social;
      }),
    notifications: () =>
      later((): NotificationsResult => {
        if (options.notifications) {
          return options.notifications;
        }
        if (options.guest) {
          return { status: 'guest' };
        }
        return fails('notifications')
          ? { status: 'error' }
          : { status: 'ok', items: notes };
      }),
    markRead: id =>
      later(() => {
        notes = notes.map(n => (n.id === id ? { ...n, read: true } : n));
      }),
    markAllRead: () =>
      later(() => {
        notes = notes.map(n => ({ ...n, read: true }));
      }),
    acceptRequest: address => later(() => dropRequest(address)),
    rejectRequest: address => later(() => dropRequest(address)),
    cancelRequest: address => later(() => dropRequest(address)),
    sendRequest: address =>
      later(() => {
        social = {
          ...social,
          nearby: social.nearby.filter(p => p.address !== address),
        };
      }),
    unblock: address =>
      later(() => {
        social = {
          ...social,
          friends: social.friends.map(f =>
            f.address === address ? { ...f, status: 'offline' } : f,
          ),
        };
      }),
    jumpToFriend: () => later(() => undefined),
    setReminder: () => later(() => undefined),
    captureScreenshot: () => later(() => 'mock://world-capture.jpg'),
    submitBugReport: () => later(() => undefined),
  };
}
