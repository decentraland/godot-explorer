import { useEffect, useState } from 'react';
import { Image, StyleSheet, View } from 'react-native';
import { core } from '../core';
import { useSeasonalSkin } from './seasonalSkin';

let loaded = false;

/** The season's image over the Discover background and under its content. A failed load leaves
 * Discover as it is and is tried again on a later open, up to the core's attempt cap. */
export function SeasonalOverlay() {
  const skin = useSeasonalSkin();
  const [show, setShow] = useState(false);
  const url = skin?.overlayUrl;
  useEffect(() => {
    setShow(
      url !== undefined && (loaded || core().seasonalSkinTakeFetchAttempt()),
    );
  }, [url]);
  if (!url || !show) {
    return null;
  }
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="none">
      <Image
        source={{ uri: url }}
        resizeMode="cover"
        style={StyleSheet.absoluteFill}
        onLoad={() => {
          loaded = true;
        }}
        onError={() => setShow(false)}
      />
    </View>
  );
}
