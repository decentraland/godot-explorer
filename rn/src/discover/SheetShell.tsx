import { ReactNode, useState } from 'react';
import {
  Pressable,
  ScrollView,
  StyleSheet,
  useWindowDimensions,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { BottomSheet, colors, Icon, LinearFill, Snap } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';

const GRADIENT_DP = 142;
const HANDLE_DP = 100;
const SHEET_TOP = '#450F6D';

export type SheetSnap = 'collapsed' | 'expanded';

/**
 * Two-snap sheet over a dimmed backdrop: collapsed shows `fraction` of the screen, expanded fills it
 * and gains the close X. Dragging below collapsed, the backdrop and Android back all close it.
 * `footer` (the action buttons) stays pinned; `children` scroll only when expanded.
 */
export function SheetShell({
  fraction,
  initial = 'collapsed',
  expandable = true,
  onClose,
  children,
  footer,
  onSnapChange,
}: {
  fraction: number;
  initial?: SheetSnap;
  expandable?: boolean;
  onClose: () => void;
  children: (snap: SheetSnap) => ReactNode;
  footer: ReactNode;
  onSnapChange?: (snap: SheetSnap) => void;
}) {
  const { height } = useWindowDimensions();
  const insets = useSafeAreaInsets();
  const [snap, setSnap] = useState<SheetSnap>(initial);
  const expanded = snap === 'expanded';
  const visible = expanded ? height : Math.round(height * fraction);

  const change = (next: Snap) => {
    if (next === 'peek') {
      onClose();
      return;
    }
    const target: SheetSnap =
      next === 'full' && expandable ? 'expanded' : 'collapsed';
    setSnap(target);
    onSnapChange?.(target);
  };

  return (
    <View style={StyleSheet.absoluteFill}>
      {expanded ? null : (
        <Pressable
          accessibilityLabel={t('DISCOVER_CLOSE')}
          style={styles.scrim}
          onPress={onClose}
        />
      )}
      <BottomSheet
        snap={expanded ? 'full' : 'half'}
        onSnapChange={change}
        handleWidth={HANDLE_DP}
        background={
          expanded ? undefined : (
            <LinearFill
              stops={[
                { offset: 0, color: SHEET_TOP },
                { offset: GRADIENT_DP / height, color: colors.purple500 },
              ]}
            />
          )
        }
        config={{ peek: 0, half: fraction, full: expandable ? 1 : fraction }}
      >
        <View style={{ height: visible - u(24) }}>
          {expanded ? (
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={t('DISCOVER_CLOSE')}
              hitSlop={12}
              onPress={onClose}
              style={[styles.close, { marginTop: insets.top - u(24) }]}
            >
              <Icon name="close" size={20} strokeWidth={2.2} />
            </Pressable>
          ) : null}
          <ScrollView
            scrollEnabled={expanded}
            showsVerticalScrollIndicator={false}
            contentContainerStyle={[
              styles.content,
              expanded && styles.contentExpanded,
            ]}
          >
            {children(snap)}
          </ScrollView>
          <View
            style={[styles.footer, { paddingBottom: u(24) + insets.bottom }]}
          >
            {footer}
          </View>
        </View>
      </BottomSheet>
    </View>
  );
}

const styles = StyleSheet.create({
  scrim: { ...StyleSheet.absoluteFill, backgroundColor: 'rgba(24,3,48,0.72)' },
  close: {
    paddingHorizontal: u(24),
    paddingVertical: u(10),
    alignSelf: 'flex-start',
  },
  content: { paddingHorizontal: u(24), paddingBottom: u(12) },
  contentExpanded: { paddingTop: u(24) },
  footer: { paddingHorizontal: u(24), backgroundColor: colors.purple500 },
});
