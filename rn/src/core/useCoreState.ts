import { useEffect, useState } from 'react';
import { onCoreEvents } from './events';
import { core } from './index';
import type { State } from './native';

/** The app state, re-rendering on every change (the state machine is the only thing the shell
 * renders from). */
export function useCoreState(): State {
  const [state, setState] = useState<State>(() => core().state());
  useEffect(() => {
    setState(core().state());
    return onCoreEvents({ stateChanged: setState });
  }, []);
  return state;
}
