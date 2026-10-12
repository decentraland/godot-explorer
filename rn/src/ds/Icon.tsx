import { SvgXml } from 'react-native-svg';
import {
  backpackSvg,
  filterSvg,
  profileSvg,
  settingsSvg,
  wearablesSvg,
} from './iconSvgs';

const sources = {
  backpack: backpackSvg,
  filter: filterSvg,
  profile: profileSvg,
  settings: settingsSvg,
  wearables: wearablesSvg,
};

const strokes = {
  back: 'M15 4 L7 12 L15 20',
  check: 'M4 12.5 L9.5 18 L20 6.5',
  plus: 'M12 5 V19 M5 12 H19',
  close: 'M5 5 L19 19 M19 5 L5 19',
  compass:
    'M2.5 12 a9.5 9.5 0 1 0 19 0 a9.5 9.5 0 1 0 -19 0 M16 8 L13.6 13.6 L8 16 L10.4 10.4 Z',
  emotes:
    'M9.5 5 a2.5 2.5 0 1 0 5 0 a2.5 2.5 0 1 0 -5 0 M12 9 V15 M12 10.5 L6.5 7 M12 10.5 L18 8.5 M12 15 L8 21 M12 15 L16.5 21',
  chevronRight: 'M9 4 L17 12 L9 20',
} as const;

export type IconName = keyof typeof sources | keyof typeof strokes;

export function Icon({
  name,
  size = 24,
  color = '#FCFCFC',
  strokeWidth = 2.4,
}: {
  name: IconName;
  size?: number;
  color?: string;
  strokeWidth?: number;
}) {
  if (name in strokes) {
    const path = strokes[name as keyof typeof strokes];
    const xml = `<svg viewBox="0 0 24 24" fill="none"><path d="${path}" stroke="${color}" stroke-width="${strokeWidth}" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
    return <SvgXml xml={xml} width={size} height={size} />;
  }
  return (
    <SvgXml
      xml={sources[name as keyof typeof sources]}
      width={size}
      height={size}
    />
  );
}
