import type { flow as en } from '../en/flow';

export const flow: Partial<Record<keyof typeof en, string>> = {
  AUTH_SIGN_IN_OTHER_REQUEST:
    'Este inicio de sesión se inició desde otra sesión. Vuelve a intentarlo.',
};
