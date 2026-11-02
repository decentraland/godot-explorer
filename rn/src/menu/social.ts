import type { Friend, FriendRequest, Person, SocialData } from './types';

export type SocialTab = 'friends' | 'requests' | 'nearby';

export type SocialState =
  | { status: 'loading' }
  | { status: 'error' }
  | { status: 'guest' }
  | ({ status: 'ready' } & SocialData);

export type SocialAction =
  | { type: 'loaded'; data: SocialData }
  | { type: 'failed' }
  | { type: 'loading' }
  | { type: 'guest' }
  | { type: 'accepted'; address: string }
  | { type: 'removed'; address: string }
  | { type: 'requested'; address: string }
  | { type: 'unblocked'; address: string };

export function socialReducer(
  state: SocialState,
  action: SocialAction,
): SocialState {
  if (action.type === 'loaded') {
    return { status: 'ready', ...action.data };
  }
  if (action.type === 'failed') {
    return { status: 'error' };
  }
  if (action.type === 'loading') {
    return { status: 'loading' };
  }
  if (action.type === 'guest') {
    return { status: 'guest' };
  }
  if (state.status !== 'ready') {
    return state;
  }
  const { address } = action;
  switch (action.type) {
    case 'accepted': {
      const accepted = state.requests.find(
        r => r.address === address && r.direction === 'received',
      );
      const friend: Friend[] = accepted
        ? [{ ...stripDirection(accepted), status: 'offline' }]
        : [];
      return {
        ...state,
        friends: [...state.friends, ...friend],
        requests: state.requests.filter(r => r.address !== address),
      };
    }
    case 'removed':
      return {
        ...state,
        requests: state.requests.filter(r => r.address !== address),
      };
    case 'requested':
      return {
        ...state,
        nearby: state.nearby.filter(p => p.address !== address),
      };
    case 'unblocked':
      return {
        ...state,
        friends: state.friends.map(f =>
          f.address === address && f.status === 'blocked'
            ? { ...f, status: 'offline' }
            : f,
        ),
      };
  }
}

function stripDirection(r: FriendRequest): Person {
  return {
    address: r.address,
    name: r.name,
    claimed: r.claimed,
    avatar: r.avatar,
    color: r.color,
    place: r.place,
  };
}

export function groupFriends(friends: Friend[]) {
  return {
    online: friends.filter(f => f.status === 'online'),
    offline: friends.filter(f => f.status === 'offline'),
    blocked: friends.filter(f => f.status === 'blocked'),
  };
}

export function groupRequests(requests: FriendRequest[]) {
  return {
    received: requests.filter(r => r.direction === 'received'),
    sent: requests.filter(r => r.direction === 'sent'),
  };
}

/** Counts shown next to the tabs and used for the REQUESTS dot. */
export function tabBadges(state: SocialState) {
  if (state.status !== 'ready') {
    return { requests: 0, nearby: 0 };
  }
  return {
    requests: groupRequests(state.requests).received.length,
    nearby: state.nearby.length,
  };
}
