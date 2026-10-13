import { useEffect } from 'react';
import { StyleSheet, View } from 'react-native';
import { Place } from '../discover/api';
import { send, useEngine } from '../engine';
import { setOrientation } from '../GodotBridge';
import { getHudApi } from '../hud/activeApi';
import { Hud } from '../hud/Hud';
import { LoadingCover } from './LoadingCover';
import { useLoading } from './useLoading';

/**
 * The world as RN sees it: Godot draws the 3D scene under this view. The loading cover hides the
 * boot, then the HUD takes over; joystick, gamepad, scene UI and camera look stay native.
 */
export function WorldScreen({
  place,
  onLeave,
}: {
  place: Place;
  onLeave: () => void;
}) {
  const { hud } = useEngine();
  const { state, dispatch } = useLoading(getHudApi(), hud.phase === 'playing');

  useEffect(() => {
    setOrientation('landscape');
    return () => setOrientation('portrait');
  }, []);

  const reload = () => {
    dispatch({ type: 'restart' });
    send({ type: 'enter_world', x: place.x, y: place.y, realm: place.realm });
  };

  return (
    <View style={styles.root} pointerEvents="box-none">
      {state.phase === 'finished' && <Hud />}
      <LoadingCover
        place={place}
        phase={state.phase}
        progress={state.progress}
        onClose={onLeave}
        onReload={reload}
        onStartAnyway={() => dispatch({ type: 'dismiss' })}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: 'transparent' },
});
