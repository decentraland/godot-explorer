import { trackEvent, trackScreen } from '../src/analytics';
import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { PropertyValue } from '../src/core/native';

const track = jest.fn();
const trackScreenCall = jest.fn();

beforeEach(() => {
  track.mockClear();
  trackScreenCall.mockClear();
  setCoreForTests({
    ...createMockCore(),
    track,
    trackScreen: trackScreenCall,
  });
});
afterEach(() => setCoreForTests(undefined));

describe('analytics helpers', () => {
  it('turns a record into typed properties', () => {
    trackEvent('Clicked', { label: 'jump', count: 3, ratio: 0.5, on: true });
    expect(track).toHaveBeenCalledWith('Clicked', [
      { key: 'label', value: new PropertyValue.Text({ value: 'jump' }) },
      { key: 'count', value: new PropertyValue.Int({ value: 3n }) },
      { key: 'ratio', value: new PropertyValue.Float({ value: 0.5 }) },
      { key: 'on', value: new PropertyValue.Bool({ value: true }) },
    ]);
  });

  it('sends none for a call without properties', () => {
    trackScreen('Home');
    expect(trackScreenCall).toHaveBeenCalledWith('Home', []);
  });
});
