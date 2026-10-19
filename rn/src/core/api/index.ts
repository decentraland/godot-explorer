import type { AvatarApi } from './avatar';
import type { ControlApi } from './control';
import type { FtueApi } from './ftue';
import type { SessionApi } from './session';
import type { SettingsApi } from './settings';
import type { ProfileApi } from './profile';
import type { PlacesApi } from './places';
import type { PushApi } from './push';
import type { GatesApi } from './gates';
import type { LifecycleApi } from './lifecycle';
import type { SceneApi } from './scene';

/** Everything the shell can ask of the core: one slice per domain. */
export type CoreApi = SessionApi &
  AvatarApi &
  ControlApi &
  FtueApi &
  SettingsApi &
  ProfileApi &
  PlacesApi &
  PushApi &
  GatesApi &
  LifecycleApi &
  SceneApi;

export type { SessionApi };
