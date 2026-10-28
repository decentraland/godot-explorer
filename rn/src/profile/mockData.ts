import type { EquippedItem, Profile } from './types';

export const MOCK_EQUIPPED: EquippedItem[] = [
  { id: 'e1', name: 'Asset Name', rarity: 'base', isEmote: false },
  { id: 'e2', name: 'Asset Name', rarity: 'legendary', isEmote: false },
  { id: 'e3', name: 'Asset Name', rarity: 'unique', isEmote: false },
  { id: 'e4', name: 'Asset Name', rarity: 'common', isEmote: true },
  { id: 'e5', name: 'Asset Name', rarity: 'legendary', isEmote: true },
  { id: 'e6', name: 'Asset Name', rarity: 'exotic', isEmote: true },
];

export const MOCK_PROFILE: Profile = {
  address: '0x4e1a5c5eb2A8a7aA7B0a09E04f1b2a1c13B',
  name: 'Namemaxcharacte',
  tag: '1234',
  hasClaimedName: false,
  description:
    "Visit my personal web. I love building and making friends in Decentraland. When I'm not decorating LAND, I like to play games and hang out in the districts!",
  country: 'Argentina',
  language: 'English',
  pronouns: 'He / Him',
  gender: 'Male',
  sexualOrientation: 'Bisexual',
  relationshipStatus: 'Single',
  employmentStatus: 'Working',
  profession: 'Dev',
  realName: 'George Pascal',
  hobbies: 'Drawing',
  links: [
    { title: 'Instagram', url: 'https://www.instagram.com/decentraland' },
    { title: 'GitHub', url: 'https://github.com/decentraland' },
    { title: 'Youtube', url: 'https://www.youtube.com/@decentraland' },
    { title: 'Web', url: 'https://decentraland.org' },
    { title: 'Discord', url: 'https://discord.gg/decentraland' },
  ],
  equipped: MOCK_EQUIPPED,
};

export const MOCK_NAMES = ['Name 01', 'Name 02', 'Name'];
