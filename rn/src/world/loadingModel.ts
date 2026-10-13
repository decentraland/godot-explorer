import type { IconName } from '../ds';
import type { I18nKey } from '../i18n';
import type { LoadingProgress } from '../hud/types';

export const SLOW_AFTER_MS = 90_000;
export const TIP_ROTATE_MS = 7000;

export type LoadingPhase = 'loading' | 'slow' | 'failed' | 'finished';

export type LoadingState = { phase: LoadingPhase; progress: LoadingProgress };

export const initialLoading: LoadingState = {
  phase: 'loading',
  progress: { loaded: 0, total: 0, mbps: 0, percent: 0 },
};

export type LoadingAction =
  | { type: 'progress'; data: LoadingProgress }
  | { type: 'finished' }
  | { type: 'failed' }
  | { type: 'slow' }
  | { type: 'dismiss' }
  | { type: 'restart' };

export function loadingReducer(
  state: LoadingState,
  action: LoadingAction,
): LoadingState {
  if (state.phase === 'finished' && action.type !== 'restart') {
    return state;
  }
  switch (action.type) {
    case 'progress':
      return { ...state, progress: action.data };
    case 'finished':
    case 'dismiss':
      return { ...state, phase: 'finished' };
    case 'failed':
      return { ...state, phase: 'failed' };
    case 'slow':
      return state.phase === 'loading' ? { ...state, phase: 'slow' } : state;
    case 'restart':
      return initialLoading;
  }
}

const pad2 = (n: number) => String(Math.max(0, Math.round(n))).padStart(2, '0');

export function progressLabel(p: LoadingProgress): {
  loaded: string;
  total: string;
  speed: string;
} {
  return {
    loaded: pad2(p.loaded),
    total: pad2(p.total),
    speed: p.mbps.toFixed(2),
  };
}

const clampPercent = (percent: number) => Math.min(100, Math.max(0, percent));

export const percentLabel = (percent: number): string =>
  `${Math.round(clampPercent(percent))}%`;

export const progressValue = (percent: number): number =>
  clampPercent(percent) / 100;

export type Tip = { id: string; icon: IconName; key: I18nKey };

export const TIPS: Tip[] = [
  { id: 'emotes', icon: 'dancer', key: 'TIP_EMOTES' },
  { id: 'friends', icon: 'friends', key: 'LOADING_TIP_FRIENDS' },
  { id: 'chat', icon: 'chat', key: 'LOADING_TIP_CHAT' },
  { id: 'explore', icon: 'bulb', key: 'TIP_CARD_EXPLORE' },
  { id: 'creator', icon: 'bulb', key: 'TIP_CREATOR_HUB' },
  { id: 'wearables', icon: 'bulb', key: 'TIP_WEARABLES' },
];

export const tipAt = (index: number): Tip =>
  TIPS[((index % TIPS.length) + TIPS.length) % TIPS.length];
