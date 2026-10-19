import { useEffect, useState } from 'react';
import { StyleSheet, View } from 'react-native';
import { AppText, u } from '../ds';
import { core } from '../core';
import { onCoreEvents } from '../core/events';

/** Pre-release builds carry more than `x.y.z`; only those show the frame rate. */
export const showsFps = (version: string): boolean => version.includes('-');

/** The build's version and, outside production, the frame rate, at the bottom of the world. */
export function VersionFps() {
  const [version, setVersion] = useState('');
  const [fps, setFps] = useState<number | null>(null);
  useEffect(() => {
    try {
      setVersion(core().version());
    } catch {
      // The engine registers the build a moment after the core; the label stays empty until then.
    }
    return onCoreEvents({ hud: snapshot => setFps(Math.round(snapshot.fps)) });
  }, []);
  if (!version) {
    return null;
  }
  const line =
    showsFps(version) && fps !== null ? `${version} - ${fps} FPS` : version;
  return (
    <View style={styles.root} pointerEvents="none">
      <AppText style={styles.text}>{line}</AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    position: 'absolute',
    bottom: u(6),
    left: 0,
    right: 0,
    alignItems: 'center',
  },
  text: {
    fontSize: u(12),
    color: 'rgba(255,255,255,0.7)',
    textShadowColor: 'rgba(0,0,0,0.5)',
    textShadowRadius: 2,
  },
});
