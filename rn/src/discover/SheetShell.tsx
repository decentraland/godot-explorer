import { ReactNode, useState } from 'react';
import {
  Pressable,
  ScrollView,
  StyleSheet,
  useWindowDimensions,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import {
  BottomSheet,
  colors,
  Icon,
  SHEET_HANDLE_HEIGHT,
  SheetFooter,
  Snap,
  u,
} from '@dcl/ui-native';
import { t } from '../i18n';

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
        config={{ peek: 0, half: fraction, full: expandable ? 1 : fraction }}
      >
        <View style={{ height: visible - SHEET_HANDLE_HEIGHT }}>
          {expanded ? (
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={t('DISCOVER_CLOSE')}
              hitSlop={12}
              onPress={onClose}
              style={[
                styles.close,
                { marginTop: insets.top - SHEET_HANDLE_HEIGHT },
              ]}
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
          <SheetFooter bottomInset={insets.bottom}>{footer}</SheetFooter>
        </View>
      </BottomSheet>
    </View>
  );
}

const styles = StyleSheet.create({
  scrim: { ...StyleSheet.absoluteFill, backgroundColor: colors.black60 },
  close: {
    paddingHorizontal: u(24),
    paddingVertical: u(10),
    alignSelf: 'flex-start',
  },
  content: { paddingHorizontal: u(24), paddingBottom: u(12) },
  contentExpanded: { paddingTop: u(24) },
});
