import { useState } from 'react';
import { Image, StyleSheet, View } from 'react-native';
import { colors, Icon, LinearFill } from '../ds';
import { u } from '../ds/scale';

const PALETTE = [
  ['#8A5A2B', '#3C5BA8'],
  ['#D8483A', '#3E7CA8'],
  ['#4A8F5A', '#C86B3A'],
  ['#C9A13A', '#7A3A8F'],
  ['#2F6B7A', '#C89B5A'],
  ['#5A8F3A', '#D8C84A'],
  ['#161518', '#2B2B2B'],
  ['#8FA8C8', '#C89B8F'],
  ['#8A3A2B', '#D8A13A'],
  ['#D8C8B8', '#4A3A5A'],
  ['#6AA8C8', '#B8D86A'],
  ['#3A8F5A', '#2B5A8F'],
  ['#7A3AC8', '#C83A8F'],
  ['#4A4A8F', '#8F4A4A'],
  ['#5A3A8F', '#3AC8A1'],
  ['#7A22B2', '#FF7439'],
  ['#E87A30', '#2B4AA8'],
  ['#1F2F8F', '#C83AC8'],
  ['#C83A8F', '#F0D84A'],
  ['#2F8F6B', '#8F2F6B'],
  ['#8F6B2F', '#2F6B8F'],
] as const;

type Size = number | `${number}%`;

/** Image with a frame; a failed or `mock:error` image becomes the dark tile with the crossed-image icon. */
export function Thumb({
  uri,
  width,
  height,
  radius = 4,
}: {
  uri: string;
  width: Size;
  height: Size;
  radius?: number;
}) {
  const [failed, setFailed] = useState(false);
  const mock = uri.startsWith('mock:') ? uri.slice(5) : null;
  const broken = failed || mock === 'error' || uri === '';
  const colorsFor =
    mock && mock !== 'error' ? PALETTE[Number(mock) % PALETTE.length] : null;
  return (
    <View style={[styles.frame, { width, height, borderRadius: u(radius) }]}>
      {broken ? (
        <View style={styles.broken}>
          <Icon
            name="imageOff"
            size={28}
            color={colors.lavender70}
            strokeWidth={1.6}
          />
        </View>
      ) : colorsFor ? (
        <LinearFill
          angle={135}
          stops={[
            { offset: 0, color: colorsFor[0] },
            { offset: 1, color: colorsFor[1] },
          ]}
        />
      ) : (
        <Image
          source={{ uri }}
          style={styles.fill}
          resizeMode="cover"
          onError={() => setFailed(true)}
        />
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  frame: {
    overflow: 'hidden',
    borderWidth: 0.5,
    borderColor: 'rgba(232,185,255,0.7)',
    backgroundColor: colors.purple450,
  },
  fill: { width: '100%', height: '100%' },
  broken: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: '#3D0F66',
  },
});
