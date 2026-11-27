import { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';

/**
 * A grid kept mounted while another one shows, so showing it again mounts nothing. Hidden, it is
 * transparent, untouchable and out of the accessibility tree; not `display: none`, which unmounts
 * the native views and makes their images load again (blank frames) when the grid shows.
 */
export function KeptLayer({
  shown,
  children,
}: {
  shown: boolean;
  children: ReactNode;
}) {
  return (
    <View
      style={shown ? styles.shown : styles.hidden}
      pointerEvents={shown ? 'auto' : 'none'}
      importantForAccessibility={shown ? 'auto' : 'no-hide-descendants'}
      accessibilityElementsHidden={!shown}
    >
      {children}
    </View>
  );
}

const styles = StyleSheet.create({
  shown: { flex: 1 },
  hidden: { ...StyleSheet.absoluteFill, opacity: 0 },
});
