import type * as Native from '../native';

/** The places slice of the generated bindings. */
export type PlacesApi = Pick<
  typeof Native,
  | 'notifications'
  | 'markNotificationsRead'
  | 'favoritePlaces'
  | 'placeUserState'
  | 'setFavorite'
  | 'vote'
  | 'setEventReminder'
  | 'creditsBalance'
  | 'submitBugReport'
  | 'bugReportIssueTypes'
>;
