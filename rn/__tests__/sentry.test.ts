import * as Sentry from '@sentry/react-native';
import { setCoreForTests } from '../src/core';
import { resetCoreEventsForTests } from '../src/core/events';
import { createMockCore } from '../src/core/mock';
import {
  Environment,
  ReportEvent,
  ReportLevel,
  ReportSource,
  type AppListener,
  type ReportPayload,
} from '../src/core/native';
import {
  applyReportTag,
  captureCoreReport,
  CLEARED_TAG,
  initSentry,
  releaseFor,
  SENTRY_DSN,
  startCoreReports,
} from '../src/sentry';

jest.mock('@sentry/react-native', () => {
  const scope = {
    setTags: jest.fn(),
    setContext: jest.fn(),
    setLevel: jest.fn(),
  };
  return {
    __scope: scope,
    init: jest.fn(),
    setTag: jest.fn(),
    captureException: jest.fn(),
    captureMessage: jest.fn(),
    withScope: jest.fn((run: (s: typeof scope) => void) => run(scope)),
  };
});

const mocked = Sentry as unknown as jest.Mocked<typeof Sentry> & {
  __scope: { setTags: jest.Mock; setContext: jest.Mock; setLevel: jest.Mock };
};

function payload(overrides: Partial<ReportEvent>): ReportPayload {
  return {
    event: {
      source: undefined,
      level: ReportLevel.Error,
      message: '',
      isCrash: false,
      eventKind: undefined,
      category: undefined,
      exception: undefined,
      ...overrides,
    },
    source: ReportSource.RustApp,
    attachLog: true,
    tags: {
      tags: [
        { key: 'source', value: 'rust_app' },
        { key: 'platform', value: 'Android' },
      ],
      graphics: [
        { key: 'name', value: 'Mali-G57' },
        { key: 'vendor', value: 'ARM' },
      ],
    },
  };
}

beforeEach(() => {
  jest.clearAllMocks();
  setCoreForTests(createMockCore());
  resetCoreEventsForTests();
});

afterEach(() => setCoreForTests(undefined));

describe('initSentry', () => {
  it('starts the SDK with the public key, the core environment and no PII or tracing', () => {
    initSentry();
    expect(mocked.init).toHaveBeenCalledTimes(1);
    const options = mocked.init.mock.calls[0][0] as Record<string, unknown>;
    expect(options.dsn).toBe(SENTRY_DSN);
    expect(options.environment).toBe('production');
    expect(options.enabled).toBe(true);
    expect(options.sendDefaultPii).toBe(false);
    expect(options).not.toHaveProperty('tracesSampleRate');
  });

  it('maps the staging environment and reports nothing from development builds', () => {
    const base = createMockCore();
    setCoreForTests({ ...base, environment: () => Environment.Zone });
    initSentry();
    expect(mocked.init.mock.calls[0][0]).toMatchObject({
      environment: 'staging',
      enabled: true,
    });

    setCoreForTests({ ...base, environment: () => Environment.Today });
    initSentry();
    expect(mocked.init.mock.calls[1][0]).toMatchObject({
      environment: 'development',
      enabled: false,
    });
  });

  it('names the release from the core version when it carries a build', () => {
    const base = createMockCore();
    setCoreForTests({ ...base, version: () => '1.15.0.1844-6cefb54-staging' });
    initSentry();
    expect(mocked.init.mock.calls[0][0]).toMatchObject({
      release: 'org.decentraland.godotexplorer@1.15.0+1844',
    });
  });

  it('leaves the release to the SDK when the version has no build or is not available', () => {
    expect(releaseFor('1.15.0')).toBeUndefined();
    expect(releaseFor('1.14.0-mock')).toBeUndefined();
    expect(releaseFor(undefined)).toBeUndefined();

    const base = createMockCore();
    setCoreForTests({
      ...base,
      version: () => {
        throw new Error('engine unavailable');
      },
    });
    initSentry();
    expect(mocked.init.mock.calls[0][0]).toMatchObject({ release: undefined });
  });
});

describe('captureCoreReport', () => {
  it('sends a logged error as an exception with the tags and the graphics context', () => {
    captureCoreReport(payload({ exception: '[Rust:dclgodot] boom (a.rs:1)' }));
    expect(mocked.__scope.setTags).toHaveBeenCalledWith({
      source: 'rust_app',
      platform: 'Android',
    });
    expect(mocked.__scope.setContext).toHaveBeenCalledWith('graphics', {
      name: 'Mali-G57',
      vendor: 'ARM',
    });
    expect(mocked.__scope.setLevel).toHaveBeenCalledWith('error');
    expect(mocked.captureException).toHaveBeenCalledTimes(1);
    expect((mocked.captureException.mock.calls[0][0] as Error).message).toBe(
      '[Rust:dclgodot] boom (a.rs:1)',
    );
    expect(mocked.captureMessage).not.toHaveBeenCalled();
  });

  it('sends an event without an exception as a message at its level', () => {
    captureCoreReport(
      payload({ level: ReportLevel.Fatal, message: 'panicked at x' }),
    );
    expect(mocked.captureMessage).toHaveBeenCalledWith(
      'panicked at x',
      'fatal',
    );
    expect(mocked.__scope.setLevel).toHaveBeenCalledWith('fatal');
    expect(mocked.captureException).not.toHaveBeenCalled();
  });

  it('maps every level', () => {
    const seen = [
      ReportLevel.Debug,
      ReportLevel.Info,
      ReportLevel.Warning,
      ReportLevel.Error,
      ReportLevel.Fatal,
    ].map(level => {
      mocked.__scope.setLevel.mockClear();
      captureCoreReport(payload({ level, message: 'm' }));
      return mocked.__scope.setLevel.mock.calls[0][0];
    });
    expect(seen).toEqual(['debug', 'info', 'warning', 'error', 'fatal']);
  });
});

describe('applyReportTag', () => {
  it('sets the tag, and marks it cleared instead of sending undefined to the native scope', () => {
    applyReportTag('loading_resource', 'res://a.scn');
    applyReportTag('loading_resource', undefined);
    expect(mocked.setTag.mock.calls).toEqual([
      ['loading_resource', 'res://a.scn'],
      ['loading_resource', CLEARED_TAG],
    ]);
  });
});

describe('startCoreReports', () => {
  it('sends the core reports and applies its tag changes through the one native listener', () => {
    let listener: AppListener | undefined;
    setCoreForTests({
      ...createMockCore(),
      addListener: l => {
        listener = l;
        return undefined as never;
      },
    });
    const stop = startCoreReports();

    listener?.report(payload({ message: 'panicked' }));
    listener?.reportTag('loading_resource', 'res://b.scn');
    expect(mocked.captureMessage).toHaveBeenCalledTimes(1);
    expect(mocked.setTag).toHaveBeenCalledWith(
      'loading_resource',
      'res://b.scn',
    );

    stop();
    listener?.report(payload({ message: 'after stop' }));
    expect(mocked.captureMessage).toHaveBeenCalledTimes(1);
  });
});
