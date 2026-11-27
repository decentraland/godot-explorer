import * as Sentry from '@sentry/react-native';
import { core } from './core';
import { onCoreEvents } from './core/events';
import {
  Environment,
  ReportLevel,
  type ReportPayload,
  type ReportTag,
} from './core/native';

/** The client key of the Decentraland Explorer project (public by design, as in project.godot). */
export const SENTRY_DSN =
  'https://03559fa545b3fa2bc9e876a41d6aab2f@o4510187684298752.ingest.us.sentry.io/4510187688361984';

const RELEASE_PREFIX = 'org.decentraland.godotexplorer@';
/** What a cleared tag reads in Sentry: the native scope has no way to remove a tag. */
export const CLEARED_TAG = 'none';

const ENVIRONMENTS: Record<Environment, string> = {
  [Environment.Org]: 'production',
  [Environment.Zone]: 'staging',
  [Environment.Today]: 'development',
};

const LEVELS: Record<ReportLevel, Sentry.SeverityLevel> = {
  [ReportLevel.Debug]: 'debug',
  [ReportLevel.Info]: 'info',
  [ReportLevel.Warning]: 'warning',
  [ReportLevel.Error]: 'error',
  [ReportLevel.Fatal]: 'fatal',
};

/**
 * `<package>@<x.y.z>+<build>` when the core's version carries a build (`x.y.z.build...`, every
 * non-production build). Otherwise undefined: the SDK then names the release from the app's own
 * version name and code, which is the same string.
 */
export function releaseFor(version: string | undefined): string | undefined {
  const match = version?.match(/^(\d+\.\d+\.\d+)\.(\d+)/);
  return match ? `${RELEASE_PREFIX}${match[1]}+${match[2]}` : undefined;
}

function coreVersion(): string | undefined {
  try {
    return core().version();
  } catch {
    // The settings service reports its version once the engine is up; at launch it is not.
    return undefined;
  }
}

const ACCESS_TOKEN = /access_token=[^&\s'"]+/g;
const JWT = /eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/g;
const WALLET = /(0x[0-9a-fA-F]{4})[0-9a-fA-F]{36}/g;

/**
 * Event text must not carry secrets or wallets off the device: LiveKit room tokens ride in
 * `?access_token=`, and a wallet identifies the player.
 */
export function scrubText(text: string): string {
  return text
    .replace(ACCESS_TOKEN, 'access_token=[redacted]')
    .replace(JWT, '[jwt]')
    .replace(WALLET, '$1…');
}

/** The message and every exception value, where core reports carry the logged text. */
export function scrubEvent<E extends Sentry.Event>(event: E): E {
  if (event.message !== undefined) {
    event.message = scrubText(event.message);
  }
  for (const exception of event.exception?.values ?? []) {
    if (exception.value !== undefined) {
      exception.value = scrubText(exception.value);
    }
  }
  return event;
}

/** Starts the one crash reporter, the owner of the native crash handlers. Call it first. */
export function initSentry() {
  const environment = ENVIRONMENTS[core().environment()];
  Sentry.init({
    dsn: SENTRY_DSN,
    environment,
    release: releaseFor(coreVersion()),
    // Development builds report nothing, as before; neither do debug (Metro) builds, whose mock core
    // says `org` (the visual-snapshot runs would report into production).
    enabled: environment !== 'development' && !__DEV__,
    sendDefaultPii: false,
    beforeSend: scrubEvent,
    // Tracing off: no tracesSampleRate.
  });
}

function toRecord(tags: ReportTag[]): Record<string, string> {
  return Object.fromEntries(tags.map(({ key, value }) => [key, value]));
}

/**
 * Sends a report the core's policy kept, with its tags and the `graphics` context. The core's
 * `attachLog` sample is not honoured: the shell has no way to read godot.log, so the
 * `attach_log_sampled` tag is the only trace of it.
 */
export function captureCoreReport(payload: ReportPayload) {
  const { event, tags } = payload;
  Sentry.withScope(scope => {
    scope.setTags(toRecord(tags.tags));
    scope.setContext('graphics', toRecord(tags.graphics));
    scope.setLevel(LEVELS[event.level]);
    if (event.exception !== undefined) {
      Sentry.captureException(new Error(event.exception));
    } else {
      Sentry.captureMessage(event.message, LEVELS[event.level]);
    }
  });
}

/** Mirrors the core's report tags (e.g. the resource being loaded) onto the scope. */
export function applyReportTag(key: string, value: string | undefined) {
  Sentry.setTag(key, value ?? CLEARED_TAG);
}

/** Routes the core's reports and tag changes to Sentry. Returns the unsubscribe function. */
export function startCoreReports(): () => void {
  return onCoreEvents({ report: captureCoreReport, reportTag: applyReportTag });
}
