import { useCallback, useEffect, useState } from 'react';
import {
  BackHandler,
  Pressable,
  ScrollView,
  StyleSheet,
  useWindowDimensions,
  View,
} from 'react-native';
import { ReactNode } from 'react';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AppText, colors, Icon, LinearFill, u, useToast } from '../ds';
import { t } from '../i18n';
import { getSettingsApi, SettingsApi } from './api';
import { isSplitLayout } from './format';
import { SectionBody, PreviewState } from './SectionBody';
import { SectionRows, VersionRow } from './SectionList';
import { SectionKey, sectionLabel } from './sections';
import { useSettings } from './useSettings';

const BG_STOPS = [
  { offset: 0, color: '#6B1FA6' },
  { offset: 0.4, color: '#470071' },
  { offset: 1, color: '#470071' },
];

function BackButton({ onPress }: { onPress: () => void }) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t('SETTINGS_BACK')}
      hitSlop={12}
      onPress={onPress}
    >
      <Icon name="back" size={24} />
    </Pressable>
  );
}

/**
 * Portrait: root list, tap a row for its detail. Landscape: permanent section column on the left,
 * the selected section scrolling on the right.
 */
export function SettingsScreen({
  api = getSettingsApi(),
  footer,
  initialSection = null,
  preview,
  onClose,
  onSignedOut,
}: {
  api?: SettingsApi;
  /** Bottom nav, portrait only. */
  footer?: ReactNode;
  initialSection?: SectionKey | null;
  preview?: PreviewState;
  onClose?: () => void;
  onSignedOut?: () => void;
}) {
  const { width, height } = useWindowDimensions();
  const insets = useSafeAreaInsets();
  const toast = useToast();
  const onSaveError = useCallback(
    () => toast(t('SETTINGS_SAVE_FAILED'), { icon: 'warning' }),
    [toast],
  );
  const m = useSettings(api, onSaveError);
  const split = isSplitLayout(width, height);
  const [section, setSection] = useState<SectionKey | null>(initialSection);
  const current = split ? section ?? 'graphics' : section;

  useEffect(() => {
    if (split || section === null) {
      return;
    }
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      setSection(null);
      return true;
    });
    return () => sub.remove();
  }, [split, section]);

  const copyVersion = () => {
    m.api.copyText(m.version.full);
    toast(t('TOAST_VERSION_COPIED'), { icon: 'check' });
  };
  const versionText = m.env.devBuild ? m.version.full : m.version.short;
  const body = current && (
    <SectionBody
      section={current}
      m={m}
      preview={preview}
      onSignedOut={onSignedOut}
    />
  );

  if (split) {
    return (
      <View style={[styles.root, styles.row]}>
        <LinearFill stops={BG_STOPS} angle={45} />
        <View
          style={[styles.column, { paddingTop: Math.max(insets.top, u(12)) }]}
        >
          <View style={styles.backRow}>
            <BackButton onPress={() => onClose?.()} />
          </View>
          <SectionRows
            devTools={m.env.devTools}
            selected={current}
            rowHeight={30}
            onSelect={setSection}
          />
          <View style={styles.spacer} />
          <VersionRow version={versionText} onCopy={copyVersion} />
        </View>
        <ScrollView
          style={styles.pane}
          showsVerticalScrollIndicator={false}
          contentContainerStyle={styles.paneContent}
        >
          {body}
        </ScrollView>
      </View>
    );
  }

  return (
    <View style={styles.root}>
      <LinearFill stops={BG_STOPS} angle={45} />
      <View style={[styles.portrait, { paddingTop: insets.top + u(3) }]}>
        <View style={[styles.header, !current && styles.headerRoot]}>
          {current && <BackButton onPress={() => setSection(null)} />}
          <AppText variant="heading" style={styles.title}>
            {t(current ? sectionLabel(current) : 'SETTINGS_SETTINGS')}
          </AppText>
        </View>
        {current ? (
          <ScrollView
            style={styles.flex}
            showsVerticalScrollIndicator={false}
            contentContainerStyle={styles.detailContent}
          >
            {body}
          </ScrollView>
        ) : (
          <View style={styles.flex}>
            <SectionRows
              devTools={m.env.devTools}
              rowHeight={51}
              onSelect={setSection}
            />
            <View style={styles.spacer} />
            <View style={styles.rootVersion}>
              <VersionRow version={versionText} onCopy={copyVersion} />
            </View>
          </View>
        )}
      </View>
      {footer}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple450 },
  flex: { flex: 1 },
  spacer: { flex: 1 },
  portrait: { flex: 1, paddingHorizontal: u(24) },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(14),
    marginBottom: u(30),
    minHeight: u(34),
  },
  headerRoot: { marginBottom: u(6) },
  title: { fontSize: u(24) },
  detailContent: { flexGrow: 1, paddingBottom: u(24) },
  rootVersion: { paddingBottom: u(32), paddingLeft: u(1) },
  row: { flexDirection: 'row' },
  column: {
    width: u(188),
    marginLeft: u(60),
    paddingBottom: u(26),
  },
  backRow: { height: u(36), justifyContent: 'center', marginBottom: u(10) },
  pane: { flex: 1, marginLeft: u(72), marginRight: u(55) },
  paneContent: { flexGrow: 1, paddingTop: u(55), paddingBottom: u(24) },
});
