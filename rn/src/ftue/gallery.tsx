import type { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';
import { ControlsFtueElement } from '../core/native';
import { registerPreview } from '../gallery/registry';
import { GodotPlaceholder } from '../godot/GodotPlaceholder';
import { hudInsets } from '../hud/layout';
import { ControlsOverlay } from './ControlsOverlay';

const noop = () => {};
const INSETS = hudInsets({ top: 0, left: 0, right: 0, bottom: 0 });
const NOTES =
  'RN layer only: joystick, gamepad and emote button are native Godot';

function World({ children }: { children: ReactNode }) {
  return (
    <View style={styles.fill}>
      <GodotPlaceholder label="World (Godot)" style={StyleSheet.absoluteFill} />
      {children}
    </View>
  );
}

function controls(title: string, elements: ControlsFtueElement[]) {
  return {
    title,
    orientation: 'landscape' as const,
    notes: NOTES,
    render: () => (
      <World>
        <ControlsOverlay
          elements={elements}
          insets={INSETS}
          lockSeconds={0}
          onDismiss={noop}
          animate={false}
        />
      </World>
    ),
  };
}

registerPreview(
  'menu__hud__106-2203',
  controls('Controls overlay', [
    ControlsFtueElement.Menu,
    ControlsFtueElement.Chat,
    ControlsFtueElement.Movement,
    ControlsFtueElement.Emotes,
  ]),
);
registerPreview(
  'menu__hud__108-2191',
  controls('Controls overlay: a scene showed the joystick later', [
    ControlsFtueElement.Movement,
  ]),
);
const styles = StyleSheet.create({ fill: { flex: 1 } });
