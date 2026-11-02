import type { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';
import { registerPreview } from '../gallery/registry';
import { GodotPlaceholder } from '../godot/GodotPlaceholder';
import { LoadingCover, CoverPlace } from '../world/LoadingCover';
import { LoadingPhase, TIPS } from '../world/loadingModel';
import type { Tip } from '../world/loadingModel';
import { Hud } from './Hud';
import { HudApi, coreHudApi } from './hudApi';
import { HudUiState, initialHud } from './hudState';
import type { ChatLine, ChatProfile } from './types';

const noop = () => {};

const CLARA: ChatProfile = {
  address: '0xc1a',
  name: 'Clara',
  tag: '#1l0u',
  claimed: false,
  color: '#FF7439',
  own: false,
};
const ME: ChatProfile = {
  ...CLARA,
  address: '0xme',
  color: '#FF4BED',
  own: true,
};
const SALLY: ChatProfile = {
  ...CLARA,
  address: '0x5a1',
  name: 'Sally',
  tag: '',
  claimed: true,
  color: '#34CE76',
};

const line = (id: number, address: string, message: string): ChatLine => ({
  id: `p${id}`,
  address,
  message,
  timestamp: id,
});

const FIVE_LINES: ChatLine[] = [
  ...[1, 2, 3, 4].map(n => line(n, CLARA.address, 'Teleported')),
  line(
    5,
    ME.address,
    'Teleported sdk sjflk s vnsd vnds vnkv xcv dxx . dcsdcc sd nmlk',
  ),
];

/** A frozen engine: replays `lines` once and answers profile lookups from `known`. */
function staticApi(lines: ChatLine[], known: ChatProfile[]): HudApi {
  return {
    ...coreHudApi,
    subscribe(listener) {
      lines.forEach(l =>
        listener({
          type: 'chat',
          data: {
            address: l.address,
            message: l.message,
            timestamp: l.timestamp,
          },
        }),
      );
      listener({ type: 'unread_chat', data: { count: 0 } });
      return noop;
    },
    send: noop,
    enterWorld: noop,
    resolveProfile: async address =>
      known.find(p => p.address === address) ?? CLARA,
    ownProfile: async () => ME,
    nearby: async () => known.filter(p => !p.own),
  };
}

const PLACE: CoverPlace = {
  title: 'Genesis Plaza',
  creator: 'Decentraland Foundation',
};
const PROGRESS = { loaded: 0, total: 0, mbps: 0.1, percent: 10 };

const tipOf = (id: string): Tip => TIPS.find(tip => tip.id === id) as Tip;

function cover(
  title: string,
  tip: Tip,
  phase: LoadingPhase = 'loading',
  place: CoverPlace | undefined = PLACE,
) {
  return {
    title,
    orientation: 'landscape' as const,
    render: () => (
      <LoadingCover
        place={place}
        phase={phase}
        progress={
          phase === 'loading'
            ? PROGRESS
            : { loaded: 412, total: 584, mbps: 3.2, percent: 71 }
        }
        tip={tip}
        onClose={noop}
        onReload={noop}
        onStartAnyway={noop}
      />
    ),
  };
}

registerPreview(
  'loading__loading-1__13-11729',
  cover('Loading 1', { id: 'generic', icon: 'bulb', key: 'TIP_EMOTES' }),
);
registerPreview(
  'loading__loading-2-tip-emotes__175-530',
  cover('Loading 2: emotes', tipOf('emotes')),
);
registerPreview(
  'loading__loading-3-tip-friends__178-21086',
  cover('Loading 3: friends', tipOf('friends')),
);
registerPreview(
  'loading__loading-4-tip-chat__178-21128',
  cover('Loading 4: chat', tipOf('chat')),
);
registerPreview(
  'extra/loading-predata',
  cover('Loading: before place data', tipOf('explore'), 'loading', undefined),
);
registerPreview(
  'extra/loading-slow',
  cover('Loading: slow modal', tipOf('creator'), 'slow'),
);
registerPreview(
  'extra/loading-failed',
  cover('Loading: failed', tipOf('explore'), 'failed'),
);

function World({ children }: { children: ReactNode }) {
  return (
    <View style={styles.fill}>
      <GodotPlaceholder label="World (Godot)" style={StyleSheet.absoluteFill} />
      {children}
    </View>
  );
}

function hudPreview(
  title: string,
  lines: ChatLine[] = FIVE_LINES,
  opts: {
    ui?: HudUiState;
    notes?: string;
  } = {},
) {
  return {
    title,
    orientation: 'landscape' as const,
    settleMs: 1500,
    notes: opts.notes ?? 'RN layer only: joystick and gamepad are native Godot',
    render: () => (
      <World>
        <Hud
          api={staticApi(lines, [CLARA, ME, SALLY])}
          initialUi={opts.ui}
          rowLifetimeMs={Infinity}
          welcome={false}
          animate={false}
        />
      </World>
    ),
  };
}

registerPreview('hud__hud__609-7021', hudPreview('HUD'));
registerPreview(
  'hud__hud-4-side-buttons__706-6623',
  hudPreview('HUD (+4 side buttons)'),
);
registerPreview(
  'hud__joystick-movement__706-6537',
  hudPreview('Joystick movement', []),
);
registerPreview(
  'hud__hud-3-secondaries__706-6149',
  hudPreview('HUD 3 secondaries', []),
);
registerPreview(
  'hud__hud-2-secondaries__706-6231',
  hudPreview('HUD 2 secondaries'),
);
registerPreview('hud__hud-defaults__1111-12496', hudPreview('HUD defaults'));
registerPreview(
  'hud__hud-1-secondary__706-6304',
  hudPreview('HUD 1 secondary'),
);
registerPreview(
  'hud__hud-no-secondary__706-6373',
  hudPreview('HUD no secondary'),
);
registerPreview(
  'hud__hideui__706-1414',
  hudPreview('Hide UI', FIVE_LINES, { ui: { ...initialHud, hidden: true } }),
);
registerPreview('menu__hud__367-4161', hudPreview('HUD with chat'));
registerPreview(
  'extra/hud-chat-open',
  hudPreview('Chat open', FIVE_LINES, { ui: { ...initialHud, chat: 'open' } }),
);
registerPreview(
  'extra/hud-chat-writing',
  hudPreview('Chat writing', FIVE_LINES, {
    ui: { ...initialHud, chat: 'writing' },
  }),
);

const styles = StyleSheet.create({ fill: { flex: 1 } });
