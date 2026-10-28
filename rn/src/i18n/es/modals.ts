import type { modals as en } from '../en/modals';

export const modals: Partial<Record<keyof typeof en, string>> = {
  NOTIFICATIONS_LIMITED_PERFORMANCE: 'Rendimiento limitado',
  AUTH_UPDATE_BODY:
    'Hay una nueva versión de Decentraland disponible. Actualiza para disfrutar de la mejor experiencia.',
  AUTH_UPDATE_TITLE: 'Nueva versión disponible',
  COMMON_UPDATE: 'ACTUALIZAR',
  MODAL_BAN_KICKED_BODY: 'Ponte en contacto con soporte para más información.',
  MODAL_BAN_KICKED_PRIMARY: 'VOLVER A DESCUBRIR',
  MODAL_BAN_KICKED_TITLE: 'Te han baneado',
  MODAL_BAN_PRE_CHECK_BODY:
    'Estás baneado de esta escena.\nPonte en contacto con soporte para más información.',
  MODAL_BAN_PRE_CHECK_PRIMARY: 'VOLVER A DESCUBRIR',
  MODAL_BAN_PRE_CHECK_TITLE: 'No puedes entrar',
  MODAL_BELOW_MINSPEC_BODY:
    'Tu dispositivo está por debajo del mínimo recomendado. Puede que notes ralentizaciones, cierres inesperados o calentamiento mientras juegas.',
  MODAL_CONNECTION_LOST_BODY:
    'Comprueba tu conexión a internet e inténtalo de nuevo.',
  MODAL_CONNECTION_LOST_PRIMARY: 'REINTENTAR',
  MODAL_CONNECTION_LOST_SECONDARY: 'SALIR',
  MODAL_CONNECTION_LOST_TITLE: 'Conexión perdida',
  MODAL_END_OF_DEVICE_SUPPORT_BODY:
    'Decentraland ya no recibirá actualizaciones en este dispositivo. Puedes seguir usando la versión actual, pero no podrás reinstalarla si la desinstalas.',
  MODAL_END_OF_DEVICE_SUPPORT_PRIMARY: 'ACEPTAR',
  MODAL_EXTERNAL_LINK_BODY:
    'Estás a punto de visitar un sitio web externo. Asegúrate de que confías en él antes de continuar.',
  MODAL_EXTERNAL_LINK_PRIMARY: 'ABRIR ENLACE',
  MODAL_EXTERNAL_LINK_SECONDARY: 'CANCELAR',
  MODAL_EXTERNAL_LINK_TITLE: '¿Abrir enlace externo?',
  MODAL_INVALID_DESTINATION_BODY:
    'Puede que este lugar se haya eliminado o ya no esté disponible.',
  MODAL_INVALID_DESTINATION_PRIMARY: 'ACEPTAR',
  MODAL_INVALID_DESTINATION_TITLE: 'Lugar no encontrado',
  MODAL_LOW_MEMORY_BODY:
    'Puede que este lugar no funcione con fluidez en tu dispositivo y que se cierre de forma inesperada.',
  MODAL_LOW_MEMORY_PRIMARY: 'CONTINUAR',
  MODAL_LOW_MEMORY_SECONDARY: 'VOLVER A DESCUBRIR',
  MODAL_LOW_MEMORY_TITLE: 'Memoria insuficiente',
  MODAL_PRIVATE_WORLD_BODY: 'Solo pueden entrar las personas invitadas.',
  MODAL_PRIVATE_WORLD_PRIMARY: 'ACEPTAR',
  MODAL_PRIVATE_WORLD_TITLE: '{world} es privado',
  MODAL_SCENE_CRASH_BODY:
    'Esta escena ha dejado de funcionar. Recárgala o vuelve a Descubrir.',
  MODAL_SCENE_CRASH_PRIMARY: 'RECARGAR',
  MODAL_SCENE_CRASH_SECONDARY: 'ATRÁS',
  MODAL_SCENE_CRASH_TITLE: 'Error de la escena',
  MODAL_SESSION_ENDED_DUPLICATE_BODY:
    'Tu sesión ha terminado porque se ha iniciado sesión con tu cuenta desde otro lugar.',
  MODAL_SESSION_ENDED_DUPLICATE_TITLE: 'Sesión finalizada',
  MODAL_SESSION_ENDED_OTHER_BODY:
    'Te has desconectado del servidor. Inténtalo más tarde.',
  MODAL_SESSION_ENDED_OTHER_TITLE: 'Sin conexión',
  MODAL_SESSION_ENDED_RETRY_PRIMARY: 'RECONECTAR',
  MODAL_SESSION_ENDED_ROOM_CLOSED_BODY:
    'La sala en la que estabas se ha cerrado.',
  MODAL_SESSION_ENDED_ROOM_CLOSED_TITLE: 'Sala cerrada',
  MODAL_SESSION_ENDED_SECONDARY: 'SALIR',
  MODAL_SESSION_ENDED_SIGN_IN_PRIMARY: 'INICIAR SESIÓN',
  MODAL_WORLD_PASSWORD_BUSY:
    'No se pudo verificar esa contraseña ahora. Inténtalo en un momento.',
  MODAL_WORLD_PASSWORD_CONFIRM: 'ENTRAR',
  MODAL_WORLD_PASSWORD_PLACEHOLDER: 'Contraseña',
  MODAL_WORLD_PASSWORD_SUBTITLE:
    'Este World está protegido. Escribe su contraseña para entrar.',
  MODAL_WORLD_PASSWORD_TITLE: 'Se necesita contraseña',
  MODAL_WORLD_PASSWORD_TOO_MANY:
    'Demasiados intentos. Espera un minuto e inténtalo de nuevo.',
  MODAL_WORLD_PASSWORD_WRONG: 'Esa contraseña no funcionó. Inténtalo de nuevo.',
  TOAST_POOR_CONNECTION_BODY:
    'Tu conexión es inestable. Puede que algunas funciones no vayan bien.',
  TOAST_POOR_CONNECTION_TITLE: 'Conexión deficiente',
};
