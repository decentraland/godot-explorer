import type * as Native from '../native';

/** The control slice of the generated bindings: worlds, the avatar preview and engine commands. */
export type ControlApi = Pick<
  typeof Native,
  | 'firstScreenDrawn'
  | 'enterWorld'
  | 'exitWorld'
  | 'showAvatarPreview'
  | 'hideAvatarPreview'
  | 'control'
  | 'avatarState'
>;
