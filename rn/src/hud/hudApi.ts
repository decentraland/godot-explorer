import { core } from '../core';
import { onCoreEvents } from '../core/events';
import type {
  ChatLine,
  Control,
  Destination,
  LoadingDetail,
} from '../core/native';
import { nameColor } from './chatModel';
import type { ChatProfile, HudEvent, LoadingProgress } from './types';

/** What the world HUD needs from the core. */
export interface HudApi {
  subscribe(listener: (event: HudEvent) => void): () => void;
  send(command: Control): void;
  enterWorld(destination: Destination): void;
  resolveProfile(address: string): Promise<ChatProfile>;
  ownProfile(): Promise<ChatProfile>;
  /** Avatars near the player, for mention autocomplete. */
  nearby(): Promise<ChatProfile[]>;
}

/** Placeholder identity until the core knows the name: short address, last four hex as the tag. */
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

type ProfileLike = {
  address: string;
  name: string;
  hasClaimedName: boolean;
  profilePictureUrl: string;
};

function toChatProfile(data: ProfileLike, own = false): ChatProfile {
  const tag = data.hasClaimedName ? '' : `#${data.address.slice(-4)}`;
  return {
    address: data.address,
    name: data.name,
    tag,
    claimed: data.hasClaimedName,
    color: nameColor(`${data.name}${tag}`),
    own,
    ...(data.profilePictureUrl ? { face: data.profilePictureUrl } : {}),
  };
}

/** The core's profile when it answers, the short-address placeholder otherwise. */
function lookup(
  address: string | undefined,
  own: boolean,
): Promise<ChatProfile> {
  return core()
    .profile(address)
    .then(profile => toChatProfile(profile, own))
    .catch(() => ({ ...fallbackProfile(address ?? '0x0000000'), own }));
}

/** The engine's chat line carries a profile without the address; key it by the sender. */
function chatEvent({ profile, ...line }: ChatLine): HudEvent {
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

const progressOf = (detail: LoadingDetail): LoadingProgress => ({
  loaded: Number(detail.loaded),
  total: Number(detail.total),
  mbps: detail.mbps,
  percent: detail.percent,
});

export const coreHudApi: HudApi = {
  subscribe: listener =>
    onCoreEvents({
      chat: line => listener(chatEvent(line)),
      unreadChat: count => listener({ type: 'unread_chat', data: { count } }),
      loadingDetail: detail =>
        listener({ type: 'loading_progress', data: progressOf(detail) }),
      openBackpack: emotes =>
        listener({ type: 'open_backpack', data: { emotes } }),
    }),
  send: command => core().control(command),
  enterWorld: destination => core().enterWorld(destination),
  resolveProfile: address => lookup(address, false),
  ownProfile: () => lookup(undefined, true),
  // The core has no nearby list; suggestions come back with friends in TypeScript (phase 6.9).
  nearby: async () => [],
};
