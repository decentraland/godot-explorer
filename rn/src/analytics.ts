import { core } from './core';
import { PropertyValue } from './core/native';
import type { Property } from './core/native';

/** Seconds between periodic flushes: short while onboarding, longer once the app is open. */
export const FLUSH_ONBOARDING_S = 2;
export const FLUSH_APP_S = 10;

export type EventProps = Record<string, string | number | boolean>;

function valueOf(value: string | number | boolean): PropertyValue {
  if (typeof value === 'string') {
    return new PropertyValue.Text({ value });
  }
  if (typeof value === 'boolean') {
    return new PropertyValue.Bool({ value });
  }
  return Number.isInteger(value)
    ? new PropertyValue.Int({ value: BigInt(value) })
    : new PropertyValue.Float({ value });
}

const toProperties = (props: EventProps = {}): Property[] =>
  Object.entries(props).map(([key, value]) => ({
    key,
    value: valueOf(value),
  }));

/** Queues `name` with `props`; the core ships it once the terms are accepted. */
export function trackEvent(name: string, props?: EventProps): void {
  core().track(name, toProperties(props));
}

/** Queues a "Screen Viewed" for `name`. */
export function trackScreen(name: string, props?: EventProps): void {
  core().trackScreen(name, toProperties(props));
}
