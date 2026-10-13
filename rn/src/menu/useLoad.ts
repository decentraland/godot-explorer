import { useCallback, useEffect, useRef, useState } from 'react';

export type Loaded<T> =
  | { status: 'loading' }
  | { status: 'error' }
  | { status: 'ready'; data: T };

/** Runs `load` on mount and on every `reload()`; a late answer to a superseded call is dropped. */
export function useLoad<T>(load: () => Promise<T>): {
  state: Loaded<T>;
  reload: () => void;
  /** Optimistic edit of the loaded data; ignored while loading or failed. */
  mutate: (edit: (data: T) => T) => void;
} {
  const [state, setState] = useState<Loaded<T>>({ status: 'loading' });
  const run = useRef(0);
  const loader = useRef(load);
  loader.current = load;

  const reload = useCallback(() => {
    const id = ++run.current;
    setState({ status: 'loading' });
    loader
      .current()
      .then(data => id === run.current && setState({ status: 'ready', data }))
      .catch(() => id === run.current && setState({ status: 'error' }));
  }, []);

  useEffect(() => {
    const token = run;
    reload();
    return () => {
      token.current++;
    };
  }, [reload]);

  const mutate = useCallback((edit: (data: T) => T) => {
    setState(prev =>
      prev.status === 'ready'
        ? { status: 'ready', data: edit(prev.data) }
        : prev,
    );
  }, []);

  return { state, reload, mutate };
}
