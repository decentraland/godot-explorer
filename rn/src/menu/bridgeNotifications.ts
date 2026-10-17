import { nameColor } from '../hud/chatModel';
import type { MenuNotification, NotificationType } from './types';

type Raw = Record<string, unknown>;

const TYPES: Record<string, NotificationType> = {
  social_service_friendship_request: 'friendship_request',
  social_service_friendship_accepted: 'friendship_accepted',
  events_started: 'event',
  reward_assignment: 'reward',
};

const str = (value: unknown): string =>
  typeof value === 'string' ? value : '';
const obj = (value: unknown): Raw =>
  typeof value === 'object' && value !== null ? (value as Raw) : {};

export function parseNotification(raw: Raw): MenuNotification | null {
  const id = str(raw.id);
  if (!id) {
    return null;
  }
  const type = TYPES[str(raw.type)] ?? 'other';
  const meta = obj(raw.metadata);
  const sender = obj(meta.sender);
  const base = {
    id,
    type,
    read: raw.read === true,
    // The service sends the timestamp as a number or as a numeric string.
    timestamp: Number(raw.timestamp) || 0,
  };
  if (type === 'friendship_request' || type === 'friendship_accepted') {
    const name = str(sender.name);
    return {
      ...base,
      subject: name,
      image: str(sender.profileImageUrl),
      avatarColor: nameColor(name),
    };
  }
  if (type === 'event') {
    const eventId = new URL(str(meta.link) || 'https://x/').searchParams.get(
      'id',
    );
    return {
      ...base,
      subject: str(meta.title),
      image: str(meta.image),
      ...(eventId ? { eventId } : {}),
    };
  }
  if (type === 'reward') {
    return {
      ...base,
      subject: str(meta.tokenName) || str(meta.title),
      image: str(meta.tokenImage),
    };
  }
  return {
    ...base,
    subject: str(meta.title) || str(raw.title),
    text: str(meta.description) || str(raw.description),
    image: str(meta.image),
  };
}

export function parseNotifications(raw: Array<Raw>): MenuNotification[] {
  return raw
    .map(parseNotification)
    .filter((n): n is MenuNotification => n !== null)
    .sort((a, b) => b.timestamp - a.timestamp);
}
