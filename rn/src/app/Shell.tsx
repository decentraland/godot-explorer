import { StyleSheet, View } from 'react-native';
import { TutorialLayer } from '../ftue/TutorialLayer';
import { WorldScreen } from '../world/WorldScreen';
import { HomeTabs } from './HomeTabs';
import { useNav } from './NavProvider';
import { visibleRoutes } from './navReducer';
import { getScreen } from './registry';
import type { Place } from '../discover/api';

/** Renders the route stack: tab shell at the root, sheets above it, the world replacing both. */
export function Shell({
  onJumpIn,
  onLeave,
}: {
  onJumpIn: (place: Place) => void;
  onLeave: () => void;
}) {
  const { state } = useNav();
  const routes = visibleRoutes(state);
  const inWorld = routes[0].name === 'world';
  const Drawer = getScreen('worldDrawer');

  return (
    <View style={styles.fill} pointerEvents={inWorld ? 'box-none' : 'auto'}>
      {routes.map((route, i) => {
        const key = `${i}:${route.name}`;
        if (route.name === 'home') {
          return <HomeTabs key={key} onJumpIn={onJumpIn} />;
        }
        if (route.name === 'world') {
          return (
            <View key={key} style={styles.fill} pointerEvents="box-none">
              <WorldScreen place={route.params.place} onLeave={onLeave} />
              {Drawer ? <Drawer params={undefined as never} /> : null}
              <TutorialLayer />
            </View>
          );
        }
        const Screen = getScreen(route.name);
        return Screen ? (
          <Screen key={key} params={route.params as never} />
        ) : null;
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  fill: StyleSheet.absoluteFill,
});
