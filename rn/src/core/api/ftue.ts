import type * as Native from '../native';

/** First-time guidance in the world: the shell reports what the player did, the core decides what
 * shows next, remembers it and records the analytics. */
export type FtueApi = Pick<
  typeof Native,
  'ftueTuning' | 'ftueSceneEntered' | 'controlsFtueDismissed'
>;
