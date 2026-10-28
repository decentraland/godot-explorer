import type * as Native from '../native';

/** The avatar rules the core owns: the wearable catalog the edits resolve against and the fixed
 * data the editor shows. The edits themselves are `control` commands. */
export type AvatarApi = Pick<
  typeof Native,
  'loadAvatarCatalog' | 'avatarPalette' | 'baseEmotes' | 'isRemovableCategory'
>;
