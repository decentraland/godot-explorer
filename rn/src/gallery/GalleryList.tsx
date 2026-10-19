import { Linking, Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AppText, colors, u } from '../ds';
import { getPreview, previewGroup, previewIds } from './registry';

/** All registered previews grouped by manifest source; tap opens the deep link. */
export function GalleryList() {
  const insets = useSafeAreaInsets();
  const groups = new Map<string, string[]>();
  previewIds().forEach(id => {
    const group = previewGroup(id);
    groups.set(group, [...(groups.get(group) ?? []), id]);
  });
  return (
    <ScrollView
      style={styles.root}
      contentContainerStyle={{
        paddingTop: insets.top + u(16),
        paddingBottom: insets.bottom + u(24),
      }}
    >
      <AppText variant="heading" style={styles.pad}>
        Preview gallery
      </AppText>
      {[...groups].map(([group, ids]) => (
        <View key={group}>
          <AppText variant="title" style={[styles.pad, styles.group]}>
            {group} ({ids.length})
          </AppText>
          {ids.map(id => (
            <Pressable
              key={id}
              accessibilityRole="button"
              onPress={() => Linking.openURL(`dclrn://preview/${id}`)}
              style={styles.row}
            >
              <AppText variant="body">{getPreview(id)?.title}</AppText>
              <AppText variant="caption" style={styles.id}>
                {id} · {getPreview(id)?.orientation}
              </AppText>
            </Pressable>
          ))}
        </View>
      ))}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: colors.purple550 },
  pad: { paddingHorizontal: u(16) },
  group: { marginTop: u(16), marginBottom: u(4) },
  row: { paddingHorizontal: u(16), paddingVertical: u(8) },
  id: { opacity: 0.6 },
});
