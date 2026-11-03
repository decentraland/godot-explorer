import type { flow as en } from '../en/flow';

export const flow: Partial<Record<keyof typeof en, string>> = {
  AUTH_SIGN_IN_OTHER_REQUEST:
    'Este login foi iniciado em outra sessão. Tente novamente.',
  FLOW_TERMS_DISCLAIMER:
    'Ao seguir, aceita os {tos}, a {privacy} e a {content} da Decentraland.',
  FLOW_TERMS_OF_SERVICE: 'Termos de Serviço',
  FLOW_PRIVACY_POLICY: 'Política de Privacidade',
  FLOW_CONTENT_POLICY: 'Política de Conteúdo',
  FLOW_ACCOUNT_TITLE: 'Bem-vindo à Decentraland',
  FLOW_ACCOUNT_TAGLINE: 'Descubra um mundo criado\npela comunidade',
  FLOW_ACCOUNT_SIGN_IN: 'ACESSAR',
  FLOW_ACCOUNT_GUEST: 'JOGAR COMO CONVIDADO',
  FLOW_SIGN_IN_TITLE: 'Entre na Decentraland',
  FLOW_OR: 'ou',
  FLOW_AVATAR_CHOOSE_LOOK: 'Escolha seu look',
  FLOW_AVATAR_EDIT: 'EDITAR',
  FLOW_AVATAR_NEXT: 'AVANÇAR',
  FLOW_NAME_TITLE: 'Escolha o nome',
  FLOW_NAME_HINT: 'Por segurança, não use seu nome real',
  FLOW_NAME_INVALID: 'Só letras, números e espaços',
  FLOW_NAME_RANDOMIZE: 'Nome aleatório',
  FLOW_LETS_GO: 'Vamos lá',
};
