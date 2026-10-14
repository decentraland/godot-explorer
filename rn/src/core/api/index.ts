import type { ControlApi } from './control';
import type { SessionApi } from './session';
import type { SettingsApi } from './settings';
import type { SocialApi } from './social';
import type { ProfileApi } from './profile';
import type { PlacesApi } from './places';
import type { GatesApi } from './gates';
import type { SceneApi } from './scene';

/** Everything the shell can ask of the core: one slice per domain. */
export type CoreApi = SessionApi &
  ControlApi &
  SettingsApi &
  SocialApi &
  ProfileApi &
  PlacesApi &
  GatesApi &
  SceneApi;

export type { SessionApi };
