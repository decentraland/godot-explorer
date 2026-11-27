/** The lobby's seed for the naming screen: a first and a last name off two short lists, never over
 * the field's 15 characters. */
const FIRST_NAMES = [
  'Zara',
  'Neo',
  'Luna',
  'Orion',
  'Kael',
  'Nova',
  'Jax',
  'Aria',
  'Cael',
  'Lyra',
  'Axel',
  'Seren',
  'Thane',
  'Elara',
  'Riven',
  'Eris',
  'Drake',
  'Iris',
  'Cyrus',
  'Vega',
  'Kai',
  'Aster',
  'Rune',
  'Talia',
  'Zane',
  'Echo',
  'Blaze',
  'Sage',
];
const LAST_NAMES = [
  'Skyforge',
  'Nightwing',
  'Ironheart',
  'Frostborn',
  'Dawnblade',
  'Voidblade',
  'Galewind',
  'Wolfbane',
  'Raveneye',
  'Firebrand',
  'Stargazer',
  'Nightsky',
  'Sunflare',
  'Starfall',
];

const pick = (list: readonly string[]) =>
  list[Math.floor(Math.random() * list.length)];

export const randomName = (): string =>
  `${pick(FIRST_NAMES)} ${pick(LAST_NAMES)}`;
