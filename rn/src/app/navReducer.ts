import type { Route, Tab } from './routes';

export interface NavState {
  stack: Route[];
  tab: Tab;
}

export type NavAction =
  | { type: 'push'; route: Route }
  | { type: 'pop' }
  | { type: 'replace'; route: Route }
  | { type: 'reset'; route: Route }
  | { type: 'setTab'; tab: Tab };

export const initialNavState = (): NavState => ({
  stack: [{ name: 'home' }],
  tab: 'explore',
});

export function navReducer(state: NavState, action: NavAction): NavState {
  switch (action.type) {
    case 'push':
      return { ...state, stack: [...state.stack, action.route] };
    case 'pop':
      return state.stack.length > 1
        ? { ...state, stack: state.stack.slice(0, -1) }
        : state;
    case 'replace':
      return { ...state, stack: [...state.stack.slice(0, -1), action.route] };
    case 'reset':
      return { ...state, stack: [action.route] };
    case 'setTab':
      return state.tab === action.tab ? state : { ...state, tab: action.tab };
  }
}

/** What the hardware back button does now, or null to let the OS leave the app. */
export function backAction(state: NavState): NavAction | null {
  if (state.stack.length > 1) {
    return { type: 'pop' };
  }
  return state.tab !== 'explore' ? { type: 'setTab', tab: 'explore' } : null;
}

/** The world is opaque, so everything below the last one is not mounted. */
export function visibleRoutes(state: NavState): Route[] {
  let start = 0;
  state.stack.forEach((route, i) => {
    if (route.name === 'world') {
      start = i;
    }
  });
  return state.stack.slice(start);
}
