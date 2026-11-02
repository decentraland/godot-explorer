import {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  useRef,
} from 'react';
import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors } from './tokens';
import { u } from './scale';

export type ToastItem = { id: number; message: string; icon?: IconName };

type ToastAction =
  | { type: 'show'; toast: ToastItem }
  | { type: 'hide'; id: number };

/** One toast at a time: a new one replaces the previous. */
export function toastReducer(
  state: ToastItem[],
  action: ToastAction,
): ToastItem[] {
  if (action.type === 'show') {
    return [action.toast];
  }
  return state.some(t => t.id === action.id)
    ? state.filter(t => t.id !== action.id)
    : state;
}

export function ToastBubble({
  message,
  icon,
}: {
  message: string;
  icon?: IconName;
}) {
  return (
    <View
      style={styles.bubble}
      accessibilityRole="alert"
      accessibilityLiveRegion="polite"
    >
      {icon && <Icon name={icon} size={16} color={colors.teal} />}
      <AppText variant="caption" style={styles.text}>
        {message}
      </AppText>
    </View>
  );
}

type Show = (
  message: string,
  options?: { icon?: IconName; duration?: number },
) => void;

const ToastContext = createContext<Show>(() => {});

export const useToast = (): Show => useContext(ToastContext);

const DEFAULT_DURATION = 2500;

/** Hosts toasts above `children`; call `useToast()(message)` from anywhere below. */
export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, dispatch] = useReducer(toastReducer, []);
  const nextId = useRef(1);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => () => clearTimeout(timer.current), []);

  const show = useCallback<Show>((message, options) => {
    const id = nextId.current++;
    clearTimeout(timer.current);
    dispatch({ type: 'show', toast: { id, message, icon: options?.icon } });
    timer.current = setTimeout(
      () => dispatch({ type: 'hide', id }),
      options?.duration ?? DEFAULT_DURATION,
    );
  }, []);

  const value = useMemo(() => show, [show]);
  return (
    <ToastContext.Provider value={value}>
      {children}
      <View pointerEvents="none" style={styles.host}>
        {toasts.map(t => (
          <ToastBubble key={t.id} message={t.message} icon={t.icon} />
        ))}
      </View>
    </ToastContext.Provider>
  );
}

const styles = StyleSheet.create({
  host: {
    position: 'absolute',
    left: u(24),
    right: u(24),
    bottom: u(100),
    alignItems: 'center',
  },
  bubble: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: u(16),
    paddingVertical: u(10),
    borderRadius: u(20),
    backgroundColor: colors.shadow,
    gap: u(8),
  },
  text: { color: colors.snow, fontSize: u(13) },
});
