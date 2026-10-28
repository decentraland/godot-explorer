import type * as Native from '../native';

/** The scene slice of the generated bindings. */
export type SceneApi = Pick<
  typeof Native,
  | 'currentScene'
  | 'captureScreenshot'
  | 'track'
  | 'trackScreen'
  | 'setAnalyticsConsent'
  | 'flushAnalytics'
  | 'setAnalyticsFlushInterval'
>;
