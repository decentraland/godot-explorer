import { Dimensions } from 'react-native';

const { width, height } = Dimensions.get('window');

/** Figma frames are 360dp wide (720px at 2x); phones are wider, so every size scales by this. */
export const SCALE = Math.min(width, height) / 360;

/** Scales a design size given in Figma px / 2. */
export const u = (size: number): number => Math.round(size * SCALE * 100) / 100;
