import { StyleSheet, View } from 'react-native';
import { SvgXml } from 'react-native-svg';
import { colors, LinearFill, RadialFill } from '../ds';
import { u } from '../ds/scale';
import { getFlag } from './flags';

const BATS = [
  { x: 0.42, y: 0.04, w: 75 },
  { x: 0.82, y: 0.38, w: 75 },
  { x: 0.28, y: 0.52, w: 48 },
  { x: 0.7, y: 0.76, w: 60 },
];
const BAT = (fill: string) =>
  `<svg viewBox="0 0 34 18"><path d="M2 10 Q8 2 14 8 Q17 3 20 8 Q26 2 32 10 Q26 8 24 14 Q20 11 17 16 Q14 11 10 14 Q8 8 2 10 Z" fill="${fill}"/></svg>`;

const VIOLET = [
  { offset: 0, color: '#7A22B2' },
  { offset: 0.3, color: colors.purple450 },
  { offset: 0.7, color: colors.purple500 },
  { offset: 1, color: '#7A1AA0' },
];

/** The Discover violet gradient; with the seasonal flag on, the dark vignette with bats. */
export function DiscoverBackground({
  seasonal,
  tone = 'violet',
}: {
  seasonal?: boolean;
  tone?: 'violet' | 'grey';
}) {
  const on = seasonal ?? getFlag('seasonalBackground');
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="none">
      {on ? (
        <>
          <RadialFill
            stops={[
              { offset: 0, color: tone === 'grey' ? '#1F1F1F' : '#3F1070' },
              { offset: 1, color: tone === 'grey' ? '#0A0A0A' : '#14062A' },
            ]}
          />
          {BATS.map((bat, i) => (
            <View
              key={i}
              style={[
                styles.bat,
                { left: `${bat.x * 100}%`, top: `${bat.y * 100}%` },
              ]}
            >
              <SvgXml
                xml={BAT(tone === 'grey' ? '#2A2A2A' : '#2A0F4A')}
                width={u(bat.w / 2)}
                height={u(bat.w / 4)}
              />
            </View>
          ))}
        </>
      ) : (
        <LinearFill angle={150} stops={VIOLET} />
      )}
    </View>
  );
}

const styles = StyleSheet.create({ bat: { position: 'absolute' } });
