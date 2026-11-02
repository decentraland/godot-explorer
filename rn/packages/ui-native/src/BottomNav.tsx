import { Pressable, StyleSheet, View } from 'react-native';
import Svg, {
  Defs,
  Ellipse,
  LinearGradient,
  RadialGradient,
  Rect,
  Stop,
} from 'react-native-svg';
import { Icon, IconName } from './Icon';
import { NotificationBadge } from './NotificationBadge';
import { colors } from './tokens';
import { u } from './scale';

export type NavKey = 'explore' | 'backpack' | 'settings' | 'profile';

const ICONS: Record<NavKey, { idle: IconName; active: IconName }> = {
  explore: { idle: 'navExplore', active: 'navExploreActive' },
  backpack: { idle: 'navBackpack', active: 'navBackpackActive' },
  settings: { idle: 'navSettings', active: 'navSettingsActive' },
  profile: { idle: 'navProfile', active: 'navProfileActive' },
};

const ORDER: NavKey[] = ['explore', 'backpack', 'settings', 'profile'];

const ITEM_PORTRAIT = u(41);
const RAIL_WIDTH = u(50);
/** Bar top edge to below the items; the rail box spans the item plus the Figma overhang. */
const INDICATOR_BAR_HEIGHT = u(77);
const INDICATOR_RAIL_HEIGHT = u(40.5);

/** Figma "Selected indicator": a blurred lavender ellipse centred 4.5px outside the bar edge (Portrait
 * 56×103 on the top edge, Landscape 75×42 on the rail's inner edge; blur 10, 60%, fading into the
 * bar) under a 2px `#E8B9FF` line whose ends fade out. The blur is a radial falloff over the
 * blurred extent: react-native-svg's Android blur is capped at 25 device px. Clipped like the frame. */
function SelectedIndicator({ landscape }: { landscape: boolean }) {
  const w = landscape ? RAIL_WIDTH : ITEM_PORTRAIT;
  const h = landscape ? INDICATOR_RAIL_HEIGHT : INDICATOR_BAR_HEIGHT;
  const cx = landscape ? w + u(2.25) : w / 2;
  const cy = landscape ? h / 2 : -u(2.25);
  const rx = landscape ? u(24) : u(20.5);
  const ry = landscape ? u(18) : u(24);
  const line = landscape ? u(34) : u(37);
  return (
    <Svg width={w} height={h} pointerEvents="none">
      <Defs>
        <RadialGradient
          id="navGlow"
          gradientUnits="userSpaceOnUse"
          cx={cx}
          cy={cy}
          fx={cx}
          fy={cy}
          rx={rx}
          ry={ry}
        >
          <Stop offset={0} stopColor="#DF9CFF" stopOpacity={0.6} />
          <Stop offset={0.35} stopColor="#DF9CFF" stopOpacity={0.35} />
          <Stop offset={0.65} stopColor="#B07BD0" stopOpacity={0.12} />
          <Stop offset={1} stopColor="#865E99" stopOpacity={0} />
        </RadialGradient>
        <LinearGradient
          id="navLine"
          x1={0}
          y1={0}
          x2={landscape ? 0 : 1}
          y2={landscape ? 1 : 0}
        >
          <Stop offset={0.096} stopColor="#E8B9FF" stopOpacity={0} />
          <Stop offset={0.495} stopColor="#E8B9FF" />
          <Stop offset={0.899} stopColor="#E8B9FF" stopOpacity={0} />
        </LinearGradient>
      </Defs>
      <Ellipse cx={cx} cy={cy} rx={rx} ry={ry} fill="url(#navGlow)" />
      <Rect
        x={landscape ? w - u(1) : (w - line) / 2}
        y={landscape ? (h - line) / 2 : 0}
        width={landscape ? u(1) : line}
        height={landscape ? line : u(1)}
        fill="url(#navLine)"
      />
    </Svg>
  );
}

/** Figma `NavBar` (`NavBar/Item` × 4): the active destination shows the gradient `Icon/NavBar` plus
 * the "Selected indicator" glow on the bar edge. Portrait is the bottom bar; `landscape` is the
 * 100px left rail. */
export function BottomNav({
  active,
  bottomInset = 0,
  badges,
  landscape = false,
  onChange,
}: {
  active: NavKey;
  bottomInset?: number;
  /** Destinations carrying a `Badge/Notification` dot. */
  badges?: Partial<Record<NavKey, boolean>>;
  landscape?: boolean;
  onChange: (key: NavKey) => void;
}) {
  const size = landscape ? 32 : 41;
  return (
    <View
      style={
        landscape
          ? styles.rail
          : [styles.bar, { paddingBottom: u(20) + bottomInset }]
      }
    >
      {ORDER.map(key => {
        const selected = key === active;
        return (
          <Pressable
            key={key}
            accessibilityRole="tab"
            accessibilityLabel={key}
            accessibilityState={{ selected }}
            hitSlop={u(8)}
            onPress={() => onChange(key)}
            style={{ width: u(size), height: u(size) }}
          >
            {selected && (
              <View
                style={[
                  styles.indicator,
                  landscape ? styles.indicatorRail : styles.indicatorBar,
                ]}
              >
                <SelectedIndicator landscape={landscape} />
              </View>
            )}
            <Icon
              name={selected ? ICONS[key].active : ICONS[key].idle}
              size={size}
            />
            {badges?.[key] && <NotificationBadge style={styles.badge} />}
          </Pressable>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  bar: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    justifyContent: 'center',
    gap: u(28),
    paddingTop: u(16),
    paddingHorizontal: u(32),
    backgroundColor: colors.navbar,
    boxShadow: [
      { offsetX: 0, offsetY: -u(2), blurRadius: u(2), color: '#00000080' },
    ],
  },
  rail: {
    width: u(50),
    alignItems: 'center',
    gap: u(18.5),
    paddingVertical: u(20),
    backgroundColor: colors.navbar,
  },
  indicator: { position: 'absolute', overflow: 'hidden' },
  indicatorBar: {
    left: 0,
    top: -u(16),
    width: ITEM_PORTRAIT,
    height: INDICATOR_BAR_HEIGHT,
  },
  indicatorRail: {
    left: -u(9),
    top: -u(4.6),
    width: RAIL_WIDTH,
    height: INDICATOR_RAIL_HEIGHT,
  },
  badge: { position: 'absolute', top: 0, right: 0 },
});
