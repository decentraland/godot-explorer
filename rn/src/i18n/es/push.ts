import type { push as en } from '../en/push';

export const push: Partial<Record<keyof typeof en, string>> = {
  NOTIF_LOCAL_BODY_DONT_MISS_ACTION: '¡No te pierdas la acción!',
  NOTIF_LOCAL_BODY_DRESS_UP: 'Arréglate y sé el alma de la fiesta',
  NOTIF_LOCAL_BODY_GANG_AWAITS: '¡Tu gente te espera, entra y a disfrutar!',
  NOTIF_LOCAL_BODY_GATHER_YOUR_CREW: '¡Reúne a tu equipo y deja tu huella!',
  NOTIF_LOCAL_BODY_HOP_IN: '¡Entra y no te pierdas nada!',
  NOTIF_LOCAL_BODY_JOIN_YOUR_FRIENDS: '¡Únete a tus amigos y conoce gente!',
  NOTIF_LOCAL_BODY_MEET_YOUR_PEOPLE: 'Únete y conoce a tu gente',
  NOTIF_LOCAL_DAY1_BODY: 'Hay gente pasando el rato en Decentraland.',
  NOTIF_LOCAL_DAY1_TITLE: 'Ven a saludar 👋',
  NOTIF_LOCAL_EVENT_BEGINS: '¡{event} comienza!',
  NOTIF_LOCAL_EVENT_IS_LIVE: '¡{event} está en directo!',
  NOTIF_LOCAL_EVENT_IS_ON: '¡{event} ya está aquí!',
  NOTIF_LOCAL_EVENT_IS_STARTING: '¡{event} está empezando!',
  NOTIF_LOCAL_EVENT_STARTED: '¡{event} ha empezado!',
};
