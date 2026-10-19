import type { DiscoverEvent, Place } from '../discover/api';

export type Person = {
  address: string;
  name: string;
  /** Claimed names get the verified badge. */
  claimed: boolean;
  /** Avatar face URL; empty renders the plain coloured disc. */
  avatar: string;
  /** Name colour derived from the name hash. */
  color: string;
  /** Current place name, when known. */
  place?: string;
};

export type Friend = Person & {
  status: 'online' | 'offline' | 'blocked';
  /** In the viewer's own scene: the jump-in button is dimmed. */
  samePlace?: boolean;
};

export type FriendRequest = Person & { direction: 'received' | 'sent' };

export type SocialData = {
  friends: Friend[];
  requests: FriendRequest[];
  nearby: Person[];
};

export type NotificationType =
  | 'friendship_accepted'
  | 'friendship_request'
  | 'event'
  | 'reward'
  | 'other';

export type MenuNotification = {
  id: string;
  type: NotificationType;
  read: boolean;
  timestamp: number;
  /** Sender nickname, event name or item name, by type. */
  subject: string;
  /** Event id for `event` notifications, to open its modal. */
  eventId?: string;
  /** Free text used by the generic fallback. */
  text?: string;
  /** Face URL (friend types) or event/item image. */
  image: string;
  avatarColor?: string;
};

export type NotificationsResult =
  | { status: 'ok'; items: MenuNotification[] }
  | { status: 'guest' }
  | { status: 'error' };

export type SceneInfo = {
  title: string;
  creator: string;
  /** Link tail for Share: `position=x,y` or `realm=name`. */
  linkQuery: string;
};

export type DiscoverFeed = {
  featured: Place[];
  events: DiscoverEvent[];
  /** Event ids that carry the Featured tag. */
  featuredEvents: string[];
};

export type BugReport = {
  category: string;
  description: string;
  screenshots: string[];
};

/** Everything the menu needs from the engine and the services; the bridge implements it later. */
export interface MenuApi {
  isGuest(): boolean;
  scene(): Promise<SceneInfo>;
  discover(): Promise<DiscoverFeed>;
  social(): Promise<SocialData>;
  notifications(): Promise<NotificationsResult>;
  markRead(id: string): Promise<void>;
  markAllRead(): Promise<void>;
  acceptRequest(address: string): Promise<void>;
  rejectRequest(address: string): Promise<void>;
  cancelRequest(address: string): Promise<void>;
  sendRequest(address: string): Promise<void>;
  unblock(address: string): Promise<void>;
  jumpToFriend(address: string): Promise<void>;
  setReminder(eventId: string, on: boolean): Promise<void>;
  /** A world JPEG as a file or data URI. */
  captureScreenshot(): Promise<string>;
  submitBugReport(report: BugReport): Promise<void>;
}
