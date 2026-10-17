import type { ComponentType, ReactNode } from 'react';
import type { RouteName, RouteParams, SlotName } from './routes';

/** Props every registered screen receives; `footer` is the bottom nav, set for tab bodies. */
export interface ScreenProps<N extends RouteName = RouteName> {
  params: RouteParams[N];
  footer?: ReactNode;
}

type Entry = ComponentType<ScreenProps<never>>;
const screens = new Map<string, Entry>();

/** Feature packages call this at import time to plug a screen, sheet or slot into the shell. */
export function registerScreen<N extends RouteName | SlotName>(
  name: N,
  component: ComponentType<ScreenProps<N extends RouteName ? N : never>>,
): void {
  screens.set(name, component as Entry);
}

export function getScreen(name: RouteName | SlotName): Entry | undefined {
  return screens.get(name);
}

export function clearScreens(): void {
  screens.clear();
}
