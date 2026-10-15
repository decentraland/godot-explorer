import type * as Native from '../native';

/** The `App Opened` chain: the shell reports what the app does, the core decides what an open is. */
export type LifecycleApi = Pick<
  typeof Native,
  | 'sessionLaunched'
  | 'sessionForegrounded'
  | 'sessionBackgrounded'
  | 'sessionTick'
  | 'reviewCheck'
  | 'reviewFire'
  | 'probeConnection'
  | 'connectionRetry'
  | 'connectionReset'
  | 'connectionHealthy'
>;
