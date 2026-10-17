import { StyleSheet, View } from 'react-native';
import { LinearFill } from '../ds';

export type Backdrop = 'purple' | 'lavender';

const STOPS: Record<Backdrop, { offset: number; color: string }[]> = {
  purple: [
    { offset: 0, color: '#6B1FB5' },
    { offset: 0.5, color: '#A24BF3' },
    { offset: 1, color: '#C040D8' },
  ],
  lavender: [
    { offset: 0, color: '#C9A0F5' },
    { offset: 0.5, color: '#E8B9FF' },
    { offset: 1, color: '#F4D8FF' },
  ],
};

/** Gradient the avatar stands on; Godot draws the 3D avatar over it. */
export function ProfileBackdrop({
  variant = 'purple',
}: {
  variant?: Backdrop;
}) {
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="none">
      <LinearFill stops={STOPS[variant]} angle={30} />
    </View>
  );
}
