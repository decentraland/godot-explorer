/**
 * VIBE-CODED SPIKE: full-screen backpack. The transparent hole on the right is where Godot draws
 * the 3D avatar preview underneath; touches inside it fall through to Godot.
 */
import { ComponentRef, useRef, useState } from 'react';
import {
  PixelRatio,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { sendCommand } from './GodotBridge';
import { CATEGORIES } from './wearables';

export function Backpack({
  fps,
  onClose,
}: {
  fps: number;
  onClose: () => void;
}) {
  const [category, setCategory] = useState(CATEGORIES[0]);
  const [equipped, setEquipped] = useState<string | null>(null);
  const opened = useRef(false);
  const hole = useRef<ComponentRef<typeof View>>(null);

  const reportHole = () => {
    hole.current?.measureInWindow((x, y, w, h) => {
      const scale = PixelRatio.get();
      const type = opened.current ? 'backpack_place' : 'backpack_open';
      opened.current = true;
      sendCommand({
        type,
        x: x * scale,
        y: y * scale,
        w: w * scale,
        h: h * scale,
      });
    });
  };

  const close = () => {
    sendCommand({ type: 'backpack_close' });
    onClose();
  };

  return (
    <View style={styles.root} pointerEvents="box-none">
      <View style={styles.panel}>
        <View style={styles.header}>
          <Text style={styles.title}>Backpack · {fps.toFixed(0)} fps</Text>
          <Pressable style={styles.close} onPress={close}>
            <Text style={styles.closeText}>Close</Text>
          </Pressable>
        </View>
        <View style={styles.tabs}>
          {CATEGORIES.map(entry => (
            <Pressable
              key={entry.key}
              style={[
                styles.tab,
                entry.key === category.key && styles.tabActive,
              ]}
              onPress={() => setCategory(entry)}
            >
              <Text style={styles.tabText}>{entry.label}</Text>
            </Pressable>
          ))}
        </View>
        <ScrollView contentContainerStyle={styles.grid}>
          {category.items.map(entry => (
            <Pressable
              key={entry.id}
              style={[styles.cell, entry.id === equipped && styles.cellActive]}
              onPress={() => {
                setEquipped(entry.id);
                sendCommand({ type: 'equip', urn: entry.id });
              }}
            >
              <Text style={styles.cellText}>{entry.label}</Text>
            </Pressable>
          ))}
        </ScrollView>
      </View>
      <View style={styles.banner} pointerEvents="none">
        <Text style={styles.bannerText}>RN banner under the avatar</Text>
      </View>
      <View
        ref={hole}
        style={styles.hole}
        pointerEvents="none"
        onLayout={reportHole}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, flexDirection: 'row' },
  panel: { flex: 1, backgroundColor: '#16101f', padding: 16 },
  banner: {
    position: 'absolute',
    left: 0,
    right: 0,
    top: '72%',
    height: 90,
    backgroundColor: 'rgba(255,45,85,0.85)',
    justifyContent: 'center',
    alignItems: 'center',
  },
  bannerText: { color: '#fff', fontSize: 28, fontWeight: '800' },
  hole: { flex: 1, backgroundColor: 'transparent' },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  title: { color: '#fff', fontSize: 22, fontWeight: '700' },
  close: {
    backgroundColor: 'rgba(255,45,85,0.9)',
    borderRadius: 16,
    paddingHorizontal: 14,
    paddingVertical: 6,
  },
  closeText: { color: '#fff', fontWeight: '600' },
  tabs: { flexDirection: 'row', gap: 8, marginVertical: 12 },
  tab: {
    backgroundColor: '#2a2038',
    borderRadius: 14,
    paddingHorizontal: 14,
    paddingVertical: 6,
  },
  tabActive: { backgroundColor: '#ff2d55' },
  tabText: { color: '#fff' },
  grid: { flexDirection: 'row', flexWrap: 'wrap', gap: 8 },
  cell: {
    width: 96,
    height: 64,
    borderRadius: 10,
    backgroundColor: '#2a2038',
    alignItems: 'center',
    justifyContent: 'center',
  },
  cellActive: { borderWidth: 2, borderColor: '#ff2d55' },
  cellText: { color: '#fff', fontSize: 13 },
});
