import { Quality } from '../src/core/native';
import type { Report } from '../src/core/native';
import {
  CONNECTION_KEY,
  applyReport,
  connectionLostModal,
} from '../src/modals/useConnectionQuality';

const report = (patch: Partial<Report>): Report => ({
  quality: Quality.Good,
  toast: false,
  alarm: false,
  restored: false,
  nextPollMs: 10_000n,
  ...patch,
});

function actions() {
  return {
    toast: jest.fn(),
    show: jest.fn(),
    dismiss: jest.fn(),
    retry: jest.fn(),
    exit: jest.fn(),
  };
}

describe('connection reports', () => {
  it('warn with a toast, raise the dialog and take it away on recovery', () => {
    const a = actions();
    applyReport(report({ toast: true }), a);
    expect(a.toast).toHaveBeenCalledWith(
      expect.stringContaining('Poor connection'),
    );
    applyReport(report({ alarm: true, quality: Quality.Lost }), a);
    expect(a.show).toHaveBeenCalledTimes(1);
    expect(a.show.mock.calls[0][0].key).toBe(CONNECTION_KEY);
    applyReport(report({ restored: true }), a);
    expect(a.dismiss).toHaveBeenCalledWith(CONNECTION_KEY);
    applyReport(report({}), a);
    expect(a.show).toHaveBeenCalledTimes(1);
  });

  it('keep the dialog until the player answers: retry quiets it, exit leaves', () => {
    const a = actions();
    const modal = connectionLostModal(a);
    expect(modal.dismissable).toBe(false);
    modal.buttons[1].onPress?.();
    expect(a.retry).toHaveBeenCalled();
    modal.buttons[0].onPress?.();
    expect(a.exit).toHaveBeenCalled();
  });
});
