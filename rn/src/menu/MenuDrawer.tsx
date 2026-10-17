import { useCallback, useEffect, useMemo, useReducer, useState } from 'react';
import { Linking, StyleSheet, View } from 'react-native';
import type { DiscoverEvent, Place } from '../discover/api';
import { recordPlayed, usePlayed } from '../discover/history';
import { useToast } from '../ds';
import { t } from '../i18n';
import { setOrientation } from '../platform/orientation';
import { enterPlace } from '../world/enterPlace';
import { getMenuApi } from './api';
import { GuestError } from './coreNotificationsApi';
import { BugReportModal } from './BugReportModal';
import { DiscoverIngame } from './DiscoverIngame';
import { DiscoverPanel } from './DiscoverPanel';
import {
  closeMenu,
  MenuSection,
  selectSection,
  useDrawerState,
} from './drawerState';
import { MenuDrawerView } from './MenuDrawerView';
import { NotificationsPanel } from './NotificationsPanel';
import {
  markAllRead,
  markRead,
  notificationTarget,
  unreadCount,
} from './notifications';
import { ModalTarget, PlaceModal } from './PlaceModal';
import { REPORT_CONTENT_URL, shareScene } from './share';
import { socialReducer, SocialTab } from './social';
import { SocialPanel } from './SocialPanel';
import type { SceneInfo } from './types';
import { useLoad } from './useLoad';

type Overlay =
  | { kind: 'modal'; target: ModalTarget }
  | { kind: 'bug' }
  | { kind: 'discover' }
  | null;

/** Pages that replace the drawer; the shell owns the route stack, so it hands us the navigator. */
export type Navigate = (page: 'backpack' | 'settings' | 'profile') => void;

export function jumpTo(place: Place): void {
  closeMenu();
  recordPlayed(place);
  enterPlace(place);
}

function OpenDrawer({ navigate }: { navigate: Navigate }) {
  const api = getMenuApi();
  const toast = useToast();
  const { section } = useDrawerState();
  const now = useMemo(() => Date.now(), []);
  const played = usePlayed();
  const [overlay, setOverlay] = useState<Overlay>(null);
  const [reminders, setReminders] = useState<ReadonlySet<string>>(new Set());
  const [socialTab, setSocialTab] = useState<SocialTab>('friends');

  const scene = useLoad<SceneInfo>(api.scene);
  const feed = useLoad(api.discover);
  const notes = useLoad(api.notifications);
  const [social, dispatch] = useReducer(socialReducer, { status: 'loading' });

  const loadSocial = useCallback(
    (silent = false) => {
      if (api.isGuest()) {
        dispatch({ type: 'guest' });
        return;
      }
      if (!silent) {
        dispatch({ type: 'loading' });
      }
      api
        .social()
        .then(data => dispatch({ type: 'loaded', data }))
        .catch(error => {
          if (silent) {
            return;
          }
          dispatch({ type: error instanceof GuestError ? 'guest' : 'failed' });
        });
    },
    [api],
  );
  useEffect(() => loadSocial(), [loadSocial]);

  const items =
    notes.state.status === 'ready' && notes.state.data.status === 'ok'
      ? notes.state.data.items
      : [];
  const events = feed.state.status === 'ready' ? feed.state.data : null;

  const openEvent = (event: DiscoverEvent) =>
    setOverlay({
      kind: 'modal',
      target: {
        kind: 'event',
        event,
        featured: events?.featuredEvents.includes(event.id) ?? false,
      },
    });

  const withPortrait = (next: Overlay) => {
    setOverlay(next);
    setOrientation('portrait');
  };
  const leavePortrait = () => {
    setOverlay(null);
    setOrientation('landscape');
  };

  const select = (next: MenuSection) => {
    if (next === 'backpack' || next === 'settings' || next === 'profile') {
      closeMenu();
      navigate(next);
      return;
    }
    selectSection(next);
  };

  const openNotification = (id: string) => {
    const item = items.find(n => n.id === id);
    if (!item) {
      return;
    }
    notes.mutate(data =>
      data.status === 'ok'
        ? { status: 'ok', items: markRead(data.items, id) }
        : data,
    );
    api.markRead(id).catch(() => {
      notes.reload();
      toast(t('MENU_ACTION_FAILED'));
    });
    const target = notificationTarget(item);
    if (target && 'eventId' in target) {
      const event = events?.events.find(e => e.id === target.eventId);
      if (event) {
        openEvent(event);
      }
    } else if (target?.section === 'social') {
      setSocialTab(target.tab);
      selectSection('social');
    } else if (target?.section === 'backpack') {
      select('backpack');
    }
  };

  const social$ =
    (
      run: (address: string) => Promise<void>,
      action: 'accepted' | 'removed' | 'requested' | 'unblocked',
    ) =>
    (address: string) => {
      dispatch({ type: action, address });
      run(address).catch(() => {
        loadSocial();
        toast(t('MENU_ACTION_FAILED'));
      });
    };

  const panel = () => {
    switch (section) {
      case 'social':
        return (
          <SocialPanel
            key={socialTab}
            state={social}
            initialTab={socialTab}
            actions={{
              jump: address => {
                closeMenu();
                api.jumpToFriend(address).catch(() => undefined);
              },
              accept: social$(api.acceptRequest, 'accepted'),
              decline: social$(api.rejectRequest, 'removed'),
              cancel: social$(api.cancelRequest, 'removed'),
              add: social$(api.sendRequest, 'requested'),
              unblock: social$(api.unblock, 'unblocked'),
              retry: () => loadSocial(),
            }}
          />
        );
      case 'notifications':
        return (
          <NotificationsPanel
            result={notes.state}
            now={now}
            onOpen={openNotification}
            onRetry={notes.reload}
            onReadAll={() => {
              notes.mutate(data =>
                data.status === 'ok'
                  ? { status: 'ok', items: markAllRead(data.items) }
                  : data,
              );
              api.markAllRead().catch(() => {
                notes.reload();
                toast(t('MENU_ACTION_FAILED'));
              });
            }}
          />
        );
      default:
        return (
          <DiscoverPanel
            scene={scene.state.status === 'ready' ? scene.state.data : null}
            feed={feed.state}
            now={now}
            onPlace={place =>
              setOverlay({ kind: 'modal', target: { kind: 'place', place } })
            }
            onEvent={openEvent}
            onExploreMore={() => withPortrait({ kind: 'discover' })}
            onRetry={feed.reload}
            onKebab={action => {
              if (action === 'share' && scene.state.status === 'ready') {
                shareScene(scene.state.data);
              } else if (action === 'reportContent') {
                Linking.openURL(REPORT_CONTENT_URL).catch(() => undefined);
              } else if (action === 'reportBug') {
                withPortrait({ kind: 'bug' });
              }
            }}
          />
        );
    }
  };

  const renderOverlay = () => {
    if (overlay?.kind === 'discover' && events) {
      return (
        <DiscoverIngame
          feed={events}
          friends={social.status === 'ready' ? social.friends : []}
          played={played}
          now={now}
          onBack={leavePortrait}
          onPlace={place => {
            setOrientation('landscape');
            jumpTo(place);
          }}
          onEvent={event => {
            setOrientation('landscape');
            jumpTo(event.place);
          }}
        />
      );
    }
    if (overlay?.kind === 'bug') {
      return (
        <BugReportModal
          capture={api.captureScreenshot}
          onSubmit={form =>
            api.submitBugReport({
              category: form.category ?? '',
              description: form.description,
              screenshots: form.screenshots,
            })
          }
          onClose={leavePortrait}
        />
      );
    }
    if (overlay?.kind === 'modal') {
      const target = overlay.target;
      const id = target.kind === 'event' ? target.event.id : '';
      const set = reminders.has(id);
      return (
        <PlaceModal
          target={target}
          now={now}
          guest={api.isGuest()}
          reminderSet={set}
          onClose={() => setOverlay(null)}
          onJumpIn={jumpTo}
          onToggleReminder={() => {
            setReminders(prev => {
              const next = new Set(prev);
              if (!next.delete(id)) {
                next.add(id);
              }
              return next;
            });
            api.setReminder(id, !set).catch(() => undefined);
          }}
        />
      );
    }
    return null;
  };

  return (
    <MenuDrawerView
      section={section}
      unreadNotifications={unreadCount(items) > 0}
      onSelect={select}
      onClose={closeMenu}
      overlay={renderOverlay()}
    >
      {panel()}
    </MenuDrawerView>
  );
}

/** The world's `worldDrawer` slot: nothing while closed, the drawer while `openMenu()` is active. */
export function MenuDrawer({ navigate }: { navigate: Navigate }) {
  const { open } = useDrawerState();
  return open ? (
    <OpenDrawer navigate={navigate} />
  ) : (
    <View style={styles.none} pointerEvents="none" />
  );
}

const styles = StyleSheet.create({ none: { height: 0 } });
