import { ReactNode } from 'react';
import {
  ScrollView,
  StyleProp,
  StyleSheet,
  View,
  ViewStyle,
} from 'react-native';
import { LinearFill } from './Gradient';
import { colors, elevation, radius } from './tokens';
import { u } from './scale';

export type SheetHeight = 'hug' | 'fixed' | 'collapsed';

/** Figma `Sheet` 841:1057 laid out in place (no drag; `BottomSheet` is the draggable one).
 * `fixed` fills the height the caller gives it and scrolls the content; `collapsed` is the handle alone. */
export function Sheet({
  handle = true,
  height = 'hug',
  footer,
  footerInset,
  style,
  children,
}: {
  handle?: boolean;
  height?: SheetHeight;
  /** The button row of `Sheet/Footer`, pinned under the body. */
  footer?: ReactNode;
  /** Bottom safe-area inset passed to the footer. */
  footerInset?: number;
  style?: StyleProp<ViewStyle>;
  children?: ReactNode;
}) {
  const fixed = height === 'fixed';
  return (
    <View style={[styles.sheet, fixed && styles.fill, style]}>
      <View
        style={[
          styles.body,
          handle ? styles.bodyHandle : styles.bodyNoHandle,
          fixed && styles.fill,
        ]}
      >
        <LinearFill
          stops={[
            { offset: 0, color: colors.purple450 },
            { offset: 0.34, color: colors.purple500 },
          ]}
        />
        {handle && <View style={styles.handle} />}
        {height === 'hug' && <View style={styles.content}>{children}</View>}
        {fixed && (
          <ScrollView
            style={styles.fill}
            contentContainerStyle={styles.content}
            showsVerticalScrollIndicator={false}
          >
            {children}
          </ScrollView>
        )}
      </View>
      {footer && height !== 'collapsed' && (
        <SheetFooter bottomInset={footerInset}>{footer}</SheetFooter>
      )}
    </View>
  );
}

/** Figma `Sheet/Footer` 841:1039: 100 dp of Purple 500 with the button row 9 dp from the top; the
 * bottom keeps 20 dp or the safe-area inset, whichever is larger. */
export function SheetFooter({
  bottomInset = 0,
  children,
}: {
  bottomInset?: number;
  children: ReactNode;
}) {
  return (
    <View
      style={[styles.footer, { paddingBottom: Math.max(u(20), bottomInset) }]}
    >
      {children}
    </View>
  );
}

const styles = StyleSheet.create({
  sheet: { width: '100%', ...elevation.up },
  fill: { flex: 1 },
  body: {
    alignItems: 'center',
    gap: u(20),
    paddingHorizontal: u(24),
    paddingBottom: u(15),
    borderTopLeftRadius: radius['2xl'],
    borderTopRightRadius: radius['2xl'],
    overflow: 'hidden',
    backgroundColor: colors.purple500,
  },
  bodyHandle: { paddingTop: u(8) },
  bodyNoHandle: { paddingTop: u(20) },
  handle: {
    width: u(100),
    height: u(4),
    borderRadius: radius.xs,
    backgroundColor: colors.white30,
  },
  content: { alignSelf: 'stretch', minHeight: u(100) },
  footer: {
    minHeight: u(100),
    paddingTop: u(9),
    paddingHorizontal: u(24),
    backgroundColor: colors.purple500,
  },
});
