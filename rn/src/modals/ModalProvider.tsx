import {
  createContext,
  useEffect,
  ReactNode,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
} from 'react';
import { Dialog, TextField } from '../ds';
import type { DialogButton } from '../ds';
import { t } from '../i18n';
import { registerModalApi } from './bridge';

export type ModalButton = {
  label: string;
  tone?: DialogButton['tone'];
  /** Runs before the modal closes. */
  onPress?: () => void;
};

export type ModalSpec = {
  /** A modal with the same key replaces the queued or shown one instead of stacking. */
  key: string;
  title: string;
  body?: string;
  buttons: ModalButton[];
  /** Back and outside taps close it; off for the ones the player has to answer. */
  dismissable?: boolean;
  onDismiss?: () => void;
};

/** What the password dialog needs: `verify` returns the error text, or null when the password worked. */
export type PasswordSpec = {
  title: string;
  subtitle: string;
  verify: (password: string) => Promise<string | null>;
  onCancel: () => void;
  onAccepted: () => void;
};

export type ModalApi = {
  show: (spec: ModalSpec) => void;
  dismiss: (key: string) => void;
  askPassword: (spec: PasswordSpec) => void;
};

const ModalContext = createContext<ModalApi | null>(null);

export function useModals(): ModalApi {
  const api = useContext(ModalContext);
  if (!api) {
    throw new Error('useModals needs a ModalProvider');
  }
  return api;
}

/** Modals one at a time in the order they were asked for. */
export function queueAfter(queue: ModalSpec[], spec: ModalSpec): ModalSpec[] {
  const at = queue.findIndex(item => item.key === spec.key);
  if (at === 0) {
    return [spec, ...queue.slice(1)];
  }
  return [...queue.filter(item => item.key !== spec.key), spec];
}

function PasswordDialog({ spec }: { spec: PasswordSpec }) {
  const [value, setValue] = useState('');
  const [error, setError] = useState<string | undefined>();
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    if (busy || !value) {
      return;
    }
    setBusy(true);
    const failure = await spec
      .verify(value)
      .catch(() => t('MODAL_WORLD_PASSWORD_BUSY'));
    setBusy(false);
    if (failure === null) {
      spec.onAccepted();
    } else {
      setError(failure);
    }
  };
  return (
    <Dialog
      visible
      title={spec.title}
      body={spec.subtitle}
      onRequestClose={spec.onCancel}
      buttons={[
        {
          label: t('COMMON_CANCEL'),
          tone: 'secondary',
          onPress: spec.onCancel,
        },
        {
          label: t('MODAL_WORLD_PASSWORD_CONFIRM'),
          onPress: submit,
          disabled: busy || !value,
        },
      ]}
    >
      <TextField
        label={t('MODAL_WORLD_PASSWORD_PLACEHOLDER')}
        placeholder={t('MODAL_WORLD_PASSWORD_PLACEHOLDER')}
        value={value}
        onChangeText={text => {
          setValue(text);
          setError(undefined);
        }}
        error={error}
        autoCapitalize="none"
        secureTextEntry
        autoFocus
      />
    </Dialog>
  );
}

export function ModalProvider({ children }: { children: ReactNode }) {
  const [queue, setQueue] = useState<ModalSpec[]>([]);
  const [password, setPassword] = useState<PasswordSpec | null>(null);
  const current = useRef<ModalSpec | undefined>(undefined);
  current.current = queue[0];

  const show = useCallback(
    (spec: ModalSpec) => setQueue(q => queueAfter(q, spec)),
    [],
  );
  const dismiss = useCallback(
    (key: string) => setQueue(q => q.filter(item => item.key !== key)),
    [],
  );
  const askPassword = useCallback(
    (spec: PasswordSpec) => setPassword(spec),
    [],
  );
  const api = useMemo(
    () => ({ show, dismiss, askPassword }),
    [show, dismiss, askPassword],
  );
  useEffect(() => {
    registerModalApi(api);
    return () => registerModalApi(null);
  }, [api]);

  const close = () => setQueue(q => q.slice(1));
  const shown = queue[0];
  return (
    <ModalContext.Provider value={api}>
      {children}
      {shown && !password ? (
        <Dialog
          visible
          title={shown.title}
          body={shown.body}
          onRequestClose={() => {
            if (shown.dismissable !== false) {
              shown.onDismiss?.();
              close();
            }
          }}
          buttons={shown.buttons.map(button => ({
            label: button.label,
            tone: button.tone,
            onPress: () => {
              close();
              button.onPress?.();
            },
          }))}
        />
      ) : null}
      {password ? (
        <PasswordDialog
          spec={{
            ...password,
            onCancel: () => {
              setPassword(null);
              password.onCancel();
            },
            onAccepted: () => {
              setPassword(null);
              password.onAccepted();
            },
          }}
        />
      ) : null}
    </ModalContext.Provider>
  );
}
