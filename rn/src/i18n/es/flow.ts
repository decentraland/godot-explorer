import type { flow as en } from '../en/flow';

export const flow: Partial<Record<keyof typeof en, string>> = {
  AUTH_SIGN_IN_OTHER_REQUEST:
    'Este inicio de sesión se inició desde otra sesión. Vuelve a intentarlo.',
  FLOW_TERMS_DISCLAIMER:
    'Al continuar, aceptas los {tos}, la {privacy} y la {content} de Decentraland.',
  FLOW_TERMS_OF_SERVICE: 'Términos de Servicio',
  FLOW_PRIVACY_POLICY: 'Política de Privacidad',
  FLOW_CONTENT_POLICY: 'Política de Contenido',
  FLOW_ACCOUNT_TITLE: 'Te damos la bienvenida a Decentraland',
  FLOW_ACCOUNT_TAGLINE: 'Descubre un mundo creado\npor su comunidad',
  FLOW_ACCOUNT_SIGN_IN: 'INICIAR SESIÓN',
  FLOW_ACCOUNT_GUEST: 'JUGAR COMO INVITADO',
  FLOW_SIGN_IN_TITLE: 'Inicia sesión en Decentraland',
  FLOW_OR: 'o',
  FLOW_AVATAR_CHOOSE_LOOK: 'Elige tu look',
  FLOW_AVATAR_EDIT: 'EDITAR',
  FLOW_AVATAR_NEXT: 'SIGUIENTE',
};
