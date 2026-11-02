import { startLinkHandling } from '../flow/browser';
import { startCoreReports } from '../sentry';

let started = false;

/** What runs once the app mounts: the core's reports go to Sentry, links start being handled.
 * Sentry itself is initialised earlier, in index.js, so launch-time native crashes are caught. */
export function startApp() {
  if (started) {
    return;
  }
  started = true;
  startCoreReports();
  startLinkHandling();
}
