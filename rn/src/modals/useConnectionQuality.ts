import { useEffect } from 'react';
import { core } from '../core';
import type { Report } from '../core/native';
import { t } from '../i18n';
import type { ModalApi, ModalSpec } from './ModalProvider';

export const CONNECTION_KEY = 'connection-lost';
/** Poll again this soon after a failed probe call (the core is not reachable at all). */
const FALLBACK_POLL_MS = 10_000;

/** The dialog for a lost connection: retry only quiets it for a while, exit leaves the world. */
export function connectionLostModal(actions: {
  retry: () => void;
  exit: () => void;
}): ModalSpec {
  return {
    key: CONNECTION_KEY,
    title: t('MODAL_CONNECTION_LOST_TITLE'),
    body: t('MODAL_CONNECTION_LOST_BODY'),
    dismissable: false,
    buttons: [
      {
        label: t('MODAL_CONNECTION_LOST_SECONDARY'),
        tone: 'secondary',
        onPress: actions.exit,
      },
      { label: t('MODAL_CONNECTION_LOST_PRIMARY'), onPress: actions.retry },
    ],
  };
}

/** What a probe's report asks the shell to do. */
export function applyReport(
  report: Report,
  actions: {
    toast: (text: string) => void;
    show: (spec: ModalSpec) => void;
    dismiss: (key: string) => void;
    retry: () => void;
    exit: () => void;
  },
): void {
  if (report.toast) {
    actions.toast(
      `${t('TOAST_POOR_CONNECTION_TITLE')}: ${t('TOAST_POOR_CONNECTION_BODY')}`,
    );
  }
  if (report.alarm) {
    actions.show(connectionLostModal(actions));
  }
  if (report.restored) {
    actions.dismiss(CONNECTION_KEY);
  }
}

/** Polls the connection while the player is in, on the interval each report asks for. */
export function useConnectionQuality(
  active: boolean,
  modals: ModalApi,
  toast: (text: string) => void,
  exit: () => void,
) {
  useEffect(() => {
    if (!active) {
      return;
    }
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      let wait = FALLBACK_POLL_MS;
      try {
        if (core().state().tag === 'Loading') {
          // A world change is in flight: its slow answers are not a connection problem.
          core().connectionReset();
        } else {
          const report = await core().probeConnection();
          wait = Number(report.nextPollMs);
          if (!stopped) {
            applyReport(report, {
              toast,
              show: modals.show,
              dismiss: modals.dismiss,
              retry: () => core().connectionRetry(),
              exit,
            });
          }
        }
      } catch {
        // The core could not run the probe; try again later.
      }
      if (!stopped) {
        timer = setTimeout(poll, wait);
      }
    };
    poll();
    return () => {
      stopped = true;
      if (timer) {
        clearTimeout(timer);
      }
    };
  }, [active, modals, toast, exit]);
}
