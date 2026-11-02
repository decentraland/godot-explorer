import type { flow as en } from '../en/flow';

export const flow: Partial<Record<keyof typeof en, string>> = {
  AUTH_SIGN_IN_OTHER_REQUEST:
    'Este login foi iniciado em outra sessão. Tente novamente.',
};
