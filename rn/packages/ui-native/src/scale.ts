import { Dimensions } from 'react-native';

const { width, height } = Dimensions.get('window');

/** Figma frames are 360x800dp (720x1600px at 2x). Everything scales so that frame fits the phone:
 * by width on most phones, by height on one that is proportionally shorter than the frame. */
export const SCALE = Math.min(
  Math.min(width, height) / 360,
  Math.max(width, height) / 800,
);

/** Scales a design size given in Figma px / 2. */
export const u = (size: number): number => Math.round(size * SCALE * 100) / 100;
