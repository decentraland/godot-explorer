import { StyleSheet, View, useWindowDimensions } from 'react-native';
import { colors, RadialFill } from '../ds';
import { BackButton, LayoutProps } from './BackpackChrome';
import { ConnectionLost } from './ConnectionLost';
import { LandscapeLayout } from './LandscapeLayout';
import { PortraitLayout } from './PortraitLayout';
import { u } from '../ds/scale';

/** Dim purple with a soft glow off to one side, behind the connection-lost state. */
function LostBackground() {
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="none">
      <View style={[StyleSheet.absoluteFill, styles.lostBase]} />
      <RadialFill
        cx={0.25}
        cy={0.5}
        rx={0.3}
        ry={0.7}
        stops={[
          { offset: 0, color: '#8A2EB0' },
          { offset: 1, color: '#35174F', opacity: 0 },
        ]}
      />
    </View>
  );
}

/** One responsive Backpack: portrait sheet layout, or landscape split when the window is wider than tall. */
export function BackpackView(props: LayoutProps) {
  const { width, height } = useWindowDimensions();
  const landscape = width > height;

  if (props.controller.catalog.status === 'error') {
    return (
      <View style={styles.lost}>
        <LostBackground />
        {props.onBack ? (
          <BackButton
            onPress={props.onBack}
            style={{ left: u(18), top: u(40) }}
          />
        ) : null}
        <ConnectionLost onRetry={props.controller.retry} />
        {landscape ? null : props.footer}
      </View>
    );
  }
  return landscape ? (
    <LandscapeLayout {...props} />
  ) : (
    <PortraitLayout {...props} />
  );
}

const styles = StyleSheet.create({
  lostBase: { backgroundColor: '#35174F' },
  lost: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple500 },
});
