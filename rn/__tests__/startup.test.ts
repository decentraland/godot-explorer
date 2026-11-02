import { setCoreForTests } from '../src/core';
import { resetCoreEventsForTests } from '../src/core/events';
import { createMockCore } from '../src/core/mock';

jest.mock('../src/sentry', () => ({ startCoreReports: jest.fn() }));
jest.mock('../src/flow/browser', () => ({ startLinkHandling: jest.fn() }));

beforeEach(() => {
  jest.resetModules();
  setCoreForTests(createMockCore());
  resetCoreEventsForTests();
});

it('wires the reports and the links once, however often the app mounts', () => {
  jest.isolateModules(() => {
    const { startApp } = require('../src/app/startup');
    const { startCoreReports } = require('../src/sentry');
    const { startLinkHandling } = require('../src/flow/browser');
    startApp();
    startApp();
    expect(startCoreReports).toHaveBeenCalledTimes(1);
    expect(startLinkHandling).toHaveBeenCalledTimes(1);
  });
});
