import { Control_Tags, CoreError, State } from '../native';
import type { UiState } from '../native';
import type { ControlApi } from '../api/control';
import { createMockAvatar } from './avatar';
import { mockBus } from './emitter';
import type { MockBus } from './emitter';
import { createMockWorld } from './worldSim';

const invalid = (call: string, state: State) =>
  new CoreError.InvalidCall({ detail: `${call}: not valid in ${state.tag}` });

const signedIn = (state: State) =>
  state.tag === 'SignedIn' ||
  state.tag === 'AvatarPreview' ||
  state.tag === 'Loading' ||
  state.tag === 'InWorld';

const inWorldFlow = (state: State) =>
  state.tag === 'Loading' || state.tag === 'InWorld';

/** Worlds that load, the avatar preview and the engine commands, in memory. */
export function createMockControl(bus: MockBus = mockBus): ControlApi {
  const world = createMockWorld(bus);
  const avatar = createMockAvatar();
  avatar.onChange(next => bus.emit(l => l.avatarChanged(next)));
  let ui: UiState = { hidden: false, chatOpen: false };
  const showUi = (patch: Partial<UiState>) => {
    ui = { ...ui, ...patch };
    const next = ui;
    bus.emit(l => l.uiState(next));
  };

  return {
    firstScreenDrawn: () => {},
    enterWorld: destination => {
      const state = bus.state();
      if (!signedIn(state)) {
        throw invalid('enter_world', state);
      }
      world.enter(destination);
    },
    exitWorld: () => {
      const state = bus.state();
      if (!inWorldFlow(state)) {
        throw invalid('exit_world', state);
      }
      world.leave();
      bus.setState(new State.SignedIn());
    },
    showAvatarPreview: rect => {
      const state = bus.state();
      if (state.tag !== 'SignedIn' && state.tag !== 'AvatarPreview') {
        throw invalid('show_avatar_preview', state);
      }
      bus.setState(new State.AvatarPreview({ rect }));
    },
    hideAvatarPreview: () => {
      const state = bus.state();
      if (state.tag !== 'AvatarPreview') {
        throw invalid('hide_avatar_preview', state);
      }
      bus.setState(new State.SignedIn());
    },
    avatarState: avatar.avatarState,
    control: command => {
      const inWorld = bus.state().tag === 'InWorld';
      switch (command.tag) {
        case Control_Tags.Teleport:
          if (inWorld) {
            world.teleport(command.inner.parcel);
          }
          break;
        case Control_Tags.SendChat:
          if (inWorld) {
            world.say(command.inner.text);
          }
          break;
        case Control_Tags.ChatOpen:
          world.setChatOpen(command.inner.open);
          showUi({ chatOpen: command.inner.open });
          break;
        case Control_Tags.HideUi:
          showUi({ hidden: command.inner.hidden });
          break;
        case Control_Tags.AnswerSceneRequest:
          break;
        default:
          avatar.applyAvatarControl(command);
      }
    },
  };
}
