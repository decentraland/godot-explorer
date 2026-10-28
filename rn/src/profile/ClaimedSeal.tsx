import { SvgXml } from 'react-native-svg';
import { colors } from '../ds';
import { u } from '../ds/scale';

const SEAL =
  'M12 1.5 L14.8 3.8 L18.4 3.6 L19.3 7.1 L22 9.5 L20.7 12.8 L22 16 L19.3 18.4 L18.4 21.9 L14.8 21.7 L12 24 L9.2 21.7 L5.6 21.9 L4.7 18.4 L2 16 L3.3 12.8 L2 9.5 L4.7 7.1 L5.6 3.6 L9.2 3.8 Z';

/** Filled claimed-name seal with a white check. */
export function ClaimedSeal({ size = 16 }: { size?: number }) {
  const xml = `<svg viewBox="0 0 24 25" fill="none"><path d="${SEAL}" fill="${colors.mythic}"/><path d="M8 12.8 L11 15.8 L16.2 9.8" stroke="#fff" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  return <SvgXml xml={xml} width={u(size)} height={u(size)} />;
}
