import { trackClick, trackEvent, trackScreen } from '../src/analytics';
import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { PropertyValue } from '../src/core/native';

const track = jest.fn();
const trackScreenCall = jest.fn();
const trackClickCall = jest.fn();

beforeEach(() => {
  track.mockClear();
  trackScreenCall.mockClear();
  trackClickCall.mockClear();
  setCoreForTests({
    ...createMockCore(),
    track,
    trackScreen: trackScreenCall,
    trackClick: trackClickCall,
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

  it('keeps a null as a null and sends clicks with their button and screen', () => {
    trackClick('JUMP_IN', 'PLACE_DETAIL_CLICK', { position: null });
    expect(trackClickCall).toHaveBeenCalledWith(
      'JUMP_IN',
      'PLACE_DETAIL_CLICK',
      [{ key: 'position', value: new PropertyValue.Null() }],
    );
  });

  it('sends none for a call without properties', () => {
    trackScreen('Home');
    expect(trackScreenCall).toHaveBeenCalledWith('Home', []);
  });
});
