import type { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';
import { ControlsFtueElement, TutorialStep } from '../core/native';
import { u } from '../ds';
import { registerPreview } from '../gallery/registry';
import { GodotPlaceholder } from '../godot/GodotPlaceholder';
import { hudInsets } from '../hud/layout';
import { MenuDrawerView } from '../menu/MenuDrawerView';
import { ControlsOverlay } from './ControlsOverlay';
import { StepOverlay, Tooltip, WelcomeCard } from './TutorialLayer';
import { startRun } from './tutorial';

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

function step(title: string, at: TutorialStep, menu = false) {
  const run = { ...startRun(false), step: at };
  return {
    title,
    orientation: 'landscape' as const,
    notes: NOTES,
    render: () => (
      <World>
        {menu && (
          <MenuDrawerView
            section="discover"
            unreadNotifications={false}
            onSelect={noop}
            onClose={noop}
          >
            <View />
          </MenuDrawerView>
        )}
        <StepOverlay run={run} insets={INSETS} animate={false} />
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
registerPreview('extra/ftue-welcome', {
  title: 'Tutorial: welcome card',
  orientation: 'landscape',
  render: () => (
    <World>
      <WelcomeCard visible onAnswer={noop} />
    </World>
  ),
});
registerPreview(
  'extra/ftue-step-move',
  step('Tutorial 1: move', TutorialStep.Move),
);
registerPreview(
  'extra/ftue-step-camera',
  step('Tutorial 2: camera', TutorialStep.Camera),
);
registerPreview(
  'extra/ftue-step-pinch',
  step('Tutorial 3: pinch', TutorialStep.Pinch),
);
registerPreview(
  'extra/ftue-step-chat',
  step('Tutorial 4: chat', TutorialStep.Chat),
);
registerPreview(
  'extra/ftue-step-menu',
  step('Tutorial 5: menu', TutorialStep.Menu),
);
registerPreview(
  'extra/ftue-step-social',
  step('Tutorial 6: social', TutorialStep.Social, true),
);
registerPreview(
  'extra/ftue-step-backpack',
  step('Tutorial 7: backpack', TutorialStep.Backpack, true),
);
registerPreview('extra/ftue-step-equip', {
  title: 'Tutorial 7: equip hint',
  orientation: 'landscape',
  notes: 'Drawn over the Backpack screen',
  render: () => (
    <World>
      <Tooltip text="FTUE_TUTORIAL_EQUIP" x={u(400)} y={u(64)} place="center" />
    </World>
  ),
});

const styles = StyleSheet.create({ fill: { flex: 1 } });
