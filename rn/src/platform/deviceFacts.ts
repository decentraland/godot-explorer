import { NativeModules, Platform } from 'react-native';
import type { DeviceFacts } from '../core/native';

const UNKNOWN: DeviceFacts = {
  totalRamMb: -1,
  model: '',
  socModel: '',
  boardPlatform: '',
  hardware: '',
};

/** What a bug report adds to the hardware facts. */
export type DeviceDetails = DeviceFacts & { brand: string; osVersion: string };

/** The hardware facts the core's device-support rule reads, with the brand and OS version a bug
 * report names; unknown values where the platform has no answer yet. */
export async function deviceDetails(): Promise<DeviceDetails> {
  const unknown = { ...UNKNOWN, brand: '', osVersion: Platform.OS };
  const native = Platform.OS === 'android' ? NativeModules.DclPush : undefined;
  if (!native?.deviceFacts) {
    return unknown;
  }
  try {
    return { ...unknown, ...(await native.deviceFacts()) };
  } catch {
    return unknown;
  }
}

export async function deviceFacts(): Promise<DeviceFacts> {
  return deviceDetails();
}
