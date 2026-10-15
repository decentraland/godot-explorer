import { NativeModules, Platform } from 'react-native';
import type { DeviceFacts } from '../core/native';

const UNKNOWN: DeviceFacts = {
  totalRamMb: -1,
  model: '',
  socModel: '',
  boardPlatform: '',
  hardware: '',
};

/** The hardware facts the core's device-support rule reads; unknown values where the platform has no answer yet. */
export async function deviceFacts(): Promise<DeviceFacts> {
  const native = Platform.OS === 'android' ? NativeModules.DclPush : undefined;
  if (!native?.deviceFacts) {
    return UNKNOWN;
  }
  try {
    return { ...UNKNOWN, ...(await native.deviceFacts()) };
  } catch {
    return UNKNOWN;
  }
}
