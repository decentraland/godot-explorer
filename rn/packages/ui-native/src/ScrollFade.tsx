import { ReactNode, useId, useState } from 'react';
import { StyleProp, StyleSheet, View, ViewStyle } from 'react-native';
import MaskedView from '@react-native-masked-view/masked-view';
import Svg, { Defs, LinearGradient, Rect, Stop } from 'react-native-svg';
import { u } from './scale';
import { colors } from './tokens';

const FADE = u(40);

/** Fades the edge of a scrollable row or grid to say there is more to scroll: an alpha mask, so it
 * works on any background. `edge` is where the fade goes; show it only while there is more content
 * that way.
 * Figma ScrollFade: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=973-29
 */
export function ScrollFade({
  edge = 'Right',
  style,
  children,
}: {
  edge?: 'Right' | 'Left' | 'Both' | 'Bottom';
  style?: StyleProp<ViewStyle>;
  children: ReactNode;
}) {
  const id = useId();
  const [size, setSize] = useState({ width: 0, height: 0 });
  const vertical = edge === 'Bottom';
  const length = vertical ? size.height : size.width;
  const fade = length > 0 ? Math.min(0.5, FADE / length) : 0;
  const stops = [
    ...(edge === 'Left' || edge === 'Both'
      ? [
          { offset: 0, opacity: 0 },
          { offset: fade, opacity: 1 },
        ]
      : [{ offset: 0, opacity: 1 }]),
    ...(edge === 'Left'
      ? [{ offset: 1, opacity: 1 }]
      : [
          { offset: 1 - fade, opacity: 1 },
          { offset: 1, opacity: 0 },
        ]),
  ];
  return (
    <MaskedView
      style={style}
      onLayout={event => setSize(event.nativeEvent.layout)}
      maskElement={
        size.width > 0 ? (
          <Svg width={size.width} height={size.height}>
            <Defs>
              <LinearGradient
                id={id}
                x1={0}
                y1={0}
                x2={vertical ? 0 : 1}
                y2={vertical ? 1 : 0}
              >
                {stops.map(stop => (
                  <Stop
                    key={stop.offset}
                    offset={stop.offset}
                    stopColor={colors.black}
                    stopOpacity={stop.opacity}
                  />
                ))}
              </LinearGradient>
            </Defs>
            <Rect
              width={size.width}
              height={size.height}
              fill={`url(#${id})`}
            />
          </Svg>
        ) : (
          <View style={styles.solid} />
        )
      }
    >
      {children}
    </MaskedView>
  );
}

const styles = StyleSheet.create({
  solid: { flex: 1, backgroundColor: colors.black },
});
