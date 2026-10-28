import { StyleSheet, Text, View } from 'react-native';
import { colors, fonts, u } from '../ds';

export type Notch = 'up' | 'down' | 'left';

const FILL = 'rgba(53,8,84,0.85)';

/** The purple label both the controls overlay and the tutorial point at things with. */
export function FtueTag({
  label,
  notch,
  small = false,
}: {
  label: string;
  notch?: Notch;
  /** The tutorial's instructions are smaller than the overlay's labels. */
  small?: boolean;
}) {
  return (
    <View style={[styles.pill, small && styles.pillSmall]}>
      <Text style={[styles.text, small && styles.textSmall]}>{label}</Text>
      {notch && <View style={[styles.notch, styles[notch]]} />}
    </View>
  );
}

const styles = StyleSheet.create({
  pill: {
    paddingHorizontal: u(7),
    paddingVertical: u(4.5),
    borderRadius: u(6),
    backgroundColor: FILL,
  },
  pillSmall: { maxWidth: u(220), paddingVertical: u(6) },
  text: { fontFamily: fonts.medium, fontSize: u(11.3), color: colors.snow },
  textSmall: { fontSize: u(9) },
  notch: {
    position: 'absolute',
    width: 0,
    height: 0,
    borderStyle: 'solid',
    borderColor: 'transparent',
  },
  up: {
    top: -u(4),
    left: '50%',
    marginLeft: -u(4),
    borderLeftWidth: u(4),
    borderRightWidth: u(4),
    borderBottomWidth: u(4),
    borderBottomColor: FILL,
  },
  down: {
    bottom: -u(4),
    left: '50%',
    marginLeft: -u(4),
    borderLeftWidth: u(4),
    borderRightWidth: u(4),
    borderTopWidth: u(4),
    borderTopColor: FILL,
  },
  left: {
    left: -u(4),
    top: '50%',
    marginTop: -u(4),
    borderTopWidth: u(4),
    borderBottomWidth: u(4),
    borderRightWidth: u(4),
    borderRightColor: FILL,
  },
});
