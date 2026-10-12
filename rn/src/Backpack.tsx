/**
 * VIBE-CODED SPIKE: full-screen backpack. The transparent hole on the right is where Godot draws
 * the 3D avatar preview; the Godot surface sits above this view, so the avatar also overlaps the
 * glow, the floor and the watermark drawn here. Touches in the hole fall through to Godot.
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
import { godotAvailable, sendCommand } from './GodotBridge';
import { GodotPlaceholder } from './godot/GodotPlaceholder';
import { CATEGORIES, WearableItem } from './wearables';

const ACCENT = '#ff2d55';

export function Backpack({
  fps,
  onClose,
}: {
  fps: number;
  onClose: () => void;
}) {
  const [category, setCategory] = useState(CATEGORIES[0]);
  const [equipped, setEquipped] = useState<Record<string, string>>({});
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

  const equip = (item: WearableItem) => {
    setEquipped(current => ({ ...current, [category.key]: item.id }));
    sendCommand({ type: 'equip', urn: item.id });
  };

  return (
    <View style={styles.root} pointerEvents="box-none">
      <View style={styles.stage} pointerEvents="none">
        <Text style={styles.watermark} numberOfLines={1}>
          BACKPACK
        </Text>
        <View style={styles.glow} />
        <View style={styles.floorOuter} />
        <View style={styles.floorInner} />
      </View>
      <View style={styles.panel}>
        <View style={styles.header}>
          <View>
            <Text style={styles.title}>Backpack</Text>
            <Text style={styles.subtitle}>
              Customize your avatar · {fps.toFixed(0)} fps
            </Text>
          </View>
          <Pressable
            accessibilityLabel="Close"
            style={({ pressed }) => [styles.close, pressed && styles.pressed]}
            onPress={close}
          >
            <Text style={styles.closeText}>✕</Text>
          </Pressable>
        </View>
        <View style={styles.tabs}>
          {CATEGORIES.map(entry => {
            const active = entry.key === category.key;
            return (
              <Pressable
                key={entry.key}
                style={({ pressed }) => [
                  styles.tab,
                  active && styles.tabActive,
                  pressed && styles.pressed,
                ]}
                onPress={() => setCategory(entry)}
              >
                <Text style={styles.tabIcon}>{entry.icon}</Text>
                <Text style={[styles.tabText, active && styles.tabTextActive]}>
                  {entry.label}
                </Text>
              </Pressable>
            );
          })}
        </View>
        <ScrollView
          contentContainerStyle={styles.grid}
          showsVerticalScrollIndicator={false}
        >
          {category.items.map(item => {
            const active = equipped[category.key] === item.id;
            return (
              <Pressable
                key={item.id}
                style={({ pressed }) => [
                  styles.card,
                  active && styles.cardActive,
                  pressed && styles.pressed,
                ]}
                onPress={() => equip(item)}
              >
                <View style={[styles.rarity, active && styles.rarityActive]} />
                {active && (
                  <View style={styles.check}>
                    <Text style={styles.checkText}>✓</Text>
                  </View>
                )}
                <Text style={styles.cardIcon}>{category.icon}</Text>
                <Text style={styles.cardName} numberOfLines={1}>
                  {item.label}
                </Text>
                <Text style={styles.cardKind}>Base</Text>
              </Pressable>
            );
          })}
        </ScrollView>
        <Text style={styles.hint}>Drag the avatar to rotate</Text>
      </View>
      <View
        ref={hole}
        style={styles.hole}
        pointerEvents="none"
        onLayout={reportHole}
      >
        {!godotAvailable && <GodotPlaceholder label="Avatar preview" style={styles.holePlaceholder} />}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    ...StyleSheet.absoluteFill,
    flexDirection: 'row',
    backgroundColor: '#120c1c',
  },
  stage: { ...StyleSheet.absoluteFill, alignItems: 'flex-end' },
  watermark: {
    position: 'absolute',
    right: -10,
    top: '8%',
    width: '58%',
    textAlign: 'center',
    fontSize: 92,
    fontWeight: '900',
    letterSpacing: 6,
    color: 'rgba(255,255,255,0.07)',
  },
  glow: {
    position: 'absolute',
    right: '8%',
    top: '16%',
    width: '36%',
    aspectRatio: 1,
    borderRadius: 999,
    backgroundColor: 'rgba(255,45,85,0.09)',
  },
  floorOuter: {
    position: 'absolute',
    right: '9%',
    bottom: '3%',
    width: '34%',
    height: 34,
    borderRadius: 999,
    backgroundColor: 'rgba(255,45,85,0.22)',
  },
  floorInner: {
    position: 'absolute',
    right: '13%',
    bottom: '4.5%',
    width: '26%',
    height: 20,
    borderRadius: 999,
    backgroundColor: 'rgba(255,45,85,0.45)',
  },
  panel: {
    flex: 1,
    paddingTop: 30,
    paddingHorizontal: 22,
    paddingBottom: 14,
    backgroundColor: 'rgba(24,16,38,0.92)',
    borderRightWidth: 1,
    borderRightColor: 'rgba(255,255,255,0.06)',
  },
  hole: { flex: 1, backgroundColor: 'transparent' },
  holePlaceholder: { flex: 1, margin: 24, borderRadius: 16, opacity: 0.85 },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  title: { color: '#fff', fontSize: 26, fontWeight: '800' },
  subtitle: { color: 'rgba(255,255,255,0.5)', fontSize: 12, marginTop: 2 },
  close: {
    width: 36,
    height: 36,
    borderRadius: 18,
    backgroundColor: 'rgba(255,255,255,0.1)',
    alignItems: 'center',
    justifyContent: 'center',
  },
  closeText: { color: '#fff', fontSize: 15, fontWeight: '700' },
  tabs: { flexDirection: 'row', gap: 8, marginTop: 16, marginBottom: 14 },
  tab: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 6,
    paddingHorizontal: 14,
    paddingVertical: 8,
    borderRadius: 20,
    backgroundColor: 'rgba(255,255,255,0.07)',
  },
  tabActive: { backgroundColor: ACCENT },
  tabIcon: { fontSize: 14 },
  tabText: { color: 'rgba(255,255,255,0.7)', fontSize: 13, fontWeight: '600' },
  tabTextActive: { color: '#fff' },
  grid: { flexDirection: 'row', flexWrap: 'wrap', gap: 10, paddingBottom: 8 },
  card: {
    flexBasis: '31%',
    flexGrow: 1,
    maxWidth: '32%',
    aspectRatio: 1.05,
    borderRadius: 16,
    backgroundColor: 'rgba(255,255,255,0.06)',
    borderWidth: 1.5,
    borderColor: 'rgba(255,255,255,0.08)',
    alignItems: 'center',
    justifyContent: 'center',
    overflow: 'hidden',
  },
  cardActive: { backgroundColor: 'rgba(255,45,85,0.16)', borderColor: ACCENT },
  rarity: {
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    height: 4,
    backgroundColor: '#a09ba8',
  },
  rarityActive: { backgroundColor: ACCENT },
  check: {
    position: 'absolute',
    top: 8,
    right: 8,
    width: 20,
    height: 20,
    borderRadius: 10,
    backgroundColor: ACCENT,
    alignItems: 'center',
    justifyContent: 'center',
  },
  checkText: { color: '#fff', fontSize: 12, fontWeight: '800' },
  cardIcon: { fontSize: 30, marginBottom: 6 },
  cardName: {
    color: '#fff',
    fontSize: 13,
    fontWeight: '700',
    paddingHorizontal: 6,
  },
  cardKind: { color: 'rgba(255,255,255,0.4)', fontSize: 10, marginTop: 2 },
  pressed: { opacity: 0.8, transform: [{ scale: 0.97 }] },
  hint: {
    color: 'rgba(255,255,255,0.35)',
    fontSize: 11,
    textAlign: 'center',
    paddingTop: 6,
  },
});
