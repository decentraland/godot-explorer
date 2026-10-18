import { NativeModules, Platform } from 'react-native';
import { core } from '../core';
import { Day1Notification_Tags, PushPlatform } from '../core/native';
import { t } from '../i18n';
import type { I18nKey } from '../i18n';

/** What the OS side (the `DclPush` module on Android) offers; absent without the engine build. */
type PushNative = {
  getToken(): Promise<string>;
  hasPermission(): Promise<boolean>;
  requestPermission(): Promise<boolean>;
  deviceInfo(): Promise<{
    appVersion: string;
    locale: string;
    timezone: string;
  }>;
  schedule(
    id: string,
    title: string,
    body: string,
    delaySeconds: number,
    deepLink: string,
  ): Promise<boolean>;
  cancel(id: string): Promise<boolean>;
};

const native = (): PushNative | undefined =>
  Platform.OS === 'android'
    ? (NativeModules.DclPush as PushNative | undefined)
    : undefined;

/** Reads the token and the permission and registers them with the core, which decides when the
 * identify goes out (after the `push-enabled` flag, once per token and permission). */
export async function registerPush(): Promise<void> {
  const push = native();
  if (!push) {
    return;
  }
  const [token, granted, info] = await Promise.all([
    push.getToken(),
    push.hasPermission(),
    push.deviceInfo(),
  ]);
  core().setPushRegistration({
    platform: PushPlatform.Android,
    token,
    permission: granted ? 'granted' : 'denied',
    appVersion: info.appVersion,
    locale: info.locale,
    timezone: info.timezone,
  });
}

/** Cooldown between two permission prompts in one run; the OS limits the rest. */
export const PROMPT_COOLDOWN_MS = 5 * 60 * 1000;
let lastPrompt = 0;

/** Asks for the notification permission unless it is granted or was asked for a moment ago. */
export async function askNotificationPermission(
  now: number = Date.now(),
): Promise<boolean> {
  const push = native();
  if (!push) {
    return false;
  }
  if (await push.hasPermission()) {
    return true;
  }
  if (now - lastPrompt < PROMPT_COOLDOWN_MS) {
    return false;
  }
  lastPrompt = now;
  return push.requestPermission();
}

/** A reminder goes off this long before the event starts. */
export const REMINDER_LEAD_SECONDS = 3 * 60;

const TITLES: I18nKey[] = [
  'NOTIF_LOCAL_EVENT_IS_LIVE',
  'NOTIF_LOCAL_EVENT_STARTED',
  'NOTIF_LOCAL_EVENT_IS_STARTING',
  'NOTIF_LOCAL_EVENT_IS_ON',
  'NOTIF_LOCAL_EVENT_BEGINS',
];
const BODIES: I18nKey[] = [
  'NOTIF_LOCAL_BODY_DRESS_UP',
  'NOTIF_LOCAL_BODY_GANG_AWAITS',
  'NOTIF_LOCAL_BODY_MEET_YOUR_PEOPLE',
  'NOTIF_LOCAL_BODY_DONT_MISS_ACTION',
  'NOTIF_LOCAL_BODY_GATHER_YOUR_CREW',
  'NOTIF_LOCAL_BODY_JOIN_YOUR_FRIENDS',
  'NOTIF_LOCAL_BODY_HOP_IN',
];

/** Seconds from `now` to the reminder, or null when it would not be in the future. */
export function reminderDelay(startsAt: number, now: number): number | null {
  const delay = Math.floor((startsAt - now) / 1000) - REMINDER_LEAD_SECONDS;
  return delay > 0 ? delay : null;
}

/** The OS notification is built at schedule time and cannot translate itself later. */
export function reminderText(
  eventTitle: string,
  random: () => number = Math.random,
): { title: string; body: string } {
  const pick = <T>(items: T[]) => items[Math.floor(random() * items.length)];
  return {
    title: t(pick(TITLES), { event: eventTitle }),
    body: t(pick(BODIES)),
  };
}

const reminderId = (eventId: string) => `event-${eventId}`;

/** Schedules (or cancels) the OS reminder of an event whose reminder was just switched. */
export async function syncEventReminder(
  event: { id: string; title: string; startsAt: number },
  on: boolean,
  link: string,
  now: number = Date.now(),
): Promise<void> {
  const push = native();
  if (!push) {
    return;
  }
  if (!on) {
    await push.cancel(reminderId(event.id));
    return;
  }
  const delay = reminderDelay(event.startsAt, now);
  if (delay === null) {
    return;
  }
  await askNotificationPermission(now);
  const { title, body } = reminderText(event.title);
  await push.schedule(reminderId(event.id), title, body, delay, link);
}

/** Arms, re-arms or disarms the Day 1 welcome as the core decides. Called whenever the permission
 * or the flags may have changed. */
export async function syncDay1Notification(): Promise<void> {
  const push = native();
  if (!push) {
    return;
  }
  const action = core().day1Notification(await push.hasPermission());
  if (action.tag === Day1Notification_Tags.Arm) {
    const { id, delaySeconds, deepLink } = action.inner;
    await push.schedule(
      id,
      t('NOTIF_LOCAL_DAY1_TITLE'),
      t('NOTIF_LOCAL_DAY1_BODY'),
      delaySeconds,
      deepLink,
    );
  } else if (action.tag === Day1Notification_Tags.Disarm) {
    await push.cancel(action.inner.id);
  }
}
