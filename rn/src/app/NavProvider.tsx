import {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useMemo,
  useReducer,
} from 'react';
import { BackHandler } from 'react-native';
import {
  backAction,
  initialNavState,
  navReducer,
  NavState,
} from './navReducer';
import type { Route, Tab } from './routes';

export interface Nav {
  state: NavState;
  /** Top of the stack. */
  route: Route;
  push: (route: Route) => void;
  pop: () => void;
  replace: (route: Route) => void;
  reset: (route: Route) => void;
  setTab: (tab: Tab) => void;
}

const NavContext = createContext<Nav | null>(null);

export function NavProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(navReducer, undefined, initialNavState);

  useEffect(() => {
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      const action = backAction(state);
      if (!action) {
        return false;
      }
      dispatch(action);
      return true;
    });
    return () => sub.remove();
  }, [state]);

  const nav = useMemo<Nav>(
    () => ({
      state,
      route: state.stack[state.stack.length - 1],
      push: route => dispatch({ type: 'push', route }),
      pop: () => dispatch({ type: 'pop' }),
      replace: route => dispatch({ type: 'replace', route }),
      reset: route => dispatch({ type: 'reset', route }),
      setTab: tab => dispatch({ type: 'setTab', tab }),
    }),
    [state],
  );
  return <NavContext.Provider value={nav}>{children}</NavContext.Provider>;
}

export function useNav(): Nav {
  const nav = useContext(NavContext);
  if (!nav) {
    throw new Error('useNav needs a NavProvider');
  }
  return nav;
}
