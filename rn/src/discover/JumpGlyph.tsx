import { SvgXml } from 'react-native-svg';
import { u } from '../ds/scale';

const XML = `<svg viewBox="0 0 24 24"><rect x="1.5" y="1.5" width="21" height="21" rx="5" fill="none" stroke="#FFFFFF" stroke-opacity="0.55" stroke-width="1.5"/><path d="M6.5 10.2 H11.5 V7 L17.5 12 L11.5 17 V13.8 H6.5 Z" fill="#FFFFFF"/></svg>`;

/** The arrow-in-a-box that trails every JUMP IN label. */
export function JumpGlyph({ size = 16 }: { size?: number }) {
  return <SvgXml xml={XML} width={u(size)} height={u(size)} />;
}
