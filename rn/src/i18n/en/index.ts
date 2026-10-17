import { backpack } from './backpack';
import { common } from './common';
import { discover } from './discover';
import { flow } from './flow';
import { hud } from './hud';
import { menu } from './menu';
import { profile } from './profile';
import { settings } from './settings';

/** One table per area, merged here; add a file and spread it in to start a new area. */
export const en = {
  ...backpack,
  ...common,
  ...discover,
  ...flow,
  ...hud,
  ...menu,
  ...profile,
  ...settings,
};
