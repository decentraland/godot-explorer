import { useCallback, useEffect, useRef } from 'react';
import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { AppText, HsvSliders, Icon, layout } from '../ds';
import { t } from '../i18n';
import { isCustomColor } from './selection';
import { SwatchStrip } from './SwatchStrip';
import { bridgeTarget, COLOR_TITLES, ColorTarget, PALETTES } from './taxonomy';
import { u } from '../ds/scale';

const SEND_INTERVAL_MS = 120;

/** Hue / saturation / brightness editor that replaces the tabs and grid inside the sheet. */
export function ColorEditor({
  target,
  value,
  onChange,
  onBack,
  bottomInset = 0,
}: {
  target: ColorTarget;
  value: string;
  onChange: (color: string) => void;
  onBack: () => void;
  bottomInset?: number;
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

  const palette = PALETTES[bridgeTarget(target)];
  return (
    <ScrollView
      removeClippedSubviews={false}
      contentContainerStyle={{ paddingBottom: u(24) + bottomInset }}
      showsVerticalScrollIndicator={false}
    >
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('BACKPACK_BACK')}
        onPress={onBack}
        style={styles.header}
      >
        <Icon name="back" size={22} />
        <AppText variant="title" style={styles.title}>
          {t(COLOR_TITLES[target])}
        </AppText>
      </Pressable>
      <View style={styles.swatches}>
        <SwatchStrip
          palette={palette}
          value={value}
          custom={isCustomColor(palette, value)}
          onSelect={onChange}
        />
      </View>
      <View style={styles.sliders}>
        <HsvSliders key={target} value={value} onChange={throttled} />
      </View>
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: u(8),
    paddingHorizontal: layout.gutter,
  },
  title: { marginLeft: u(12), fontSize: u(18) },
  swatches: { marginTop: u(10) },
  sliders: { paddingHorizontal: layout.gutter },
});
