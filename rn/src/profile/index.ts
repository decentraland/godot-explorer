import { registerScreen } from '../app/registry';
import { ProfileEditRoute, ProfileTab } from './ProfileScreens';

registerScreen('profile', ProfileTab);
registerScreen('profileEdit', ProfileEditRoute);

export { getProfileApi, mockApi, setProfileApi } from './api';
export type { ProfileApi } from './api';
export type { Profile, ProfileDeployPayload } from './types';
