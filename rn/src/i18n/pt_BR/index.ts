import { backpack } from './backpack';
import { common } from './common';
import { discover } from './discover';
import { engine } from './engine';
import { flow } from './flow';
import { ftue } from './ftue';
import { hud } from './hud';
import { menu } from './menu';
import { modals } from './modals';
import { profile } from './profile';
import { push } from './push';
import { settings } from './settings';

export const table: Record<string, string> = {
  ...backpack,
  ...common,
  ...discover,
  ...engine,
  ...flow,
  ...ftue,
  ...hud,
  ...menu,
  ...modals,
  ...profile,
  ...push,
  ...settings,
};
