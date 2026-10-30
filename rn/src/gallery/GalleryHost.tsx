import { StyleSheet, View } from 'react-native';
import { AppText, colors, u } from '../ds';
import { GalleryList } from './GalleryList';
import { PreviewFrame } from './PreviewFrame';
import { getPreview } from './registry';
import { PreviewRoute } from './url';

/** Renders one registered screen full-bleed, or the list when no id is given. */
export function GalleryHost({
  id,
  orientation,
  state,
}: { id: string | null } & Pick<PreviewRoute, 'orientation' | 'state'>) {
  const entry = id ? getPreview(id) : undefined;
  if (!id) {
    return <GalleryList />;
  }
  if (!entry) {
    return (
      <View style={styles.root} testID="gallery-host">
        <AppText variant="heading" style={styles.msg}>
          Unknown preview {id}
        </AppText>
        <GalleryList />
      </View>
    );
  }
  return (
    <PreviewFrame
      id={id}
      orientation={orientation ?? entry.orientation}
      settleMs={entry.settleMs}
    >
      {entry.render(state)}
    </PreviewFrame>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: colors.purple550 },
  msg: { padding: u(16) },
});
