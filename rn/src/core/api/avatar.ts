import type * as Native from '../native';

/** The avatar rules the core owns: the wearable catalog the edits resolve against, what the
 * account owns, the fixed data the editor shows and saving the result. The edits themselves are
 * `control` commands. */
export type AvatarApi = Pick<
  typeof Native,
  | 'loadAvatarCatalog'
  | 'loadOwnedAvatarItems'
  | 'saveAvatar'
  | 'avatarPalette'
  | 'baseEmotes'
  | 'isRemovableCategory'
>;
