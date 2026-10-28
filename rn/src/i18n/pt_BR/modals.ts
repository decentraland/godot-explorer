import type { modals as en } from '../en/modals';

export const modals: Partial<Record<keyof typeof en, string>> = {
  NOTIFICATIONS_LIMITED_PERFORMANCE: 'Desempenho limitado',
  AUTH_UPDATE_BODY:
    'Nova versão da Decentraland disponível. Atualize para a melhor experiência.',
  AUTH_UPDATE_TITLE: 'Nova versão liberada!',
  COMMON_UPDATE: 'ATUALIZAR',
  MODAL_BAN_KICKED_BODY: 'Contate o suporte para obter mais informações.',
  MODAL_BAN_KICKED_PRIMARY: 'IR PARA EXPLORAR',
  MODAL_BAN_KICKED_TITLE: 'Você foi banido(a)',
  MODAL_BAN_PRE_CHECK_BODY:
    'Você foi banido desta cena.\nPara saber mais, entre em contato com o suporte',
  MODAL_BAN_PRE_CHECK_PRIMARY: 'IR PARA EXPLORAR',
  MODAL_BAN_PRE_CHECK_TITLE: 'Acesso proibido',
  MODAL_BELOW_MINSPEC_BODY:
    'Seu dispositivo está abaixo do recomendado. Pode haver lentidão, travamentos ou aquecimento enquanto você joga.',
  MODAL_CONNECTION_LOST_BODY:
    'Confira sua conexão com a internet e tente outra vez',
  MODAL_CONNECTION_LOST_PRIMARY: 'REPETIR',
  MODAL_CONNECTION_LOST_SECONDARY: 'SAIR',
  MODAL_CONNECTION_LOST_TITLE: 'Conexão perdida',
  MODAL_END_OF_DEVICE_SUPPORT_BODY:
    'A Decentraland não vai mais receber atualizações neste dispositivo. Você pode continuar usando a versão atual, mas não poderá reinstalá-la se desinstalar o app.',
  MODAL_END_OF_DEVICE_SUPPORT_PRIMARY: 'OK',
  MODAL_EXTERNAL_LINK_BODY:
    'Você está prestes a abrir um site externo. Confirme que ele é confiável antes de continuar.',
  MODAL_EXTERNAL_LINK_PRIMARY: 'ABRA LINK',
  MODAL_EXTERNAL_LINK_SECONDARY: 'CANCELAR',
  MODAL_EXTERNAL_LINK_TITLE: 'Abrir link externo?',
  MODAL_INVALID_DESTINATION_BODY:
    'Este lugar pode ter sido removido ou não está mais disponível.',
  MODAL_INVALID_DESTINATION_PRIMARY: 'OK',
  MODAL_INVALID_DESTINATION_TITLE: 'Lugar não encontrado',
  MODAL_LOW_MEMORY_BODY:
    'Este lugar pode rodar mal no seu aparelho e acabar fechando inesperadamente.',
  MODAL_LOW_MEMORY_PRIMARY: 'CONTINUE',
  MODAL_LOW_MEMORY_SECONDARY: 'IR PARA EXPLORAR',
  MODAL_LOW_MEMORY_TITLE: 'Pouca memória',
  MODAL_PRIVATE_WORLD_BODY: 'Só os convidados podem entrar.',
  MODAL_PRIVATE_WORLD_PRIMARY: 'OK',
  MODAL_PRIVATE_WORLD_TITLE: '{world} é privado.',
  MODAL_SCENE_CRASH_BODY:
    'Esta cena parou de funcionar. Recarregue ou volte à aba Explorar.',
  MODAL_SCENE_CRASH_PRIMARY: 'RECARREGAR',
  MODAL_SCENE_CRASH_SECONDARY: 'VOLTAR',
  MODAL_SCENE_CRASH_TITLE: 'Erro na cena',
  MODAL_SESSION_ENDED_DUPLICATE_BODY:
    'Sua sessão terminou porque a sua conta foi acessada a partir de outro local.',
  MODAL_SESSION_ENDED_DUPLICATE_TITLE: 'Fim da sessão',
  MODAL_SESSION_ENDED_OTHER_BODY:
    'Você foi desconectado do servidor. Tente de novo em outro momento.',
  MODAL_SESSION_ENDED_OTHER_TITLE: 'Desconectado',
  MODAL_SESSION_ENDED_RETRY_PRIMARY: 'RECONECTAR',
  MODAL_SESSION_ENDED_ROOM_CLOSED_BODY: 'A sala em que você estava foi fechada',
  MODAL_SESSION_ENDED_ROOM_CLOSED_TITLE: 'Sala fechada',
  MODAL_SESSION_ENDED_SECONDARY: 'SAIR',
  MODAL_SESSION_ENDED_SIGN_IN_PRIMARY: 'ACESSAR',
  MODAL_WORLD_PASSWORD_BUSY:
    'Não foi possível verificar essa senha agora. Tente daqui a pouco.',
  MODAL_WORLD_PASSWORD_CONFIRM: 'ENTRAR',
  MODAL_WORLD_PASSWORD_PLACEHOLDER: 'Senha',
  MODAL_WORLD_PASSWORD_SUBTITLE:
    'Este World é protegido. Digite a senha para entrar.',
  MODAL_WORLD_PASSWORD_TITLE: 'Senha necessária',
  MODAL_WORLD_PASSWORD_TOO_MANY:
    'Muitas tentativas. Espere um minuto e tente novamente.',
  MODAL_WORLD_PASSWORD_WRONG: 'Essa senha não funcionou. Tente novamente.',
  TOAST_POOR_CONNECTION_BODY:
    'Sua conexão está instável. Alguns recursos podem não funcionar bem.',
  TOAST_POOR_CONNECTION_TITLE: 'Conexão precária',
};
