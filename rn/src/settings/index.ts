import { registerScreen } from '../app/registry';
import { SettingsTab } from './SettingsTab';

registerScreen('settings', SettingsTab);

export { SettingsScreen } from './SettingsScreen';
export { getSettingsApi, setSettingsApi } from './api';
export type { SettingsApi } from './api';
export { createMockApi } from './mockApi';
