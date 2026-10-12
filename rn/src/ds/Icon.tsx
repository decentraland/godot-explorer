import { parse, SvgAst, SvgXml } from 'react-native-svg';
import {
  backpackOutlineSvg,
  backpackSvg,
  compassActiveSvg,
  compassOutlineSvg,
  profileFillSvg,
  profileRingSvg,
  filterSvg,
  profileSvg,
  settingsSvg,
  wearablesSvg,
} from './iconSvgs';
import { u } from './scale';

const sources = {
  backpack: backpackSvg,
  backpackOutline: backpackOutlineSvg,
  compassActive: compassActiveSvg,
  compassOutline: compassOutlineSvg,
  profileFill: profileFillSvg,
  profileRing: profileRingSvg,
  filter: filterSvg,
  profile: profileSvg,
  settings: settingsSvg,
  wearables: wearablesSvg,
};

/** Parsed once: re-parsing the exported SVGs on every mount occasionally rendered them empty on device. */
const asts = Object.fromEntries(
  Object.entries(sources).map(([name, xml]) => [name, parse(xml)]),
);

const strokes = {
  back: 'M15 4 L7 12 L15 20',
  check: 'M4 12.5 L9.5 18 L20 6.5',
  plus: 'M12 5 V19 M5 12 H19',
  close: 'M5 5 L19 19 M19 5 L5 19',
  compass:
    'M2.5 12 a9.5 9.5 0 1 0 19 0 a9.5 9.5 0 1 0 -19 0 M16 8 L13.6 13.6 L8 16 L10.4 10.4 Z',
  emotes:
    'M9.5 5 a2.5 2.5 0 1 0 5 0 a2.5 2.5 0 1 0 -5 0 M12 9 V15 M12 10.5 L6.5 7 M12 10.5 L18 8.5 M12 15 L8 21 M12 15 L16.5 21',
  search: 'M3 10.5 a7.5 7.5 0 1 0 15 0 a7.5 7.5 0 1 0 -15 0 M16 16 L21 21',
  user: 'M8 8 a4 4 0 1 0 8 0 a4 4 0 1 0 -8 0 M4 21 C4 16 8 14 12 14 C16 14 20 16 20 21',
  like: 'M7 11 V21 H4 V11 Z M7 11 L11 3 C13 3 14 4.5 13.5 6.5 L13 9 H19 C20.5 9 21.5 10.5 21 12 L19 19 C18.7 20.3 17.7 21 16.5 21 H7',
  dislike:
    'M7 13 V3 H4 V13 Z M7 13 L11 21 C13 21 14 19.5 13.5 17.5 L13 15 H19 C20.5 15 21.5 13.5 21 12 L19 5 C18.7 3.7 17.7 3 16.5 3 H7',
  heart:
    'M12 20.5 C5 15 3 11.5 3 8.5 A4.5 4.5 0 0 1 12 6.5 A4.5 4.5 0 0 1 21 8.5 C21 11.5 19 15 12 20.5 Z',
  share:
    'M15 6 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0 M3 12 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0 M15 18 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0 M8.7 10.5 L15.3 7.5 M8.7 13.5 L15.3 16.5',
  pin: 'M12 21 C12 21 5 14.5 5 9.5 A7 7 0 0 1 19 9.5 C19 14.5 12 21 12 21 Z M9.5 9.5 a2.5 2.5 0 1 0 5 0 a2.5 2.5 0 1 0 -5 0',
  jump: 'M3 3 H21 V21 H3 Z M7 12 H16 M12.5 8.5 L16 12 L12.5 15.5',
  chevronRight: 'M9 4 L17 12 L9 20',
} as const;

export type IconName = keyof typeof sources | keyof typeof strokes;

export function Icon({
  name,
  size: designSize = 24,
  color = '#FCFCFC',
  strokeWidth = 2.4,
}: {
  name: IconName;
  size?: number;
  color?: string;
  strokeWidth?: number;
}) {
  const size = u(designSize);
  if (name in strokes) {
    const path = strokes[name as keyof typeof strokes];
    const xml = `<svg viewBox="0 0 24 24" fill="none"><path d="${path}" stroke="${color}" stroke-width="${strokeWidth}" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
    return <SvgXml xml={xml} width={size} height={size} />;
  }
  return <SvgAst ast={asts[name]} override={{ width: size, height: size }} />;
}
