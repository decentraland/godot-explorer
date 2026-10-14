import type * as Native from '../native';

/** The social slice of the generated bindings. */
export type SocialApi = Pick<
  typeof Native,
  | 'friends'
  | 'blockedUsers'
  | 'friendRequests'
  | 'nearby'
  | 'sendFriendRequest'
  | 'acceptFriendRequest'
  | 'rejectFriendRequest'
  | 'cancelFriendRequest'
  | 'blockUser'
  | 'unblockUser'
>;
