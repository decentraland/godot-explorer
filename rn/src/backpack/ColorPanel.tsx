import { useCallback, useEffect, useRef } from 'react';
import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { AppText, HsvSliders, Icon, SwatchRow } from '../ds';
import { COLOR_TITLES, ColorTarget, PALETTES } from './taxonomy';
import { u } from '../ds/scale';

const SEND_INTERVAL_MS = 120;

/** Hair / skin / eye color editor that replaces the grid inside the sheet. */
export function ColorPanel({
  target,
  value,
  onChange,
  onBack,
}: {
  target: ColorTarget;
  value: string;
  onChange: (color: string) => void;
  onBack: () => void;
}) {
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pending = useRef<string | null>(null);
  const emit = useRef(onChange);
  emit.current = onChange;

  // Slider drags fire per frame; Godot rebuilds the avatar on each color, so coalesce them.
  const throttled = useCallback((color: string) => {
    pending.current = color;
    if (timer.current) {
      return;
    }
    timer.current = setTimeout(() => {
      timer.current = null;
      if (pending.current) {
        emit.current(pending.current);
        pending.current = null;
      }
    }, SEND_INTERVAL_MS);
  }, []);

  useEffect(
    () => () => {
      if (timer.current) {
        clearTimeout(timer.current);
      }
      if (pending.current) {
        emit.current(pending.current);
      }
    },
    [],
  );

  return (
    <ScrollView
      removeClippedSubviews={false}
      contentContainerStyle={styles.content}
      showsVerticalScrollIndicator={false}
    >
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Back"
        onPress={onBack}
        style={styles.header}
      >
        <Icon name="back" size={20} />
        <AppText variant="title" style={styles.title}>
          {COLOR_TITLES[target]}
        </AppText>
      </Pressable>
      <View style={styles.swatches}>
        <SwatchRow
          palette={PALETTES[target]}
          value={value}
          onChange={onChange}
        />
      </View>
      <HsvSliders key={target} value={value} onChange={throttled} />
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  content: { paddingHorizontal: u(24), paddingBottom: u(24) },
  header: { flexDirection: 'row', alignItems: 'center', paddingVertical: u(8) },
  title: { marginLeft: u(12) },
  swatches: { marginTop: u(14) },
});
