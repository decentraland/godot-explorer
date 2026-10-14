import { CoreError } from '../native';
import type { Friend, NearbyUser } from '../native';
import type { SocialApi } from '../api/social';

const address = (n: number) => `0x${String(n).padStart(40, '0')}`;

const user = (n: number, name: string, hasClaimedName = true): NearbyUser => ({
  address: address(n),
  name,
  hasClaimedName,
  profilePictureUrl: '',
});
const withId = (who: NearbyUser, friendshipId: string): Friend => ({
  ...who,
  friendshipId,
});

const FRIENDS: Friend[] = [
  withId(user(1, 'Ana'), ''),
  withId(user(2, 'Bo'), ''),
  withId(user(3, 'Carla'), ''),
  withId(user(4, 'Dani', false), ''),
];
const RECEIVED: Friend[] = [
  withId(user(7, 'Eli'), 'fr-7'),
  withId(user(8, 'Fran', false), 'fr-8'),
];
const SENT: Friend[] = [withId(user(9, 'Gus'), 'fr-9')];
const NEARBY: NearbyUser[] = Array.from({ length: 10 }, (_, i) =>
  user(10 + i, `Visitor${i + 1}`),
);

const remove = <T extends { address: string }>(items: T[], who: string) =>
  items.filter(item => item.address !== who);

/** In-memory friends graph; `onChanged` stands in for the engine's `friendsChanged` event. */
export function createMockSocial(onChanged: () => void = () => {}): SocialApi {
  let friends = FRIENDS;
  let received = RECEIVED;
  let sent = SENT;
  let nearby = NEARBY;
  let blocked: string[] = [];

  const change = (edit: () => void) => {
    edit();
    onChanged();
  };
  const find = (list: Friend[], who: string): Friend => {
    const found = list.find(item => item.address === who);
    if (!found) {
      throw new CoreError.InvalidCall({ detail: `no request from ${who}` });
    }
    return found;
  };

  return {
    friends: async (limit, offset) =>
      friends.slice(offset, limit > 0 ? offset + limit : undefined),
    friendRequests: async () => ({ received, sent }),
    nearby: async () => nearby.filter(u => !blocked.includes(u.address)),
    sendFriendRequest: async who =>
      change(() => {
        const target = nearby.find(u => u.address === who);
        sent = [
          ...sent,
          withId(target ?? user(0, who.slice(0, 6), false), `fr-${who}`),
        ];
        nearby = remove(nearby, who);
      }),
    acceptFriendRequest: async who =>
      change(() => {
        const accepted = find(received, who);
        friends = [...friends, { ...accepted, friendshipId: '' }];
        received = remove(received, who);
      }),
    rejectFriendRequest: async who =>
      change(() => {
        find(received, who);
        received = remove(received, who);
      }),
    cancelFriendRequest: async who =>
      change(() => {
        find(sent, who);
        sent = remove(sent, who);
      }),
    blockUser: async who =>
      change(() => {
        blocked = [...blocked, who];
        friends = remove(friends, who);
        received = remove(received, who);
        sent = remove(sent, who);
      }),
    unblockUser: async who =>
      change(() => {
        blocked = blocked.filter(item => item !== who);
      }),
  };
}
