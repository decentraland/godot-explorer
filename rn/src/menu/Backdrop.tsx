import { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';
import { LinearFill, u } from '../ds';

const PAD = [
  { right: u(51), bottom: u(36), size: u(40) },
  { right: u(86), bottom: u(76), size: u(40) },
  { right: u(30), bottom: u(60), size: u(40) },
];

/** Stand-in for the Godot world behind the drawer in gallery previews, with a ghost gamepad. */
export function Backdrop({
  children,
  pad = true,
}: {
  children: ReactNode;
  pad?: boolean;
}) {
  return (
    <View style={styles.root}>
      <LinearFill
        angle={170}
        stops={[
          { offset: 0, color: '#6B1F3E' },
          { offset: 1, color: '#3A0F20' },
        ]}
      />
      {pad &&
        PAD.map((p, i) => (
          <View
            key={i}
            style={[
              styles.ghost,
              {
                right: p.right,
                bottom: p.bottom,
                width: p.size,
                height: p.size,
              },
            ]}
          />
        ))}
      {children}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#3A0F20' },
  ghost: {
    position: 'absolute',
    borderRadius: u(20),
    borderWidth: 1,
    borderColor: 'rgba(255,255,255,0.35)',
    backgroundColor: 'rgba(255,255,255,0.12)',
  },
});
