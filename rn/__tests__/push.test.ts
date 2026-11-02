import {
  PROMPT_COOLDOWN_MS,
  REMINDER_LEAD_SECONDS,
  askNotificationPermission,
  registerPush,
  reminderDelay,
  reminderText,
  syncDay1Notification,
  syncEventReminder,
} from '../src/notifications/push';
import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import type { CoreApi } from '../src/core';
import { Day1Notification, PushPlatform } from '../src/core/native';
import { NativeModules, Platform } from 'react-native';

const fake = () => ({
  getToken: jest.fn().mockResolvedValue('tok'),
  hasPermission: jest.fn().mockResolvedValue(false),
  requestPermission: jest.fn().mockResolvedValue(true),
  deviceInfo: jest
    .fn()
    .mockResolvedValue({
      appVersion: '2.0.0',
      locale: 'es_AR',
      timezone: 'UTC',
    }),
  schedule: jest.fn().mockResolvedValue(true),
  cancel: jest.fn().mockResolvedValue(true),
});

describe('reminders', () => {
  it('fire three minutes before the start and never in the past', () => {
    const now = 1_000_000;
    expect(reminderDelay(now + 10 * 60_000, now)).toBe(
      10 * 60 - REMINDER_LEAD_SECONDS,
    );
    expect(reminderDelay(now + 2 * 60_000, now)).toBeNull();
    expect(reminderDelay(now - 1, now)).toBeNull();
  });

  it('say the event name in the title and pick the pools by the random draw', () => {
    const first = reminderText('Party', () => 0);
    expect(first.title).toBe('Party is Live!');
    expect(first.body).toBe('Dress up and be the soul of the party');
    const last = reminderText('Party', () => 0.999);
    expect(last.title).toBe('Party Begins!');
    expect(last.body).toBe("Hop in and don't miss a beat!");
  });
});

describe('with the Android module', () => {
  const original = Platform.OS;
  let module: ReturnType<typeof fake>;
  beforeEach(() => {
    Platform.OS = 'android';
    module = fake();
    NativeModules.DclPush = module;
  });
  afterEach(() => {
    Platform.OS = original;
    delete NativeModules.DclPush;
    setCoreForTests(undefined);
  });

  it('registers the token, the permission and the device info with the core', async () => {
    const setPushRegistration = jest.fn();
    setCoreForTests({ ...createMockCore(), setPushRegistration } as CoreApi);
    await registerPush();
    expect(setPushRegistration).toHaveBeenCalledWith({
      platform: PushPlatform.Android,
      token: 'tok',
      permission: 'denied',
      appVersion: '2.0.0',
      locale: 'es_AR',
      timezone: 'UTC',
    });
  });

  it('asks for the permission at most once in the cooldown', async () => {
    const t0 = 10_000_000;
    expect(await askNotificationPermission(t0)).toBe(true);
    expect(await askNotificationPermission(t0 + 1000)).toBe(false);
    expect(module.requestPermission).toHaveBeenCalledTimes(1);
    expect(await askNotificationPermission(t0 + PROMPT_COOLDOWN_MS + 1)).toBe(
      true,
    );
    expect(module.requestPermission).toHaveBeenCalledTimes(2);
  });

  it('schedules the reminder of an event and cancels it when switched off', async () => {
    const now = 20_000_000;
    const event = { id: 'e1', title: 'Party', startsAt: now + 30 * 60_000 };
    await syncEventReminder(event, true, 'https://x/e1', now);
    expect(module.schedule).toHaveBeenCalledWith(
      'event-e1',
      expect.stringContaining('Party'),
      expect.any(String),
      30 * 60 - REMINDER_LEAD_SECONDS,
      'https://x/e1',
    );
    await syncEventReminder(event, false, 'https://x/e1', now);
    expect(module.cancel).toHaveBeenCalledWith('event-e1');
  });

  it('schedules nothing for an event that already started', async () => {
    const now = 20_000_000;
    await syncEventReminder(
      { id: 'e2', title: 'Old', startsAt: now - 1 },
      true,
      'l',
      now,
    );
    expect(module.schedule).not.toHaveBeenCalled();
  });

  it('does what the core decides for the Day 1 welcome, passing the permission', async () => {
    const day1Notification = jest
      .fn()
      .mockReturnValueOnce(
        new Day1Notification.Arm({
          id: 'day1_welcome',
          delaySeconds: 86_400,
          deepLink: 'decentraland://open?position=0,0',
        }),
      )
      .mockReturnValueOnce(new Day1Notification.Disarm({ id: 'day1_welcome' }))
      .mockReturnValueOnce(new Day1Notification.Keep());
    setCoreForTests({ ...createMockCore(), day1Notification } as CoreApi);
    module.hasPermission.mockResolvedValue(true);

    await syncDay1Notification();
    expect(day1Notification).toHaveBeenCalledWith(true);
    expect(module.schedule).toHaveBeenCalledWith(
      'day1_welcome',
      'Come and say hi 👋',
      'People are hanging out in Decentraland.',
      86_400,
      'decentraland://open?position=0,0',
    );
    await syncDay1Notification();
    expect(module.cancel).toHaveBeenCalledWith('day1_welcome');
    await syncDay1Notification();
    expect(module.schedule).toHaveBeenCalledTimes(1);
    expect(module.cancel).toHaveBeenCalledTimes(1);
  });
});

describe('without the module', () => {
  it('does nothing', async () => {
    delete NativeModules.DclPush;
    await expect(registerPush()).resolves.toBeUndefined();
    expect(await askNotificationPermission()).toBe(false);
  });
});
