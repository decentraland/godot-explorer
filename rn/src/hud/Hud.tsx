import {
  useCallback,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
} from 'react';
import { Linking, PixelRatio, Pressable, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { HudCircleButton, u } from '../ds';
import { Control } from '../core/native';
import { worldRealm } from '../discover/api';
import { t } from '../i18n';
import { useMenuUnread } from '../menu/useMenuUnread';
import { getHudApi } from './activeApi';
import { ChatPanel } from './ChatPanel';
import { Segment } from './chatModel';
import { ControlsOverlay } from '../ftue/ControlsOverlay';
import { dismissControls, ftueTuning, useFtue } from '../ftue/store';
import type { HudApi } from './hudApi';
import {
  emitIntent,
  setUiHidden,
  useMenuOpen,
  useUiHidden,
} from './hudSignals';
import { HudUiState, hudReducer, initialHud } from './hudState';
import { HUD_SIZES, hudInsets } from './layout';
import { LinkDialogs } from './LinkDialogs';
import { MenuButton } from './MenuButton';
import { NotificationList } from './NotificationList';
import { ROW_LIFETIME_MS } from './notifications';
import type { ChatProfile } from './types';
import { fullName, useChatFeed } from './useChatFeed';
import { useNotificationRows } from './useNotificationRows';

/** The RN part of the in-world HUD; joystick and gamepad stay native and untouched under it. */
export function Hud({
  api = getHudApi(),
  initialUi = initialHud,
  rowLifetimeMs = ROW_LIFETIME_MS,
  welcome = true,
  animate = true,
}: {
  api?: HudApi;
  initialUi?: HudUiState;
  rowLifetimeMs?: number;
  welcome?: boolean;
  animate?: boolean;
}) {
  const ins = hudInsets(useSafeAreaInsets());
  const [ui, dispatch] = useReducer(hudReducer, initialUi);
  const menuOpen = useMenuOpen();
  const menuUnread = useMenuUnread();
  const hiddenByApp = useUiHidden();
  const ftue = useFtue();
  // The welcome line would land under the controls overlay's labels.
  const { feed, profiles, own, markRead } = useChatFeed(
    api,
    ui.chat !== 'closed',
    welcome && ftue.decided && !ftue.controls,
  );
  const rows = useNotificationRows(feed.lines, rowLifetimeMs);
  const [nearby, setNearby] = useState<ChatProfile[]>([]);
  const [pending, setPending] = useState<Segment | null>(null);

  useEffect(() => {
    dispatch({ type: 'menu', open: menuOpen });
  }, [menuOpen]);
  const seenHidden = useRef(hiddenByApp);
  useEffect(() => {
    if (seenHidden.current !== hiddenByApp) {
      seenHidden.current = hiddenByApp;
      dispatch({ type: 'hidden', hidden: hiddenByApp });
    }
  }, [hiddenByApp]);
  useEffect(
    () =>
      api.subscribe(event => {
        if (event.type === 'open_backpack') {
          emitIntent({ type: 'edit_emotes' });
        }
      }),
    [api],
  );

  useEffect(() => {
    if (ui.chat !== 'writing') {
      return undefined;
    }
    let live = true;
    api
      .nearby()
      .then(list => live && setNearby(list))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [api, ui.chat]);

  const ratio = PixelRatio.get();
  useEffect(() => {
    api.send(
      new Control.SetHudInsets({
        insets: {
          left: Math.round(ins.left * ratio),
          top: Math.round(ins.top * ratio),
          right: Math.round(ins.right * ratio),
          bottom: Math.round(ins.bottom * ratio),
        },
      }),
    );
  }, [api, ins.left, ins.top, ins.right, ins.bottom, ratio]);
  useEffect(() => {
    api.send(new Control.ChatOpen({ open: ui.chat === 'writing' }));
  }, [api, ui.chat]);
  useEffect(() => {
    api.send(new Control.HideUi({ hidden: ui.hidden }));
  }, [api, ui.hidden]);

  const ctx = useMemo(
    () => ({
      names: [...Object.values(profiles), ...nearby].map(fullName),
      myName: own ? fullName(own) : undefined,
    }),
    [profiles, nearby, own],
  );

  const setChat = useCallback(
    (mode: HudUiState['chat']) => {
      dispatch({ type: 'chat', mode });
      if (mode !== 'closed') {
        markRead();
      }
    },
    [markRead],
  );

  const onLink = (segment: Segment) => {
    if (segment.kind === 'mention') {
      const target = Object.values(profiles).find(
        p => fullName(p) === segment.value,
      );
      if (target) {
        emitIntent({ type: 'open_profile', address: target.address });
      }
    } else {
      setPending(segment);
    }
  };

  const confirm = (segment: Segment) => {
    setPending(null);
    const value = segment.value ?? '';
    if (segment.kind === 'url') {
      Linking.openURL(value);
      return;
    }
    if (segment.kind === 'coord') {
      const [x, y] = value.split(',').map(Number);
      api.send(new Control.Teleport({ parcel: { x, y } }));
    } else {
      api.enterWorld({ realm: worldRealm(value), parcel: { x: 0, y: 0 } });
    }
    setChat('closed');
  };

  if (ui.hidden) {
    return (
      <View
        style={[styles.corner, { top: ins.top, left: ins.left }]}
        pointerEvents="box-none"
      >
        <HudCircleButton
          icon="close"
          label={t('HUD_SHOW_UI')}
          size={HUD_SIZES.hideUi / 2}
          onPress={() => setUiHidden(false)}
        />
      </View>
    );
  }
  if (ui.menu) {
    return null;
  }

  const below = u(HUD_SIZES.menu / 2 + HUD_SIZES.listGap / 2);
  const chatOpen = ui.chat !== 'closed';
  return (
    <View style={StyleSheet.absoluteFill} pointerEvents="box-none">
      <View
        style={[styles.corner, { top: ins.top, left: ins.left }]}
        pointerEvents="box-none"
      >
        <View style={styles.group} pointerEvents="box-none">
          <MenuButton
            face={own?.face}
            badge={menuUnread}
            onPress={() => emitIntent({ type: 'open_menu' })}
          />
          <HudCircleButton
            icon="chat"
            label={t('HUD_CHAT')}
            size={HUD_SIZES.chat / 2}
            active={chatOpen}
            badge={feed.unread > 0 ? feed.unread : undefined}
            onPress={() => setChat(chatOpen ? 'closed' : 'open')}
          />
        </View>
        {!chatOpen && (
          <NotificationList rows={rows} profiles={profiles} ctx={ctx} />
        )}
      </View>
      {chatOpen && (
        <>
          <Pressable
            accessibilityLabel={t('CHAT_CLOSE')}
            style={StyleSheet.absoluteFill}
            onPress={() => setChat('closed')}
          />
          <ChatPanel
            mode={ui.chat as 'open' | 'writing'}
            lines={feed.lines}
            profiles={profiles}
            nearby={nearby}
            ctx={ctx}
            top={ins.top + below}
            left={ins.left}
            bottom={ins.bottom}
            onMode={setChat}
            onSend={text => api.send(new Control.SendChat({ text }))}
            onLink={onLink}
          />
        </>
      )}
      <LinkDialogs
        pending={pending}
        onCancel={() => setPending(null)}
        onConfirm={confirm}
      />
      {ftue.controls && (
        <ControlsOverlay
          elements={ftue.controls}
          insets={ins}
          lockSeconds={ftueTuning().overlayLockSeconds}
          onDismiss={dismissControls}
          animate={animate}
        />
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  corner: { position: 'absolute' },
  group: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(HUD_SIZES.groupGap / 2),
    marginBottom: u(HUD_SIZES.listGap / 2),
  },
});
