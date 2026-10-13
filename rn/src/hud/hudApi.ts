import { sendCommand, subscribe } from '../GodotBridge';
import type { GodotCommand } from '../GodotBridge';
import { request } from '../bridge/request';
import type { Friend, ProfileData } from '../bridge/methods';
import { nameColor } from './chatModel';
import type { ChatProfile, HudBridgeEvent, HudCommand } from './types';

/** What the world HUD needs from the engine; profiles come from the req/res bridge. */
export interface HudApi {
  subscribe(listener: (event: HudBridgeEvent) => void): () => void;
  send(command: HudCommand): void;
  resolveProfile(address: string): Promise<ChatProfile>;
  ownProfile(): Promise<ChatProfile>;
  /** Avatars near the player, for mention autocomplete. */
  nearby(): Promise<ChatProfile[]>;
}

const HUD_EVENTS = new Set([
  'loading',
  'loading_progress',
  'chat',
  'hud_state',
  'unread_chat',
  'open_backpack',
]);

/** Placeholder identity until Godot sends names: short address, last four hex as the tag. */
export function fallbackProfile(address: string): ChatProfile {
  const tag = `#${address.slice(-4)}`;
  return {
    address,
    name: address.slice(0, 6),
    tag,
    claimed: false,
    color: nameColor(`${address.slice(0, 6)}${tag}`),
    own: false,
  };
}

type ProfileLike = Pick<Friend, 'address' | 'name' | 'has_claimed_name'> & {
  profile_picture_url?: string;
};

function toChatProfile(data: ProfileLike, own = false): ChatProfile {
  const claimed = Boolean(data.has_claimed_name);
  const tag = claimed ? '' : `#${data.address.slice(-4)}`;
  return {
    address: data.address,
    name: data.name,
    tag,
    claimed,
    color: nameColor(`${data.name}${tag}`),
    own,
    ...(data.profile_picture_url ? { face: data.profile_picture_url } : {}),
  };
}

/** Godot's profile when the engine answers, the short-address placeholder otherwise. */
function lookup(
  address: string | undefined,
  own: boolean,
): Promise<ChatProfile> {
  return request('profile.get', address ? { address } : undefined)
    .then(data => toChatProfile(data as ProfileData & ProfileLike, own))
    .catch(() => ({
      ...fallbackProfile(address ?? '0x0000000'),
      own,
    }));
}

type RawChat = {
  address: string;
  message: string;
  timestamp: number;
  profile?: Omit<ProfileLike, 'address'>;
};

/** Godot's chat line carries a profile without the address; key it by the sender. */
function chatWithProfile({ profile, ...line }: RawChat): HudBridgeEvent {
  return {
    type: 'chat',
    data: {
      ...line,
      ...(profile
        ? { profile: toChatProfile({ ...profile, address: line.address }) }
        : {}),
    },
  };
}

export const bridgeHudApi: HudApi = {
  subscribe(listener) {
    return subscribe(event => {
      if (!HUD_EVENTS.has(event.type)) {
        return;
      }
      listener(
        event.type === 'chat'
          ? chatWithProfile(event.data as RawChat)
          : (event as unknown as HudBridgeEvent),
      );
    });
  },
  send(command) {
    sendCommand(command as unknown as GodotCommand);
  },
  resolveProfile: address => lookup(address, false),
  ownProfile: () => lookup(undefined, true),
  nearby: () =>
    request('friends.nearby')
      .then(({ users }) => users.map(user => toChatProfile(user)))
      .catch(() => []),
};
